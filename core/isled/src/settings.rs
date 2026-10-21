//! Settings the user owns.
//!
//! Everything here survives a restart and is editable from the panels. The
//! defaults are chosen so that a fresh install does something sensible
//! without asking anything: wallpapers come from `~/Pictures/Wallpapers`,
//! notes live in `~/Documents/isle-notes`, and calendars come from the
//! standard vdir location.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Where wallpapers are picked up from.
    pub wallpaper_dir: String,
    /// Whether the wallpaper changes on a timer.
    #[serde(default)]
    pub wallpaper_loop: bool,
    /// How long between wallpaper changes, in seconds.
    #[serde(default = "default_loop_secs")]
    pub wallpaper_loop_secs: u64,
    /// Where quick notes are written.
    pub notes_dir: String,
    /// Where `.ics` calendars are read from.
    pub calendar_dir: String,
    /// The active theme name. The theme menu edits this.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Whether the island reacts to events on its own.
    #[serde(default = "default_true")]
    pub auto_context: bool,
}

fn default_loop_secs() -> u64 {
    60
}

fn default_theme() -> String {
    "midnight".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wallpaper_dir: default_wallpaper_dir().to_string_lossy().into_owned(),
            wallpaper_loop: false,
            wallpaper_loop_secs: default_loop_secs(),
            notes_dir: default_notes_dir().to_string_lossy().into_owned(),
            calendar_dir: default_calendar_dir().to_string_lossy().into_owned(),
            theme: default_theme(),
            auto_context: default_true(),
        }
    }
}

impl Settings {
    pub fn wallpaper_dir(&self) -> PathBuf {
        PathBuf::from(&self.wallpaper_dir)
    }

    pub fn notes_dir(&self) -> PathBuf {
        PathBuf::from(&self.notes_dir)
    }

    pub fn calendar_dir(&self) -> PathBuf {
        PathBuf::from(&self.calendar_dir)
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// `~/Pictures/Wallpapers` unless the XDG picture directory says otherwise.
pub fn default_wallpaper_dir() -> PathBuf {
    home().join("Pictures/Wallpapers")
}

/// `~/Documents/isle-notes`. Point this at an Obsidian vault to get notes
/// editable in Obsidian for free.
pub fn default_notes_dir() -> PathBuf {
    home().join("Documents/isle-notes")
}

/// The standard vdir location for calendars.
pub fn default_calendar_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("calendars");
    }
    home().join(".local/share/calendars")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_sensible_paths() {
        let settings = Settings::default();
        assert!(settings.wallpaper_dir.contains("Wallpapers"));
        assert!(settings.notes_dir.contains("isle-notes"));
        assert!(settings.calendar_dir.contains("calendars"));
    }

    #[test]
    fn the_loop_defaults_to_a_minute() {
        assert_eq!(Settings::default().wallpaper_loop_secs, 60);
    }

    #[test]
    fn settings_survive_a_round_trip_through_json() {
        let settings = Settings {
            wallpaper_dir: "/tmp/walls".into(),
            wallpaper_loop: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.wallpaper_dir, "/tmp/walls");
        assert!(back.wallpaper_loop);
    }
}
