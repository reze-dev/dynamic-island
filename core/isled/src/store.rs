//! Persistence.
//!
//! Everything the shell remembers between sessions lives in two directories:
//! `~/.local/state/isled` for state the shell owns outright (todos, the timer,
//! the clipboard history) and `~/.local/share/isled` for data the user owns
//! (the wallpaper cache, note files).
//!
//! JSON rather than SQLite. The volumes involved are tiny — hundreds of rows
//! at most — and plain files are readable, diffable and trivial to repair by
//! hand. A database would buy nothing here and cost a build dependency.
//!
//! Writes are atomic: the payload lands in a sibling temporary file which is
//! then renamed over the target. A crash mid-write leaves the previous file
//! intact rather than a half-written one.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where mutable shell state lives.
pub fn state_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("isled");
    }
    home_dir().join(".local/state/isled")
}

/// Where user-owned data lives.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("isled");
    }
    home_dir().join(".local/share/isled")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// A directory of JSON documents.
#[derive(Debug, Clone)]
pub struct Store {
    state: PathBuf,
    data: PathBuf,
}

impl Store {
    /// The store rooted at the standard XDG locations.
    pub fn new() -> Self {
        Self {
            state: state_dir(),
            data: data_dir(),
        }
    }

    /// A store rooted somewhere else. Used by the tests so they never touch
    /// the real `~/.local/state`.
    pub fn rooted_at(state: PathBuf, data: PathBuf) -> Self {
        Self { state, data }
    }

    pub fn state_dir(&self) -> &Path {
        &self.state
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    fn path(&self, dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.json"))
    }

    /// Read a document. A missing or unreadable file is `None`, not an error:
    /// a corrupt file should degrade to "no data", not take the daemon down.
    pub fn read<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        self.read_from(&self.state, name)
    }

    pub fn read_from<T: DeserializeOwned>(&self, dir: &Path, name: &str) -> Option<T> {
        let raw = fs::read_to_string(self.path(dir, name)).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// Write a document, creating the directory if needed.
    pub fn write<T: Serialize>(&self, name: &str, value: &T) -> io::Result<()> {
        self.write_to(&self.state, name, value)
    }

    pub fn write_to<T: Serialize>(&self, dir: &Path, name: &str, value: &T) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let target = self.path(dir, name);
        let tmp = dir.join(format!(".{name}.json.tmp"));
        let raw = serde_json::to_vec_pretty(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp, raw)?;
        fs::rename(&tmp, &target)
    }

    /// Remove a document. Missing files are not an error.
    pub fn remove(&self, name: &str) -> io::Result<()> {
        let target = self.path(&self.state, name);
        match fs::remove_file(&target) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Doc {
        #[serde(default)]
        name: String,
        #[serde(default)]
        count: u32,
    }

    fn temp_store(tag: &str) -> Store {
        let base = std::env::temp_dir().join(format!("isled-store-{tag}-{}", std::process::id()));
        Store::rooted_at(base.join("state"), base.join("data"))
    }

    #[test]
    fn a_document_round_trips() {
        let store = temp_store("roundtrip");
        let doc = Doc {
            name: "isle".into(),
            count: 3,
        };
        store.write("thing", &doc).unwrap();
        let back: Doc = store.read("thing").unwrap();
        assert_eq!(back, doc);
        let _ = fs::remove_dir_all(store.state_dir().parent().unwrap());
    }

    #[test]
    fn a_missing_document_is_none_not_an_error() {
        let store = temp_store("missing");
        let back: Option<Doc> = store.read("nothing-here");
        assert!(back.is_none());
    }

    #[test]
    fn a_corrupt_document_is_none_not_a_panic() {
        let store = temp_store("corrupt");
        fs::create_dir_all(store.state_dir()).unwrap();
        fs::write(store.state_dir().join("broken.json"), "{not json").unwrap();
        let back: Option<Doc> = store.read("broken");
        assert!(back.is_none());
    }

    #[test]
    fn removing_a_document_is_idempotent() {
        let store = temp_store("remove");
        store.write("gone", &Doc::default()).unwrap();
        store.remove("gone").unwrap();
        store.remove("gone").unwrap();
        assert!(store.read::<Doc>("gone").is_none());
    }

    #[test]
    fn the_state_and_data_directories_are_separate() {
        let store = temp_store("separate");
        assert_ne!(store.state_dir(), store.data_dir());
    }
}
