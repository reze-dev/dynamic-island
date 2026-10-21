//! Quick notes.
//!
//! Notes are plain markdown files in a directory. That choice is what makes
//! the Obsidian integration free: an Obsidian vault *is* a directory of
//! markdown files, so pointing the notes directory at a vault makes every
//! note editable in Obsidian with no import step and no second copy.
//!
//! The title is the first `# heading` in the file. The body is everything
//! else. There is no front matter parser because there is no need for one.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// The file name without its extension. Stable, and usable as a key.
    pub id: String,
    pub title: String,
    pub body: String,
    /// Absolute path on disk.
    pub path: String,
    /// When the file was last modified, in seconds since the epoch.
    pub updated_at_unix: u64,
}

impl Note {
    /// The first line of the file, if it is a heading.
    pub fn title_from(body: &str) -> String {
        body.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

/// Where notes live and how they are read.
pub struct NoteStore {
    dir: PathBuf,
}

impl NoteStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every note in the directory, most recently changed first.
    pub fn list(&self) -> Vec<Note> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            if let Some(note) = self.read_path(&path) {
                notes.push(note);
            }
        }
        notes.sort_by(|a, b| {
            b.updated_at_unix
                .cmp(&a.updated_at_unix)
                .then_with(|| a.id.cmp(&b.id))
        });
        notes
    }

    pub fn get(&self, id: &str) -> Option<Note> {
        let path = self.path_for(id);
        self.read_path(&path)
    }

    /// Create or replace a note. Returns the note as written.
    pub fn write(&self, id: &str, title: &str, body: &str) -> std::io::Result<Note> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path_for(id);
        let content = if title.trim().is_empty() {
            body.to_string()
        } else if body.trim().is_empty() {
            format!("# {title}\n")
        } else {
            format!("# {title}\n\n{body}\n")
        };
        fs::write(&path, &content)?;
        Ok(Note {
            id: id.to_string(),
            title: if title.trim().is_empty() {
                Note::title_from(&content)
            } else {
                title.trim().to_string()
            },
            body: body.to_string(),
            path: path.to_string_lossy().into_owned(),
            updated_at_unix: modified_unix(&path),
        })
    }

    pub fn remove(&self, id: &str) -> bool {
        fs::remove_file(self.path_for(id)).is_ok()
    }

    fn path_for(&self, id: &str) -> PathBuf {
        let safe = sanitize(id);
        self.dir.join(format!("{safe}.md"))
    }

    fn read_path(&self, path: &Path) -> Option<Note> {
        let content = fs::read_to_string(path).ok()?;
        let id = path.file_stem()?.to_string_lossy().into_owned();
        Some(Note {
            title: Note::title_from(&content),
            body: body_without_title(&content),
            id,
            path: path.to_string_lossy().into_owned(),
            updated_at_unix: modified_unix(path),
        })
    }
}

/// A note's contents without the title line, for editing in the panel.
pub fn body_without_title(body: &str) -> String {
    if body.trim().is_empty() {
        return String::new();
    }
    let mut lines = body.lines();
    if let Some(first) = lines.next() {
        let trimmed = first.trim();
        if trimmed.starts_with('#') {
            return lines
                .collect::<Vec<_>>()
                .join("\n")
                .trim_start()
                .to_string();
        }
    }
    body.trim_start().to_string()
}

/// A file name that cannot escape the notes directory.
fn sanitize(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

fn modified_unix(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

/// The URI that opens a note in Obsidian, if it is installed.
///
/// Obsidian accepts `obsidian://open?path=<absolute path>`, which resolves
/// against every open vault. The panel offers the link; the desktop decides
/// whether anything happens.
pub fn obsidian_uri(path: &str) -> String {
    format!("obsidian://open?path={}", urlencode(path))
}

fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isled-notes-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_written_note_comes_back_with_its_title() {
        let dir = temp_dir("write");
        let store = NoteStore::new(dir.clone());
        let note = store
            .write("groceries", "Groceries", "milk\nbread")
            .unwrap();
        assert_eq!(note.title, "Groceries");
        let back = store.get("groceries").unwrap();
        assert_eq!(back.title, "Groceries");
        assert_eq!(back.body, "milk\nbread");
    }

    #[test]
    fn a_note_without_a_title_falls_back_to_its_first_line() {
        let dir = temp_dir("untitled");
        let store = NoteStore::new(dir.clone());
        fs::write(dir.join("raw.md"), "just some text\nmore").unwrap();
        let note = store.get("raw").unwrap();
        assert_eq!(note.title, "just some text");
    }

    #[test]
    fn an_empty_note_is_called_untitled() {
        assert_eq!(Note::title_from(""), "Untitled");
        assert_eq!(Note::title_from("   \n  \n"), "Untitled");
    }

    #[test]
    fn the_body_drops_the_title_line() {
        assert_eq!(body_without_title("# Title\n\nbody"), "body");
        assert_eq!(body_without_title("body only"), "body only");
    }

    #[test]
    fn listing_returns_notes_newest_first() {
        let dir = temp_dir("list");
        let store = NoteStore::new(dir.clone());
        store.write("one", "One", "a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        store.write("two", "Two", "b").unwrap();
        let notes = store.list();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].id, "two");
    }

    #[test]
    fn non_markdown_files_are_ignored() {
        let dir = temp_dir("filter");
        let store = NoteStore::new(dir.clone());
        store.write("real", "Real", "a").unwrap();
        fs::write(dir.join("notes.txt"), "not a note").unwrap();
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn a_missing_directory_lists_nothing_rather_than_failing() {
        let store = NoteStore::new(PathBuf::from("/definitely/not/here"));
        assert!(store.list().is_empty());
        assert!(store.get("nope").is_none());
    }

    #[test]
    fn an_id_cannot_escape_the_notes_directory() {
        assert_eq!(sanitize("../../etc/passwd"), "etc-passwd");
        assert_eq!(sanitize(""), "untitled");
        assert_eq!(sanitize("ok_name-1"), "ok_name-1");
    }

    #[test]
    fn removing_a_note_reports_whether_it_worked() {
        let dir = temp_dir("remove");
        let store = NoteStore::new(dir.clone());
        store.write("gone", "Gone", "a").unwrap();
        assert!(store.remove("gone"));
        assert!(!store.remove("gone"));
    }

    #[test]
    fn the_obsidian_uri_encodes_the_path() {
        let uri = obsidian_uri("/home/me/My Vault/note.md");
        assert!(uri.starts_with("obsidian://open?path="));
        assert!(uri.contains("%20"));
        assert!(!uri.contains(' '));
    }
}
