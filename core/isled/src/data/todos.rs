//! The todo list.
//!
//! Todos are time-stamped: each one may carry a due time, and the panel
//! groups them by when they are due rather than by insertion order. A todo
//! with no due time is an someday item and sorts after everything else.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    #[serde(default)]
    pub done: bool,
    /// When the todo is due, in seconds since the epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_unix: Option<u64>,
    #[serde(default)]
    pub created_at_unix: u64,
    /// 0 is the default. Higher is more urgent.
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoList {
    #[serde(default)]
    pub items: Vec<Todo>,
    #[serde(default = "one")]
    pub next_id: u64,
}

fn one() -> u64 {
    1
}

impl Default for TodoList {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }
}

impl TodoList {
    /// Add a todo and return its id.
    pub fn add(
        &mut self,
        text: impl Into<String>,
        due_unix: Option<u64>,
        priority: u8,
        now_unix: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.items.push(Todo {
            id,
            text: text.into(),
            done: false,
            due_unix,
            created_at_unix: now_unix,
            priority,
            note: String::new(),
        });
        id
    }

    /// Flip a todo's done flag. Returns the new state.
    pub fn toggle(&mut self, id: u64) -> Option<bool> {
        let todo = self.items.iter_mut().find(|t| t.id == id)?;
        todo.done = !todo.done;
        Some(todo.done)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.id != id);
        before != self.items.len()
    }

    /// Move a todo to a new index. Out-of-range indices are clamped rather
    /// than rejected — a drag that overshoots should still land somewhere.
    pub fn reorder(&mut self, id: u64, to: usize) -> bool {
        let Some(from) = self.items.iter().position(|t| t.id == id) else {
            return false;
        };
        let to = to.min(self.items.len().saturating_sub(1));
        let todo = self.items.remove(from);
        self.items.insert(to, todo);
        true
    }

    pub fn clear_done(&mut self) -> usize {
        let before = self.items.len();
        self.items.retain(|t| !t.done);
        before - self.items.len()
    }

    /// The todos in the order the panel shows them: unfinished first, then
    /// by due time, then by priority, then by when they were written.
    pub fn ordered(&self) -> Vec<&Todo> {
        let mut items: Vec<&Todo> = self.items.iter().collect();
        items.sort_by(|a, b| {
            a.done
                .cmp(&b.done)
                .then_with(|| match (a.due_unix, b.due_unix) {
                    (Some(x), Some(y)) => x.cmp(&y),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| b.priority.cmp(&a.priority))
                .then_with(|| a.created_at_unix.cmp(&b.created_at_unix))
                .then_with(|| a.id.cmp(&b.id))
        });
        items
    }

    /// The next todo that is due and not finished. This is what the island
    /// surfaces as a todo activity.
    pub fn next_due(&self, now_unix: u64) -> Option<&Todo> {
        self.items
            .iter()
            .filter(|t| !t.done)
            .filter(|t| t.due_unix.is_some_and(|d| d <= now_unix))
            .min_by_key(|t| (t.due_unix.unwrap_or(u64::MAX), t.id))
    }

    /// The next todo that is due, whether or not it is already past.
    pub fn next_upcoming(&self) -> Option<&Todo> {
        self.items
            .iter()
            .filter(|t| !t.done && t.due_unix.is_some())
            .min_by_key(|t| (t.due_unix.unwrap_or(u64::MAX), t.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_monotonic() {
        let mut list = TodoList::default();
        assert_eq!(list.add("a", None, 0, 0), 1);
        assert_eq!(list.add("b", None, 0, 0), 2);
    }

    #[test]
    fn toggling_flips_the_done_flag() {
        let mut list = TodoList::default();
        let id = list.add("a", None, 0, 0);
        assert_eq!(list.toggle(id), Some(true));
        assert_eq!(list.toggle(id), Some(false));
        assert_eq!(list.toggle(999), None);
    }

    #[test]
    fn removing_reports_whether_anything_went() {
        let mut list = TodoList::default();
        let id = list.add("a", None, 0, 0);
        assert!(list.remove(id));
        assert!(!list.remove(id));
    }

    #[test]
    fn reordering_moves_a_todo_and_clamps_out_of_range() {
        let mut list = TodoList::default();
        let a = list.add("a", None, 0, 0);
        let _b = list.add("b", None, 0, 0);
        let c = list.add("c", None, 0, 0);
        assert!(list.reorder(c, 0));
        assert_eq!(list.items[0].id, c);
        assert_eq!(list.items[1].id, a);
        assert!(list.reorder(a, 99));
        assert_eq!(list.items[2].id, a);
    }

    #[test]
    fn ordering_puts_finished_todos_last() {
        let mut list = TodoList::default();
        let a = list.add("a", None, 0, 0);
        list.add("b", None, 0, 0);
        list.toggle(a);
        let ordered = list.ordered();
        assert_eq!(ordered[0].text, "b");
        assert_eq!(ordered[1].text, "a");
    }

    #[test]
    fn ordering_sorts_by_due_time_then_priority() {
        let mut list = TodoList::default();
        list.add("late", Some(500), 0, 0);
        list.add("soon", Some(100), 0, 0);
        list.add("urgent", Some(100), 9, 0);
        let ordered = list.ordered();
        assert_eq!(ordered[0].text, "urgent");
        assert_eq!(ordered[1].text, "soon");
        assert_eq!(ordered[2].text, "late");
    }

    #[test]
    fn todos_without_a_due_time_sort_last() {
        let mut list = TodoList::default();
        list.add("someday", None, 0, 0);
        list.add("today", Some(100), 0, 0);
        let ordered = list.ordered();
        assert_eq!(ordered[0].text, "today");
        assert_eq!(ordered[1].text, "someday");
    }

    #[test]
    fn the_next_due_todo_is_the_earliest_past_its_time() {
        let mut list = TodoList::default();
        list.add("later", Some(500), 0, 0);
        list.add("now", Some(100), 0, 0);
        assert_eq!(list.next_due(200).map(|t| t.text.as_str()), Some("now"));
        assert_eq!(list.next_due(50), None);
    }

    #[test]
    fn a_finished_todo_is_never_due() {
        let mut list = TodoList::default();
        let id = list.add("now", Some(100), 0, 0);
        list.toggle(id);
        assert!(list.next_due(200).is_none());
    }

    #[test]
    fn clearing_done_only_removes_finished_todos() {
        let mut list = TodoList::default();
        let a = list.add("a", None, 0, 0);
        list.add("b", None, 0, 0);
        list.toggle(a);
        assert_eq!(list.clear_done(), 1);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].text, "b");
    }

    #[test]
    fn a_list_survives_a_round_trip_through_json() {
        let mut list = TodoList::default();
        list.add("write the thing", Some(42), 1, 7);
        let json = serde_json::to_string(&list).unwrap();
        let back: TodoList = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items[0].text, "write the thing");
        assert_eq!(back.items[0].due_unix, Some(42));
    }
}
