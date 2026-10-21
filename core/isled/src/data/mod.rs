//! The things the shell remembers between sessions.
//!
//! Each submodule owns one kind of data and knows how to load and save it.
//! They are deliberately boring: plain serialisable structs, no framework.

pub mod clipboard;
pub mod notes;
pub mod timer;
pub mod todos;
