//! Browser handoff for wallpaper imports.
//!
//! Some sites disallow automated downloads entirely (MoeWalls' robots.txt
//! forbids `/download*`), so the only compliant acquisition is a human
//! saving the file. The workflow: the panel opens the page in the default
//! browser, the user saves the image to `~/Downloads`, and this module notices
//! the arrival and imports it into the wallpaper catalog.
//!
//! It is a poll loop, not an inotify watcher: the daemon already has a
//! fast pulse, and polling every couple of seconds is simpler and
//! portable. Videos are rejected at import time — awww cannot render them.

use crate::wallpaper::is_renderable;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The XDG download directory: `$XDG_DOWNLOAD_DIR` or `~/Downloads`.
pub fn downloads_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DOWNLOAD_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    home_dir().join("Downloads")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// One file that arrived since the previous scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCandidate {
    pub path: PathBuf,
    pub modified: SystemTime,
}

/// The files that appeared in `dir` since the `seen` snapshot, newest first
/// and filtered to renderable images. Temporary browser download suffixes
/// are ignored.
pub fn newcomers(dir: &Path, seen: &HashSet<PathBuf>) -> Vec<ImportCandidate> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if seen.contains(&path) {
            continue;
        }
        if is_browser_temp(&path) {
            continue;
        }
        if !is_renderable(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        found.push(ImportCandidate { path, modified });
    }
    found.sort_by_key(|c| c.modified);
    found
}

/// Snapshot of what exists now. Persisted in memory by the server.
pub fn snapshot(dir: &Path) -> HashSet<PathBuf> {
    let mut set = HashSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return set;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            set.insert(path);
        }
    }
    set
}

/// Open a page in the default browser (handoff).
pub fn open_page(url: &str) -> Result<(), String> {
    let opener = if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        "open"
    };
    let status = std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map_err(|e| format!("could not open the page: {e}"))?;
    let _ = status;
    Ok(())
}

/// Copy a downloaded image into the wallpaper catalog directory.
pub fn import(src: &Path, catalog_dir: &Path) -> Result<PathBuf, String> {
    if !is_renderable(src) {
        return Err("the file is not a renderable image (video wallpapers are unsupported)".into());
    }
    if src.extension().is_none() {
        return Err("the file has no image extension".into());
    }
    std::fs::create_dir_all(catalog_dir)
        .map_err(|e| format!("could not create the wallpaper directory: {e}"))?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "wallpaper.img".into());
    let mut dest = catalog_dir.join(&name);
    if dest.exists() {
        dest = catalog_dir.join(format!("imported-{}.{}", timestamp(), name));
    }
    std::fs::copy(src, &dest).map_err(|e| format!("could not import: {e}"))?;
    Ok(dest)
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "0".into())
}

fn is_browser_temp(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.ends_with(".crdownload")
        || name.ends_with(".part")
        || name.ends_with(".download")
        || name.ends_with(".tmp")
        || name.starts_with(".com.google.Chrome.")
}

/// How long after a file appears before it is considered settled. Browsers
/// write in chunks; importing mid-write would copy a partial image.
pub const SETTLE: Duration = Duration::from_millis(800);

/// Whether a candidate is old enough to import.
pub fn is_settled(candidate: &ImportCandidate) -> bool {
    std::time::SystemTime::now()
        .duration_since(candidate.modified)
        .map(|d| d >= SETTLE)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isled-handoff-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_renderable_files_are_importable() {
        assert!(is_renderable(Path::new("a.jpg")));
        assert!(!is_renderable(Path::new("a.mp4")));
    }

    #[test]
    fn browser_temp_files_are_ignored() {
        assert!(is_browser_temp(Path::new("photo.jpg.crdownload")));
        assert!(is_browser_temp(Path::new("download.part")));
        assert!(!is_browser_temp(Path::new("photo.jpg")));
    }

    #[test]
    fn newcomers_are_renderable_and_unseen() {
        let dir = test_dir("new");
        fs::write(dir.join("one.jpg"), b"x").unwrap();
        fs::write(dir.join("clip.mp4"), b"x").unwrap();
        let seen = HashSet::new();
        let found = newcomers(&dir, &seen);
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with("one.jpg"));
        let seen = snapshot(&dir);
        assert!(newcomers(&dir, &seen).is_empty());
    }

    #[test]
    fn an_imported_file_lands_in_the_catalog() {
        let dir = test_dir("import");
        let src = dir.join("from-downloads.jpg");
        fs::write(&src, b"bytes").unwrap();
        let catalog = dir.join("catalog");
        let dest = import(&src, &catalog).unwrap();
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"bytes");
    }

    #[test]
    fn a_video_is_refused() {
        let dir = test_dir("video");
        let src = dir.join("clip.mp4");
        fs::write(&src, b"bytes").unwrap();
        assert!(import(&src, &dir.join("catalog")).is_err());
    }
}
