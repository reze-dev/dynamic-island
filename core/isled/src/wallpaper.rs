//! The wallpaper subsystem.
//!
//! Three concerns, kept apart the way the plan describes them: the *source*
//! finds things, the *catalog* owns the truth, and the *engine* draws pixels.
//! We are the catalog. The engine is a separate process we drive.
//!
//! v1 sources are local directories. Remote sources (the Wallhaven API) and
//! browser handoff are later, and the trait boundary below is where they
//! will go.
//!
//! v1 engines are `awww` and `hyprpaper`. Both are driven by shelling out
//! and parsing their exit status, which keeps a heavyweight dependency out of
//! the daemon and makes swapping engines a matter of adding a match arm.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The image types a wallpaper engine can actually draw.
///
/// `awww` has no video support at all, so a `.mp4` in the catalog would be an
/// entry nothing can render. Rejecting them at import time keeps the catalog
/// honest.
pub fn is_renderable(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "jxl" | "webp" | "gif" | "apng" | "avif" | "bmp" | "tiff" | "tga"
    )
}

/// One wallpaper in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wallpaper {
    /// Absolute path. This is the key.
    pub path: String,
    /// The file name without its extension, for display.
    pub name: String,
    pub bytes: u64,
    pub modified_unix: u64,
}

/// Which engine draws the pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    /// The `awww` daemon, in the `isle` namespace.
    Awww,
    /// `hyprpaper`, via `hyprctl`.
    Hyprpaper,
    /// Nothing is installed. The panel says so rather than pretending.
    #[default]
    None,
}

impl Engine {
    /// Pick an engine. `awww` wins when it is installed because it is the
    /// one the plan settled on; `hyprpaper` is the Hyprland fallback.
    pub fn detect() -> Self {
        if which("awww").is_some() {
            return Self::Awww;
        }
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() && which("hyprpaper").is_some()
        {
            return Self::Hyprpaper;
        }
        Self::None
    }

    /// Is the engine actually answering? `awww` needs its daemon running;
    /// `hyprpaper` needs hyprctl.
    pub fn probe(self) -> Result<(), String> {
        match self {
            Self::Awww => {
                let output = Command::new("awww")
                    .args(["query", "--json", "-n", "isle"])
                    .output()
                    .map_err(|e| format!("could not run awww: {e}"))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err("the awww daemon is not answering".to_string())
                }
            }
            Self::Hyprpaper => {
                let output = Command::new("hyprctl")
                    .arg("version")
                    .output()
                    .map_err(|e| format!("could not run hyprctl: {e}"))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err("hyprctl is not answering".to_string())
                }
            }
            Self::None => Err("no wallpaper engine is installed".to_string()),
        }
    }

    /// Draw a wallpaper.
    pub fn apply(self, path: &Path) -> Result<(), String> {
        match self {
            Self::Awww => {
                let output = Command::new("awww")
                    .args([
                        "img",
                        "-n",
                        "isle",
                        "--transition-type",
                        "fade",
                        "--transition-step",
                        "60",
                        "--transition-fps",
                        "60",
                    ])
                    .arg(path)
                    .output()
                    .map_err(|e| format!("could not run awww: {e}"))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                }
            }
            Self::Hyprpaper => {
                let output = Command::new("hyprctl")
                    .arg("hyprpaper")
                    .arg("preload")
                    .arg(path)
                    .output()
                    .map_err(|e| format!("could not run hyprctl: {e}"))?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                }
                let output = Command::new("hyprctl")
                    .arg("hyprpaper")
                    .arg("wallpaper")
                    .arg(format!("{},1", path.display()))
                    .output()
                    .map_err(|e| format!("could not run hyprctl: {e}"))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                }
            }
            Self::None => Err("no wallpaper engine is installed".to_string()),
        }
    }
}

/// The wallpaper catalog: what exists, what is showing, and how it changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    /// The directory the catalog was scanned from.
    pub dir: String,
    #[serde(default)]
    pub items: Vec<Wallpaper>,
    /// The path currently on screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    #[serde(default)]
    pub engine: Engine,
    /// Whether the wallpaper changes on a timer.
    #[serde(default)]
    pub loop_enabled: bool,
    /// How long between changes, in seconds.
    #[serde(default = "default_interval")]
    pub loop_interval_secs: u64,
    #[serde(default)]
    pub last_changed_unix: u64,
}

fn default_interval() -> u64 {
    60
}

impl Default for Catalog {
    fn default() -> Self {
        Self {
            dir: String::new(),
            items: Vec::new(),
            current: None,
            engine: Engine::None,
            loop_enabled: false,
            loop_interval_secs: default_interval(),
            last_changed_unix: 0,
        }
    }
}

