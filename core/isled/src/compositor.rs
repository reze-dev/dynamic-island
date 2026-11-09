//! Compositor adapters.
//!
//! One trait, three implementations, because the schemas are unrelated:
//!
//! * **Niri** — `$NIRI_SOCKET`, newline-delimited JSON. The `EventStream`
//!   request streams full state up-front and then deltas; no polling.
//! * **Hyprland** — `hyprctl -j` for queries, `event/socket2` for push.
//!   No good Rust crate exists — shell out.
//! * **Sway** — `$SWAYSOCK` (or `$I3SOCK`) IPC. The wire format is the
//!   i3/Sway protocol: u32 type + u32 length + `"i3-ipc"` + JSON payload.
//!
//! Every adapter returns the normalized types from `isle_core`, never its
//! own. Unknown compositor shapes are an `Other` event, never a crash.

use isle_core::compositor::CompositorInfo;
use isle_core::{Compositor, CompositorEvent, FocusedWindow, Output, Workspace};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// Bind a listener on the adapter socket. Tests call this with a path in a
/// temp dir.
#[allow(dead_code)]
pub async fn bind_adapter_socket(path: std::path::PathBuf) -> std::io::Result<UnixListener> {
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::remove_file(&path).await;
    UnixListener::bind(&path)
}

/// The normalized adapter interface. Implementations are small; the server
/// owns retry/reconnect semantics.
pub trait CompositorAdapter: Send + Sync {
    /// Query the current compositor state (focused window, workspaces,
    /// outputs). Best-effort: adapters with no data return an empty info.
    fn query(&self) -> CompositorInfo;

    /// Parse one line from the compositor's event stream into a normalized
    /// event. `None` means "not an event we model".
    fn parse_line(&self, line: &str) -> Option<CompositorEvent>;
}

/// The adapter for the compositor we detected at startup.
pub enum Adapter {
    Niri(NiriAdapter),
    Hyprland(HyprlandAdapter),
    Sway(SwayAdapter),
    Unknown,
}

impl Adapter {
    pub fn from_detected(compositor: Compositor) -> Self {
        match compositor {
            Compositor::Niri => Self::Niri(NiriAdapter::new()),
            Compositor::Hyprland => Self::Hyprland(HyprlandAdapter::new()),
            Compositor::Sway => Self::Sway(SwayAdapter::new()),
            Compositor::Unknown => Self::Unknown,
        }
    }

    pub fn query(&self) -> CompositorInfo {
        match self {
            Self::Niri(a) => a.query(),
            Self::Hyprland(a) => a.query(),
            Self::Sway(a) => a.query(),
            Self::Unknown => CompositorInfo::default(),
        }
    }

    pub fn parse_line(&self, line: &str) -> Option<CompositorEvent> {
        match self {
            Self::Niri(a) => a.parse_line(line),
            Self::Hyprland(a) => a.parse_line(line),
            Self::Sway(a) => a.parse_line(line),
            Self::Unknown => None,
        }
    }
}

// ---------------------------------------------------------------- niri

/// Niri queries are best-effort shell-outs to `niri msg -j`. The socket
/// path is discoverable via `$NIRI_SOCKET`; the event stream is
/// newline-delimited JSON, requested with `{"EventStream":null}` (niri
/// replies `{"Ok":"Handled"}` and then streams full state + deltas).
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct NiriAdapter {
    socket: std::path::PathBuf,
}

