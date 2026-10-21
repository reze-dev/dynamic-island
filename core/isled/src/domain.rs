//! The mutable state the shell owns.
//!
//! One struct, one lock. The shell has a lot of small pieces of mutable
//! state — todos, a timer, a clipboard, a wallpaper catalog, the activity
//! queue — and they are read and written together far more often than they
//! are written independently. Keeping them under a single lock removes any
//! question of lock ordering, and the critical sections are short enough
//! that it does not matter.

use crate::data::clipboard::Clipboard;
use crate::data::notes::NoteStore;
use crate::data::timer::TimerState;
use crate::data::todos::TodoList;
use crate::ics::Calendar;
use crate::settings::Settings;
use crate::store::Store;
use crate::wallpaper::{Catalog, Engine};
use isle_core::live::ActivityQueue;

/// Everything the shell remembers.
pub struct Domain {
    pub settings: Settings,
    pub todos: TodoList,
    pub timer: TimerState,
    pub notes: NoteStore,
    pub clipboard: Clipboard,
    pub wallpaper: Catalog,
    pub activities: ActivityQueue,
    store: Store,
}

impl Domain {
    /// Load everything from disk, falling back to defaults for anything
    /// missing or unreadable.
    pub fn load(store: Store) -> Self {
        let settings = store.read::<Settings>("settings").unwrap_or_default();
        let todos = store.read::<TodoList>("todos").unwrap_or_default();
        let timer = store.read::<TimerState>("timer").unwrap_or_default();
        let clipboard = store.read::<Clipboard>("clipboard").unwrap_or_default();

        let engine = Engine::detect();
        let wallpaper = Catalog::scan(&settings.wallpaper_dir(), engine);

        Self {
            notes: NoteStore::new(settings.notes_dir()),
            todos,
            timer,
            clipboard,
            wallpaper,
            activities: ActivityQueue::new(),
            settings,
            store,
        }
    }

    /// The events on the calendar, read fresh.
    pub fn calendar(&self) -> Calendar {
        Calendar::from_dir(&self.settings.calendar_dir())
    }

    pub fn save_settings(&self) {
        if let Err(err) = self.store.write("settings", &self.settings) {
            tracing::warn!(%err, "could not save settings");
        }
    }

    pub fn save_todos(&self) {
        if let Err(err) = self.store.write("todos", &self.todos) {
            tracing::warn!(%err, "could not save todos");
        }
    }

    pub fn save_timer(&self) {
        if let Err(err) = self.store.write("timer", &self.timer) {
            tracing::warn!(%err, "could not save the timer");
        }
    }

    pub fn save_clipboard(&self) {
        if let Err(err) = self.store.write("clipboard", &self.clipboard) {
            tracing::warn!(%err, "could not save the clipboard");
        }
    }

    /// Point the notes at a different directory — an Obsidian vault, usually.
    pub fn set_notes_dir(&mut self, dir: std::path::PathBuf) {
        self.settings.notes_dir = dir.to_string_lossy().into_owned();
        self.notes = NoteStore::new(dir);
        self.save_settings();
    }

    /// Point the wallpaper catalog at a different directory and rescan.
    pub fn set_wallpaper_dir(&mut self, dir: std::path::PathBuf) {
        self.settings.wallpaper_dir = dir.to_string_lossy().into_owned();
        self.wallpaper = Catalog::scan(&dir, self.wallpaper.engine);
        self.save_settings();
    }

    /// Rescan the wallpaper directory without changing it.
    pub fn rescan_wallpapers(&mut self) {
        let engine = self.wallpaper.engine;
        self.wallpaper = Catalog::scan(&self.settings.wallpaper_dir(), engine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let base = std::env::temp_dir().join(format!("isled-domain-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        Store::rooted_at(base.join("state"), base.join("data"))
    }

    #[test]
    fn a_fresh_domain_has_default_settings() {
        let store = temp_store("fresh");
        let domain = Domain::load(store);
        assert_eq!(domain.settings.theme, "midnight");
        assert!(domain.todos.items.is_empty());
        assert_eq!(domain.timer.phase, crate::data::timer::TimerPhase::Idle);
    }

    #[test]
    fn todos_survive_a_reload() {
        let store = temp_store("todos");
        {
            let mut domain = Domain::load(store.clone());
            domain.todos.add("buy milk", None, 0, 1);
            domain.save_todos();
        }
        let domain = Domain::load(store);
        assert_eq!(domain.todos.items.len(), 1);
        assert_eq!(domain.todos.items[0].text, "buy milk");
    }

    #[test]
    fn the_clipboard_survives_a_reload() {
        let store = temp_store("clip");
        {
            let mut domain = Domain::load(store.clone());
            domain.clipboard.push("hello", 1);
            domain.save_clipboard();
        }
        let domain = Domain::load(store);
        assert_eq!(domain.clipboard.items[0].text, "hello");
    }

    #[test]
    fn changing_the_wallpaper_directory_rescans() {
        let store = temp_store("walls");
        let dir = std::env::temp_dir().join(format!("isled-walls-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.png"), b"x").unwrap();
        let mut domain = Domain::load(store);
        domain.set_wallpaper_dir(dir.clone());
        assert_eq!(domain.wallpaper.items.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn changing_the_notes_directory_repoints_the_store() {
        let store = temp_store("notes");
        let dir = std::env::temp_dir().join(format!("isled-notes-dir-{}", std::process::id()));
        let mut domain = Domain::load(store);
        domain.set_notes_dir(dir.clone());
        assert_eq!(domain.notes.dir(), dir);
    }
}
