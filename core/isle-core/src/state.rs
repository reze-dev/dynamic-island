use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Compositor {
    Niri,
    Hyprland,
    #[default]
    Unknown,
}

impl Compositor {
    pub fn detect() -> Self {
        if std::env::var_os("NIRI_SOCKET").is_some_and(|v| !v.is_empty()) {
            Self::Niri
        } else if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some_and(|v| !v.is_empty()) {
            Self::Hyprland
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub compositor: Compositor,
    pub wayland_display: Option<String>,
    pub started_at_unix: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Features {
    pub network_manager: bool,
    pub bluetooth: bool,
    pub mpris: bool,
    pub screencast: bool,
    pub wallpaper_engine: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub session: Session,
    pub features: Features,
}

impl Snapshot {
    pub fn new(session: Session, features: Features) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            session,
            features,
        }
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_carries_the_protocol_version() {
        let snapshot = Snapshot::new(
            Session {
                compositor: Compositor::Niri,
                wayland_display: Some("wayland-1".into()),
                started_at_unix: 1,
            },
            Features::default(),
        );
        assert_eq!(snapshot.version, PROTOCOL_VERSION);
    }

    #[test]
    fn snapshot_default_is_still_a_well_formed_message() {
        assert!(serde_json::to_string(&Snapshot::default()).is_ok());
    }

    #[test]
    fn compositor_serialises_in_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Compositor::Hyprland).unwrap(),
            r#""hyprland""#
        );
    }

    #[test]
    fn detecting_a_compositor_outside_a_session_yields_unknown() {
        // The test harness runs with a scrubbed environment, so this asserts the
        // no-session branch rather than a specific compositor.
        if std::env::var_os("NIRI_SOCKET").is_none()
            && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
        {
            assert_eq!(Compositor::detect(), Compositor::Unknown);
        }
    }
}
