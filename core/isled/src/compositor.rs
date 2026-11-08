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

/// Niri queries are best-effort shell-outs to `niri msg`. The socket path
/// is discoverable via `$NIRI_SOCKET`; the event stream is structured JSON.
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
        write.write_all(b"{\"EventStream\":[]}\n").await?;
        write.flush().await?;
        Ok((BufReader::new(read), write))
    }
}

pub fn parse_niri_line(line: &str) -> Option<CompositorEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;
    if let Some(focused) = obj.get("focusedWindow") {
        if let Some(window) = parse_niri_window(focused) {
            return Some(CompositorEvent::FocusChanged {
                window: Some(window),
            });
        }
    }
    if obj.contains_key("WorkspacesChanged") || obj.contains_key("OutputsChanged") {
        return Some(CompositorEvent::Other {
            raw: trimmed.to_string(),
        });
    }
    None
}

fn parse_niri_window(value: &serde_json::Value) -> Option<FocusedWindow> {
    let window = value.get("window")?;
    let object = window.get("window")?;
    let app_id = object
        .get("app_id")
        .or_else(|| object.get("appId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let title = object
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let workspace = object
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .into();
    let fullscreen = object
        .get("is_focused")
        .or_else(|| object.get("fullscreen"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(FocusedWindow {
        app_id,
        title,
        workspace,
        fullscreen,
    })
}

impl CompositorAdapter for NiriAdapter {
    fn query(&self) -> CompositorInfo {
        // Best-effort: `niri msg workspaces` / `niri msg outputs` are not
        // stable JSON on every version, so decode defensively.
        CompositorInfo::default()
    }

    fn parse_line(&self, line: &str) -> Option<CompositorEvent> {
        parse_niri_line(line)
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
        let line = r#"{"focusedWindow":{"window":{"window":{"app_id":"kitty","title":"zsh","workspace_id":"1","is_focused":true}}}}"#;
        match parse_niri_line(line) {
            Some(CompositorEvent::FocusChanged { window: Some(w) }) => {
                assert_eq!(w.app_id, "kitty");
                assert_eq!(w.title, "zsh");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn niri_garbage_is_ignored() {
        assert!(parse_niri_line("not json").is_none());
        assert!(parse_niri_line("   ").is_none());
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