impl NiriAdapter {
    pub fn new() -> Self {
        Self {
            socket: std::env::var_os("NIRI_SOCKET")
                .map(std::path::PathBuf::from)
                .unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    pub fn with_socket(socket: std::path::PathBuf) -> Self {
        Self { socket }
    }

    /// Connect and send the EventStream request. The stream carries full
    /// state first, then deltas.
    #[allow(dead_code)]
    async fn announce_stream(
        &self,
    ) -> std::io::Result<(
        BufReader<tokio::net::unix::OwnedReadHalf>,
        tokio::net::unix::OwnedWriteHalf,
    )> {
        let stream = UnixStream::connect(&self.socket).await?;
        let (read, mut write) = stream.into_split();
        write.write_all(b"{\"EventStream\":null}\n").await?;
        write.flush().await?;
        Ok((BufReader::new(read), write))
    }

    /// One `niri msg -j <cmd>` invocation, parsed as JSON.
    fn niri_msg(&self, cmd: &str) -> Option<serde_json::Value> {
        let output = std::process::Command::new("niri")
            .arg("msg")
            .arg("-j")
            .arg(cmd)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }

    fn query_focused(&self) -> Option<FocusedWindow> {
        let value = self.niri_msg("focused-window")?;
        if value.is_null() {
            return None;
        }
        parse_niri_window(&value)
    }

    fn query_workspaces(&self) -> Vec<Workspace> {
        let Some(value) = self.niri_msg("workspaces") else {
            return Vec::new();
        };
        let Some(array) = value.as_array() else {
            return Vec::new();
        };
        array.iter().map(parse_niri_workspace).collect()
    }

    fn query_outputs(&self) -> Vec<Output> {
        let Some(value) = self.niri_msg("outputs") else {
            return Vec::new();
        };
        let Some(object) = value.as_object() else {
            return Vec::new();
        };
        object.values().map(parse_niri_output).collect()
    }
}

/// Parse one `niri msg -j` / EventStream window object:
/// `{"id":4,"title":"...","app_id":"helium","workspace_id":1,...}`.
/// `workspace_id` is a number; `fullscreen` is always false (niri's Window
/// carries no fullscreen flag).
pub fn parse_niri_window(value: &serde_json::Value) -> Option<FocusedWindow> {
    let object = value.as_object()?;
    let app_id = object
        .get("app_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let title = object
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let workspace = match object.get("workspace_id") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    Some(FocusedWindow {
        app_id,
        title,
        workspace,
        fullscreen: false,
    })
}

/// Parse one `niri msg -j workspaces` entry:
/// `{"id":1,"idx":1,"name":null,"output":"eDP-1","is_focused":true,...}`.
pub fn parse_niri_workspace(value: &serde_json::Value) -> Workspace {
    let object = value.as_object();
    let idx = object.and_then(|o| o.get("idx")).and_then(|v| v.as_u64());
    let name = object
        .and_then(|o| o.get("name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| idx.map(|i| i.to_string()).unwrap_or_default());
    Workspace {
        name,
        focused: object
            .and_then(|o| o.get("is_focused"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        output: object
            .and_then(|o| o.get("output"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

/// Parse one `niri msg -j outputs` entry. `refresh_hz` keeps niri's raw
/// `refresh_rate` (mHz) to match the Sway adapter's convention.
pub fn parse_niri_output(value: &serde_json::Value) -> Output {
    let object = value.as_object();
    let name = object
        .and_then(|o| o.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let logical = object
        .and_then(|o| o.get("logical"))
        .and_then(|v| v.as_object());
    let current = object
        .and_then(|o| o.get("current_mode"))
        .and_then(|v| v.as_u64());
    let refresh_hz = object
        .and_then(|o| o.get("modes"))
        .and_then(|v| v.as_array())
        .and_then(|modes| {
            let index = current.unwrap_or(0) as usize;
            modes.get(index).or_else(|| modes.first())
        })
        .and_then(|m| m.get("refresh_rate"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    Output {
        name,
        scale: logical
            .and_then(|l| l.get("scale"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0),
        width: logical
            .and_then(|l| l.get("width"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        height: logical
            .and_then(|l| l.get("height"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        refresh_hz,
        active: true,
    }
}

/// The focused window inside a `WindowsChanged` payload, if any window is
/// focused.
fn focused_in_windows(payload: &serde_json::Value) -> Option<FocusedWindow> {
    let windows = payload.get("windows")?.as_array()?;
    windows
        .iter()
        .find(|w| {
            w.get("is_focused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .and_then(parse_niri_window)
}

/// Parse one line of the niri EventStream. State-carrying variants
/// (`WindowsChanged`, `WorkspacesChanged`, `OutputsChanged`,
/// `WindowOpenedOrChanged`) are translated directly. Id-only deltas
/// (`WindowFocusChanged`, `WorkspaceFocusChanged`, `WindowClosed`,
/// `WorkspaceClosed`, `WorkspaceOpenedOrChanged`) carry no window or
/// workspace fields, so this returns `Other` for them and the adapter's
/// `parse_line` re-queries instead.
pub fn parse_niri_line(line: &str) -> Option<CompositorEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;
    if let Some(windows) = obj.get("WindowsChanged") {
        return Some(CompositorEvent::FocusChanged {
            window: focused_in_windows(windows),
        });
    }
    if let Some(workspaces) = obj.get("WorkspacesChanged") {
        let list = workspaces
            .get("workspaces")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(parse_niri_workspace).collect())
            .unwrap_or_default();
        return Some(CompositorEvent::WorkspaceChanged { workspaces: list });
    }
    if let Some(outputs) = obj.get("OutputsChanged") {
        let list = outputs
            .get("outputs")
            .and_then(|v| v.as_object())
            .map(|o| o.values().map(parse_niri_output).collect())
            .unwrap_or_default();
        return Some(CompositorEvent::OutputsChanged { outputs: list });
    }
    if let Some(opened) = obj.get("WindowOpenedOrChanged") {
        if let Some(window) = opened.get("window").and_then(parse_niri_window) {
            return Some(CompositorEvent::WindowSongChanged {
                window: Some(window),
            });
        }
    }
    if obj.contains_key("WindowFocusChanged")
        || obj.contains_key("WorkspaceFocusChanged")
        || obj.contains_key("WindowClosed")
        || obj.contains_key("WorkspaceClosed")
        || obj.contains_key("WorkspaceOpenedOrChanged")
    {
        return Some(CompositorEvent::Other {
            raw: trimmed.to_string(),
        });
    }
    None
}

impl CompositorAdapter for NiriAdapter {
    fn query(&self) -> CompositorInfo {
        CompositorInfo {
            focused_window: self.query_focused(),
            workspaces: self.query_workspaces(),
            outputs: self.query_outputs(),
        }
    }

    fn parse_line(&self, line: &str) -> Option<CompositorEvent> {
        match parse_niri_line(line) {
            Some(CompositorEvent::Other { raw })
                if raw.contains("WindowFocusChanged")
                    || raw.contains("WorkspaceFocusChanged")
                    || raw.contains("WindowClosed")
                    || raw.contains("WorkspaceClosed")
                    || raw.contains("WorkspaceOpenedOrChanged") =>
            {
                // Id-only deltas: refresh the affected slice from niri.
                if raw.contains("Workspace") {
                    Some(CompositorEvent::WorkspaceChanged {
                        workspaces: self.query_workspaces(),
                    })
                } else {
                    Some(CompositorEvent::FocusChanged {
                        window: self.query_focused(),
                    })
                }
            }
            other => other,
        }
    }
}

// -------------------------------------------------------------- hyprland

/// Hyprland queries are shell-outs to `hyprctl -j`. The event socket is
/// `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/event/socket2`.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct HyprlandAdapter {
    instance: String,
}

impl HyprlandAdapter {
    pub fn new() -> Self {
        Self {
            instance: std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    pub fn with_instance(instance: String) -> Self {
        Self { instance }
    }

    #[allow(dead_code)]
    pub fn event_socket(&self) -> std::path::PathBuf {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        runtime
            .join("hypr")
            .join(&self.instance)
            .join("event")
            .join("socket2")
    }

    fn hyprctl(&self, arg: &str) -> Option<serde_json::Value> {
        let output = std::process::Command::new("hyprctl")
            .args(["-j", arg])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }
}

impl CompositorAdapter for HyprlandAdapter {
    fn query(&self) -> CompositorInfo {
        let mut info = CompositorInfo::default();
        if let Some(clients) = self.hyprctl("clients") {
            if let Some(array) = clients.as_array() {
                let focused = array
                    .iter()
                    .find(|c| c.get("focusHistoryID").and_then(|v| v.as_i64()) == Some(0));
                if let Some(client) = focused {
                    info.focused_window = Some(parse_hypr_window(client));
                }
            }
        }
        if let Some(workspaces) = self.hyprctl("workspaces") {
            if let Some(array) = workspaces.as_array() {
                info.workspaces = array.iter().map(parse_hypr_workspace).collect();
            }
        }
        if let Some(monitors) = self.hyprctl("monitors") {
            if let Some(array) = monitors.as_array() {
                info.outputs = array.iter().map(parse_hypr_output).collect();
            }
        }
        info
    }

    fn parse_line(&self, line: &str) -> Option<CompositorEvent> {
        parse_hypr_line(line)
    }
}

pub fn parse_hypr_line(line: &str) -> Option<CompositorEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Hyprland events are `<action>><data>` lines, not JSON.
    let (action, data) = match trimmed.split_once(">>") {
        Some(pair) => pair,
        None => (trimmed, ""),
    };
    match action {
        "focuswindow" | "activewindow" => {
            // data is `<class>,<title>` (class may be empty for a desk).
            let mut parts = data.splitn(2, ',');
            let app_id = parts.next().unwrap_or("").into();
            let title = parts.next().unwrap_or("").into();
            Some(CompositorEvent::FocusChanged {
                window: Some(FocusedWindow {
                    app_id,
                    title,
                    workspace: String::new(),
                    fullscreen: false,
                }),
            })
        }
        "focus" => Some(CompositorEvent::WorkspaceChanged {
            workspaces: vec![Workspace {
                name: data.into(),
                focused: true,
                output: None,
            }],
        }),
        "fullscreen" => Some(CompositorEvent::Other {
            raw: trimmed.to_string(),
        }),
        _ => Some(CompositorEvent::Other {
            raw: trimmed.to_string(),
        }),
    }
}

fn parse_hypr_window(value: &serde_json::Value) -> FocusedWindow {
    let app_id = value
        .get("class")
        .or_else(|| value.get("class_"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let workspace = value
        .get("workspace")
        .and_then(|ws| ws.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let fullscreen = value
        .get("fullscreen")
        .and_then(|v| v.as_i64())
        .map(|v| v != 0)
        .unwrap_or(false);
    FocusedWindow {
        app_id,
        title,
        workspace,
        fullscreen,
    }
}

fn parse_hypr_workspace(value: &serde_json::Value) -> Workspace {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let focused = value
        .get("active")
        .and_then(|v| v.as_i64())
        .map(|v| v != 0)
        .unwrap_or(false);
    Workspace {
        name,
        focused,
        output: value
            .get("monitor")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

fn parse_hypr_output(value: &serde_json::Value) -> Output {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let scale = value.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let width = value.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let height = value.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let refresh_hz = value
        .get("refreshRate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    let active = value
        .get("active")
        .and_then(|v| v.as_i64())
        .map(|v| v != 0)
        .unwrap_or(false);
    Output {
        name,
        scale,
        width,
        height,
        refresh_hz,
        active,
    }
}

// ------------------------------------------------------------------ sway

/// Sway queries are `swaymsg` shell-outs; the socket is `$SWAYSOCK`.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SwayAdapter {
    socket: std::path::PathBuf,
}

impl SwayAdapter {
    pub fn new() -> Self {
        let socket = std::env::var_os("SWAYSOCK")
            .or_else(|| std::env::var_os("I3SOCK"))
            .map(std::path::PathBuf::from)
            .unwrap_or_default();
        Self { socket }
    }

    #[allow(dead_code)]
    pub fn with_socket(socket: std::path::PathBuf) -> Self {
        Self { socket }
    }

    fn swaymsg(&self, payload: &str) -> Option<serde_json::Value> {
        let output = std::process::Command::new("swaymsg")
            .arg("-t")
            .arg(payload)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }
}

impl CompositorAdapter for SwayAdapter {
    fn query(&self) -> CompositorInfo {
        let mut info = CompositorInfo::default();
        if let Some(outputs) = self.swaymsg("get_outputs") {
            if let Some(array) = outputs.as_array() {
                info.outputs = array.iter().map(parse_sway_output).collect();
            }
        }
        if let Some(workspaces) = self.swaymsg("get_workspaces") {
            if let Some(array) = workspaces.as_array() {
                info.workspaces = array.iter().map(parse_sway_workspace).collect();
            }
        }
        if let Some(tree) = self.swaymsg("get_tree") {
            info.focused_window = parse_sway_focused(&tree);
        }
        info
    }

    fn parse_line(&self, line: &str) -> Option<CompositorEvent> {
        parse_sway_line(line)
    }
}

pub fn parse_sway_line(line: &str) -> Option<CompositorEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Sway IPC command replies are JSON; subscribe events carry a `change`
    // string plus `container`. Parse defensively.
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    if let Some(change) = value.get("change").and_then(|v| v.as_str()) {
        match change {
            "focus" => {
                let window = value
                    .get("container")
                    .map(parse_sway_focused_from_node)
                    .unwrap_or(None);
                return Some(CompositorEvent::FocusChanged { window });
            }
            "output" => {
                return Some(CompositorEvent::Other {
                    raw: trimmed.to_string(),
                });
            }
            _ => {}
        }
    }
    None
}

fn parse_sway_output(value: &serde_json::Value) -> Output {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let scale = value.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let width = value
        .get("current_mode")
        .and_then(|m| m.get("width"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let height = value
        .get("current_mode")
        .and_then(|m| m.get("height"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let refresh_hz = value
        .get("current_mode")
        .and_then(|m| m.get("refresh"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let active = !value
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Output {
        name,
        scale,
        width,
        height,
        refresh_hz,
        active: !active,
    }
}

fn parse_sway_workspace(value: &serde_json::Value) -> Workspace {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let focused = value
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let output = value
        .get("output")
        .and_then(|v| v.as_str())
        .map(String::from);
    Workspace {
        name,
        focused,
        output,
    }
}

fn parse_sway_focused(tree: &serde_json::Value) -> Option<FocusedWindow> {
    // Walk the tree for the node with `focused == true` and `type == "con"`.
    fn walk(value: &serde_json::Value) -> Option<FocusedWindow> {
        if let Some(found) = parse_sway_focused_from_node(value) {
            return Some(found);
        }
        for child in value
            .get("nodes")
            .and_then(|n| n.as_array())
            .unwrap_or(&Vec::new())
        {
            if let Some(found) = walk(child) {
                return Some(found);
            }
        }
        for child in value
            .get("floating_nodes")
            .and_then(|n| n.as_array())
            .unwrap_or(&Vec::new())
        {
            if let Some(found) = walk(child) {
                return Some(found);
            }
        }
        None
    }
    walk(tree)
}

fn parse_sway_focused_from_node(value: &serde_json::Value) -> Option<FocusedWindow> {
    let focused = value
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !focused {
        return None;
    }
    if value.get("type").and_then(|v| v.as_str()) != Some("con") {
        return None;
    }
    let app_id = value
        .get("app_id")
        .or_else(|| value.get("window_properties"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let title = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let workspace = value
        .get("workspace_name")
        .or_else(|| value.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let fullscreen = value
        .get("fullscreen")
        .and_then(|v| v.as_i64())
        .map(|v| v != 0)
        .unwrap_or(false);
    Some(FocusedWindow {
        app_id,
        title,
        workspace,
        fullscreen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn niri_focus_changes_are_parsed() {
        // Verbatim shape from a live `niri msg event-stream` (niri 26.04).
        let line = r#"{"WindowsChanged":{"windows":[{"id":4,"title":"Configure Nexus TLS for OpenShift - Helium","app_id":"helium","pid":13659,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1560.0,900.0],"window_size":[1560,900],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":{"secs":8095,"nanos":849763916}}]}}"#;
        match parse_niri_line(line) {
            Some(CompositorEvent::FocusChanged { window: Some(w) }) => {
                assert_eq!(w.app_id, "helium");
                assert_eq!(w.title, "Configure Nexus TLS for OpenShift - Helium");
                assert_eq!(w.workspace, "1");
                assert!(!w.fullscreen);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn niri_workspaces_are_parsed() {
        let line = r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"idx":1,"name":null,"output":"eDP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":4}]}}"#;
        match parse_niri_line(line) {
            Some(CompositorEvent::WorkspaceChanged { workspaces }) => {
                assert_eq!(workspaces.len(), 1);
                assert_eq!(workspaces[0].name, "1");
                assert!(workspaces[0].focused);
                assert_eq!(workspaces[0].output.as_deref(), Some("eDP-1"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn niri_outputs_are_parsed() {
        let line = r#"{"OutputsChanged":{"outputs":{"eDP-1":{"name":"eDP-1","physical_size":[340,190],"modes":[{"width":1920,"height":1080,"refresh_rate":144000,"is_preferred":true}],"current_mode":0,"logical":{"x":0,"y":0,"width":1600,"height":900,"scale":1.2,"transform":"Normal"}}}}}"#;
        match parse_niri_line(line) {
            Some(CompositorEvent::OutputsChanged { outputs }) => {
                assert_eq!(outputs.len(), 1);
                assert_eq!(outputs[0].name, "eDP-1");
                assert_eq!(outputs[0].width, 1600);
                assert_eq!(outputs[0].height, 900);
                assert!((outputs[0].scale - 1.2).abs() < 0.01);
                assert_eq!(outputs[0].refresh_hz, 144000);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn niri_id_only_deltas_are_other_so_the_adapter_re_queries() {
        assert!(matches!(
            parse_niri_line(r#"{"WindowFocusChanged":{"id":4,"previous":11}}"#),
            Some(CompositorEvent::Other { .. })
        ));
        assert!(matches!(
            parse_niri_line(r#"{"WorkspaceFocusChanged":{"id":1,"previous":null}}"#),
            Some(CompositorEvent::Other { .. })
        ));
    }

    #[test]
    fn niri_garbage_is_ignored() {
        assert!(parse_niri_line("not json").is_none());
        assert!(parse_niri_line("   ").is_none());
        assert!(parse_niri_line(r#"{"Ok":"Handled"}"#).is_none());
    }

    #[test]
    fn hyprland_focus_lines_are_parsed() {
        match parse_hypr_line("activewindow>>Alacritty,htop") {
            Some(CompositorEvent::FocusChanged { window: Some(w) }) => {
                assert_eq!(w.app_id, "Alacritty");
                assert_eq!(w.title, "htop");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn hyprland_unknown_actions_are_other() {
        match parse_hypr_line("openwindow>>0,0x41") {
            Some(CompositorEvent::Other { raw }) => assert!(raw.contains("openwindow")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn sway_focus_events_are_parsed() {
        let line = r#"{"change":"focus","container":{"focused":true,"type":"con","app_id":"firefox","name":"GitHub","workspace_name":"2"}}"#;
        match parse_sway_line(line) {
            Some(CompositorEvent::FocusChanged { window: Some(w) }) => {
                assert_eq!(w.app_id, "firefox");
                assert_eq!(w.title, "GitHub");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn sway_outputs_and_workspaces_are_parsed() {
        let output = serde_json::json!({
            "name": "eDP-1", "scale": 1.25, "active": true,
            "current_mode": {"width": 1920, "height": 1080, "refresh": 14400}
        });
        let parsed = parse_sway_output(&output);
        assert_eq!(parsed.name, "eDP-1");
        assert_eq!(parsed.refresh_hz, 14400);

        let workspace = serde_json::json!({"name": "1", "focused": true, "output": "eDP-1"});
        let parsed = parse_sway_workspace(&workspace);
        assert!(parsed.focused);
    }

    #[test]
    fn hyprland_client_json_is_parsed() {
        let client = serde_json::json!({
            "class": "firefox", "title": "GitHub", "focusHistoryID": 0,
            "workspace": {"name": "2"}, "fullscreen": 1
        });
        let window = parse_hypr_window(&client);
        assert_eq!(window.app_id, "firefox");
        assert_eq!(window.workspace, "2");
        assert!(window.fullscreen);
    }
}
