//! The clipboard history.
//!
//! Entries are classified on the way in — a URL reads differently from a
//! path, and both read differently from a command line — because the panel
//! shows the kind as a label and the type decides how a paste-back behaves.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClipKind {
    Text,
    Url,
    Path,
    Code,
}

impl ClipKind {
    /// Guess what an entry is from its contents.
    pub fn classify(text: &str) -> Self {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Self::Text;
        }
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("obsidian://")
            || trimmed.starts_with("mailto:")
        {
            return Self::Url;
        }
        if trimmed.starts_with('/') && trimmed.contains('/') && !trimmed.contains(' ') {
            return Self::Path;
        }
        if trimmed.starts_with('$')
            || trimmed.starts_with("sudo ")
            || trimmed.starts_with("nix ")
            || trimmed.starts_with("just ")
            || trimmed.starts_with("git ")
            || trimmed.contains("(){")
            || trimmed.contains("=>")
        {
            return Self::Code;
        }
        Self::Text
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipEntry {
    pub id: u64,
    pub text: String,
    pub kind: ClipKind,
    pub created_at_unix: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub use_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clipboard {
    #[serde(default)]
    pub items: Vec<ClipEntry>,
    #[serde(default = "one")]
    pub next_id: u64,
    /// How many entries to keep. Old, unpinned entries fall off the end.
    #[serde(default = "default_capacity")]
    pub capacity: usize,
}

fn one() -> u64 {
    1
}

fn default_capacity() -> usize {
    200
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            capacity: default_capacity(),
        }
    }
}

impl Clipboard {
    /// Add an entry. Returns its id, or `None` if the text was already at
    /// the top of the list — copying the same thing twice should not fill
    /// the history with duplicates.
    pub fn push(&mut self, text: &str, now_unix: u64) -> Option<u64> {
        let text = text.trim_end_matches(['\n', '\r']);
        if text.trim().is_empty() {
            return None;
        }
        if self.items.first().is_some_and(|e| e.text == text) {
            return None;
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.items.insert(
            0,
            ClipEntry {
                id,
                text: text.to_string(),
                kind: ClipKind::classify(text),
                created_at_unix: now_unix,
                pinned: false,
                use_count: 0,
            },
        );
        self.prune();
        Some(id)
    }

    pub fn get(&self, id: u64) -> Option<&ClipEntry> {
        self.items.iter().find(|e| e.id == id)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|e| e.id != id);
        before != self.items.len()
    }

    /// Pin or unpin an entry. Pinned entries are never pruned.
    pub fn set_pinned(&mut self, id: u64, pinned: bool) -> Option<bool> {
        let entry = self.items.iter_mut().find(|e| e.id == id)?;
        entry.pinned = pinned;
        Some(pinned)
    }

    /// Record that an entry was pasted back, and bump it to the top.
    pub fn use_entry(&mut self, id: u64) -> Option<String> {
        let text = self.get(id)?.text.clone();
        if let Some(entry) = self.items.iter_mut().find(|e| e.id == id) {
            entry.use_count = entry.use_count.saturating_add(1);
        }
        Some(text)
    }

    pub fn clear(&mut self) -> usize {
        let count = self.items.len();
        self.items.clear();
        count
    }

    /// Drop unpinned entries beyond the capacity. Pinned entries are exempt,
    /// the same way favourited wallpapers are exempt from pruning.
    fn prune(&mut self) {
        let capacity = self.capacity;
        if self.items.len() <= capacity {
            return;
        }
        let mut kept = Vec::with_capacity(capacity);
        let mut dropped = 0;
        for entry in self.items.drain(..) {
            if entry.pinned || kept.len() < capacity {
                kept.push(entry);
            } else {
                dropped += 1;
            }
        }
        self.items = kept;
        let _ = dropped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_is_recognised() {
        assert_eq!(ClipKind::classify("https://quickshell.org"), ClipKind::Url);
        assert_eq!(ClipKind::classify("mailto:a@b.c"), ClipKind::Url);
    }

    #[test]
    fn a_path_is_recognised() {
        assert_eq!(ClipKind::classify("/home/me/Documents"), ClipKind::Path);
    }

    #[test]
    fn a_command_is_recognised_as_code() {
        assert_eq!(
            ClipKind::classify("nix develop .#devShells"),
            ClipKind::Code
        );
        assert_eq!(ClipKind::classify("git status"), ClipKind::Code);
    }

    #[test]
    fn plain_text_stays_text() {
        assert_eq!(ClipKind::classify("hello there"), ClipKind::Text);
        assert_eq!(ClipKind::classify(""), ClipKind::Text);
    }

    #[test]
    fn pushing_assigns_an_id_and_classifies() {
        let mut c = Clipboard::default();
        let id = c.push("https://example.com", 5).unwrap();
        assert_eq!(id, 1);
        assert_eq!(c.get(id).unwrap().kind, ClipKind::Url);
    }

    #[test]
    fn empty_text_is_not_added() {
        let mut c = Clipboard::default();
        assert!(c.push("   ", 0).is_none());
        assert!(c.items.is_empty());
    }

    #[test]
    fn the_same_text_twice_in_a_row_is_not_duplicated() {
        let mut c = Clipboard::default();
        c.push("hello", 0);
        assert!(c.push("hello", 1).is_none());
        assert_eq!(c.items.len(), 1);
    }

    #[test]
    fn a_trailing_newline_does_not_make_two_entries() {
        let mut c = Clipboard::default();
        c.push("hello\n", 0);
        assert!(c.push("hello", 1).is_none());
    }

    #[test]
    fn pinning_keeps_an_entry_out_of_the_prune() {
        let mut c = Clipboard {
            capacity: 2,
            ..Clipboard::default()
        };
        let pinned = c.push("keep me", 0).unwrap();
        c.set_pinned(pinned, true);
        for _ in 0..5 {
            c.push("noise", 0);
        }
        assert!(c.get(pinned).is_some());
    }

    #[test]
    fn the_history_is_bounded() {
        let mut c = Clipboard {
            capacity: 3,
            ..Clipboard::default()
        };
        for i in 0..10 {
            c.push(&format!("entry {i}"), 0);
        }
        assert_eq!(c.items.len(), 3);
    }

    #[test]
    fn using_an_entry_counts_it_and_returns_its_text() {
        let mut c = Clipboard::default();
        let id = c.push("hello", 0).unwrap();
        assert_eq!(c.use_entry(id).as_deref(), Some("hello"));
        assert_eq!(c.get(id).unwrap().use_count, 1);
    }

    #[test]
    fn clearing_empties_the_history() {
        let mut c = Clipboard::default();
        c.push("a", 0);
        c.push("b", 0);
        assert_eq!(c.clear(), 2);
        assert!(c.items.is_empty());
    }

    #[test]
    fn the_history_survives_a_round_trip_through_json() {
        let mut clip = Clipboard::default();
        clip.push("nix develop", 1);
        let json = serde_json::to_string(&clip).unwrap();
        let back: Clipboard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items[0].text, "nix develop");
        assert_eq!(back.items[0].kind, ClipKind::Code);
    }
}