impl Catalog {
    /// Scan a directory for renderable images.
    ///
    /// Files that an engine cannot draw are skipped rather than imported, so
    /// the catalog never accumulates entries the user cannot apply.
    pub fn scan(dir: &Path, engine: Engine) -> Self {
        let mut items = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() || !is_renderable(&path) {
                    continue;
                }
                let meta = entry.metadata().ok();
                items.push(Wallpaper {
                    name: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: path.to_string_lossy().into_owned(),
                    bytes: meta.as_ref().map_or(0, |m| m.len()),
                    modified_unix: meta
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs()),
                });
            }
        }
        items.sort_by_key(|a| a.name.to_lowercase());
        Self {
            dir: dir.to_string_lossy().into_owned(),
            items,
            engine,
            ..Self::default()
        }
    }

    pub fn find(&self, path: &str) -> Option<&Wallpaper> {
        self.items.iter().find(|w| w.path == path)
    }

    /// Pick a wallpaper at random. The current one is excluded so "random"
    /// always visibly changes something.
    pub fn random_excluding(&self, current: Option<&str>) -> Option<&Wallpaper> {
        let candidates: Vec<&Wallpaper> = self
            .items
            .iter()
            .filter(|w| Some(w.path.as_str()) != current)
            .collect();
        if candidates.is_empty() {
            return self.items.first();
        }
        let index = pseudo_random(candidates.len());
        candidates.get(index).copied()
    }
}

/// A small xorshift so the wallpaper picker does not need a dependency.
///
/// Seeded from the clock, so two calls in the same second still differ
/// because the seed advances.
fn pseudo_random(len: usize) -> usize {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }

    STATE.with(|state| {
        let mut s = state.get();
        if s == 0 {
            s = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0x9E3779B97F4A7C15, |d| d.as_nanos() as u64)
                | 1;
        }
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        state.set(s);
        (s % len as u64) as usize
    })
}

/// Find a binary on `PATH`.
pub fn which(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isled-wall-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_renderable_formats_are_accepted() {
        assert!(is_renderable(Path::new("/a/b.png")));
        assert!(is_renderable(Path::new("/a/b.JPEG")));
        assert!(is_renderable(Path::new("/a/b.webp")));
        assert!(!is_renderable(Path::new("/a/b.mp4")));
        assert!(!is_renderable(Path::new("/a/b.txt")));
        assert!(!is_renderable(Path::new("/a/b")));
    }

    #[test]
    fn scanning_finds_images_and_skips_everything_else() {
        let dir = temp_dir("scan");
        fs::write(dir.join("one.png"), b"x").unwrap();
        fs::write(dir.join("two.jpg"), b"x").unwrap();
        fs::write(dir.join("clip.mp4"), b"x").unwrap();
        fs::write(dir.join("notes.txt"), b"x").unwrap();
        let catalog = Catalog::scan(&dir, Engine::Awww);
        assert_eq!(catalog.items.len(), 2);
        assert!(
            catalog
                .items
                .iter()
                .all(|w| w.path.ends_with(".png") || w.path.ends_with(".jpg"))
        );
    }

    #[test]
    fn scanning_a_missing_directory_is_not_an_error() {
        let catalog = Catalog::scan(Path::new("/definitely/not/here"), Engine::None);
        assert!(catalog.items.is_empty());
    }

    #[test]
    fn random_never_picks_the_current_wallpaper_when_there_is_a_choice() {
        let dir = temp_dir("random");
        fs::write(dir.join("a.png"), b"x").unwrap();
        fs::write(dir.join("b.png"), b"x").unwrap();
        fs::write(dir.join("c.png"), b"x").unwrap();
        let catalog = Catalog::scan(&dir, Engine::Awww);
        let current = catalog.items[0].path.clone();
        for _ in 0..20 {
            let pick = catalog.random_excluding(Some(&current)).unwrap();
            assert_ne!(pick.path, current);
        }
    }

    #[test]
    fn random_falls_back_to_the_only_wallpaper() {
        let dir = temp_dir("single");
        fs::write(dir.join("only.png"), b"x").unwrap();
        let catalog = Catalog::scan(&dir, Engine::Awww);
        let current = catalog.items[0].path.clone();
        assert_eq!(
            catalog
                .random_excluding(Some(&current))
                .map(|w| w.name.as_str()),
            Some("only")
        );
    }

    #[test]
    fn an_empty_catalog_has_nothing_to_pick() {
        let catalog = Catalog::default();
        assert!(catalog.random_excluding(None).is_none());
    }

    #[test]
    fn the_default_loop_interval_is_a_minute() {
        assert_eq!(Catalog::default().loop_interval_secs, 60);
    }

    #[test]
    fn the_engine_serialises_as_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Engine::Hyprpaper).unwrap(),
            "\"hyprpaper\""
        );
    }
}
