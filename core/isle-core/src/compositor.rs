//! Normalized compositor types.
//!
//! Each compositor speaks a different protocol (niri JSON events, hyprland
//! `hyprctl -j` + event socket, Sway IPC), so the daemon translates whatever
//! it sees into these shared types. The UI and the wallpaper engine only
//! ever see these.

use serde::{Deserialize, Serialize};

/// One output (monitor) as reported by the compositor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    pub scale: f64,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub refresh_hz: u32,
    #[serde(default)]
    pub active: bool,
}

/// One workspace as reported by the compositor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    #[serde(default)]
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// The window that has focus, reduced to what the shell displays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedWindow {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub fullscreen: bool,
}

/// Everything the shell wants to know about the screen, in one place.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompositorInfo {
    #[serde(default)]
    pub focused_window: Option<FocusedWindow>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub outputs: Vec<Output>,
}

/// A normalized event from a compositor's stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CompositorEvent {
    /// The focused window changed (title, app_id, workspace, fullscreen).
    FocusChanged { window: Option<FocusedWindow> },
    /// The workspace list changed (names, focus, outputs).
    WorkspaceChanged { workspaces: Vec<Workspace> },
    /// An output was added or removed.
    OutputsChanged { outputs: Vec<Output> },
    /// A window opened or closed.
    WindowSongChanged { window: Option<FocusedWindow> },
    /// Anything else the compositor emitted that we do not model yet.
    Other { raw: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_info_serialises() {
        let info = CompositorInfo::default();
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("focused_window"));
        assert!(json.contains("workspaces"));
    }

    #[test]
    fn events_round_trip_through_json() {
        let event = CompositorEvent::FocusChanged {
            window: Some(FocusedWindow {
                app_id: "kitty".into(),
                title: "zsh".into(),
                workspace: "1".into(),
                fullscreen: false,
            }),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: CompositorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn an_output_round_trips() {
        let output = Output {
            name: "eDP-1".into(),
            scale: 1.25,
            width: 1920,
            height: 1080,
            refresh_hz: 144,
            active: true,
        };
        let json = serde_json::to_string(&output).unwrap();
        let back: Output = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "eDP-1");
        assert_eq!(back.refresh_hz, 144);
        assert_eq!(back.scale, 1.25);
    }
}
