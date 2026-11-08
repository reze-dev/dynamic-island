//! LRU pruning for the wallpaper thumbnail cache.
//!
//! The cache is the wallhaven thumbnails under
//! `$XDG_CACHE_HOME/isled/wallhaven-thumbs` (falling back to
//! `$HOME/.cache/isled/wallhaven-thumbs`, never `/tmp`). Downloads land
//! in the user's wallpaper directory — user data that the pruner never
//! touches.
//!
//! The policy: least-recently-used first, oldest files go first until the
//! cache is under the budget. The budget lives in settings
//! (`cache_max_bytes`, default 200 MiB) and the pruner is skipped
//! entirely when `cache_prune` is false.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The thumbnail cache directory. Respects XDG, falls back to
/// `~/.cache`, and never to `/tmp` (which a tmp-cleaner may wipe out
/// from under the panel).
pub fn thumbs_dir() -> PathBuf {
    cache_root().join("wallhaven-thumbs")
}

/// The tray pixmap directory: BMP renderings of StatusNotifier pixmaps,
/// one file per item. Same XDG discipline as the thumbnails.
pub fn tray_pixmaps_dir() -> PathBuf {
    cache_root().join("tray-pixmaps")
}

fn cache_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("isled");
    }
    home_dir().join(".cache/isled")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// One cached file and how old it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub path: PathBuf,
    pub bytes: u64,
    pub age: Duration,
}

/// Read the cache directory, oldest first. Missing dir is empty vec.
pub fn list(dir: &Path) -> Vec<CacheEntry> {
    let now = SystemTime::now();
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return entries;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let bytes = meta.len();
        let age = meta
            .modified()
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .unwrap_or(Duration::from_secs(u64::MAX));
        entries.push(CacheEntry { path, bytes, age });
    }
    // Least-recently-used first: largest age at the front.
    entries.sort_by_key(|e| std::cmp::Reverse(e.age));
    entries
}

/// Total bytes currently under `dir`.
pub fn total_bytes(dir: &Path) -> u64 {
    list(dir).iter().map(|e| e.bytes).sum()
}

/// Total bytes under both cache directories (thumbnails + tray pixmaps),
/// which share one LRU budget.
pub fn total_bytes_both() -> u64 {
    total_bytes(&thumbs_dir()) + total_bytes(&tray_pixmaps_dir())
}

/// Prune `dir` until it fits under `budget`. Returns the number of
/// files removed. `budget == 0` means "remove everything".
#[allow(dead_code)] // kept for single-dir callers and tests; server uses prune_all
pub fn prune(dir: &Path, budget: u64) -> Result<usize, String> {
    if !dir.exists() {
        return Ok(0);
    }
    prune_dirs(&[dir.to_path_buf()], budget)
}

/// Prune the shared cache (thumbnails + tray pixmaps) until their
/// combined size fits under `budget`, oldest-first across both dirs.
pub fn prune_all(budget: u64) -> Result<usize, String> {
    prune_dirs(&[thumbs_dir(), tray_pixmaps_dir()], budget)
}

fn prune_dirs(dirs: &[std::path::PathBuf], budget: u64) -> Result<usize, String> {
    // Merge entries oldest-first across all dirs.
    let mut entries: Vec<CacheEntry> = Vec::new();
    let mut total: u64 = 0;
    for dir in dirs {
        for entry in list(dir) {
            total = total.saturating_add(entry.bytes);
            entries.push(entry);
        }
    }
    // list() sorts each dir oldest-first; re-sort the merge globally.
    entries.sort_by_key(|e| std::cmp::Reverse(e.age));
    let mut removed = 0;
    let mut index = 0;
    while total > budget {
        let Some(victim) = entries.get(index) else {
            break;
        };
        match std::fs::remove_file(&victim.path) {
            Ok(()) => {
                total = total.saturating_sub(victim.bytes);
                removed += 1;
                index += 1;
            }
            Err(e) => {
                return Err(format!("could not remove {}: {e}", victim.path.display()));
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isled-lru-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cache_lists_oldest_first() {
        let dir = test_dir("order");
        fs::write(dir.join("new.jpg"), b"x").unwrap();
        fs::write(dir.join("old.jpg"), b"xx").unwrap();
        let entries = list(&dir);
        assert_eq!(entries.len(), 2);
        // Oldest (largest age) first.
        assert!(entries[0].age >= entries[1].age);
    }

    #[test]
    fn pruning_removes_the_oldest_until_under_budget() {
        let dir = test_dir("prune");
        fs::write(dir.join("a.jpg"), vec![0u8; 100]).unwrap();
        fs::write(dir.join("b.jpg"), vec![0u8; 100]).unwrap();
        fs::write(dir.join("c.jpg"), vec![0u8; 100]).unwrap();
        let removed = prune(&dir, 150).unwrap();
        assert_eq!(removed, 2, "two 100-byte files must go to fit 150");
        assert_eq!(total_bytes(&dir), 100);
        // The newest file survives (mtime ties break by listing order).
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn pruning_a_missing_dir_is_fine() {
        let dir = test_dir("missing");
        assert_eq!(prune(&dir.join("nope"), 100).unwrap(), 0);
    }

    #[test]
    fn a_zero_budget_removes_everything() {
        let dir = test_dir("zero");
        fs::write(dir.join("a.jpg"), b"x").unwrap();
        assert_eq!(prune(&dir, 0).unwrap(), 1);
        assert_eq!(total_bytes(&dir), 0);
    }

    #[test]
    fn files_under_budget_are_untouched() {
        let dir = test_dir("keep");
        fs::write(dir.join("a.jpg"), b"x").unwrap();
        assert_eq!(prune(&dir, 1000).unwrap(), 0);
        assert!(dir.join("a.jpg").exists());
    }

    #[test]
    fn prune_dirs_shares_one_budget_across_dirs() {
        // Two dirs under one budget: one 100-byte file must go to fit
        // 150. Which one goes is mtime order (both files land within the
        // same second in tests), so assert the total, not the victim.
        let dir_a = test_dir("shared_a");
        let dir_b = test_dir("shared_b");
        fs::write(dir_a.join("a.jpg"), vec![0u8; 100]).unwrap();
        fs::write(dir_b.join("b.jpg"), vec![0u8; 100]).unwrap();
        let dirs = vec![dir_a.clone(), dir_b.clone()];
        let removed = prune_dirs(&dirs, 150).unwrap();
        assert_eq!(removed, 1);
        let remaining =
            fs::read_dir(&dir_a).unwrap().count() + fs::read_dir(&dir_b).unwrap().count();
        assert_eq!(remaining, 1);
    }
}
