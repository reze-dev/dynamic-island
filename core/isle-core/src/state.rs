use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Compositor {
    Niri,
    Hyprland,
    #[default]
    Unknown,
}

impl Compositor {
    pub fn detect() -> Self {
        if std::env::var_os("NIRI_SOCKET").is_some_and(|v| !v.is_empty()) {
            Self::Niri
        } else if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some_and(|v| !v.is_empty()) {
            Self::Hyprland
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub compositor: Compositor,
    pub wayland_display: Option<String>,
    pub started_at_unix: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Features {
    pub network_manager: bool,
    pub bluetooth: bool,
    pub mpris: bool,
    #[serde(default)]
    pub notifications: bool,
    pub screencast: bool,
    pub wallpaper_engine: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub uptime_secs: u64,
    pub load_average: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub session: Session,
    pub features: Features,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notifications: Vec<Notification>,
    /// The persistent condition that owns the island right now, if any. The
    /// UI colours the idle dot from its tone.
    ///
    /// Boxed because an activity is a fat struct and the snapshot is
    /// constructed on every connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Box<crate::live::Activity>>,
    /// Everything the island has seen recently, newest first. The peek view
    /// falls back to the first entry when nothing is ambient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activities: Vec<crate::live::Activity>,
}

impl Snapshot {
    pub fn new(session: Session, features: Features) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            session,
            features,
            notifications: Vec::new(),
            activity: None,
            activities: Vec::new(),
        }
    }
}

impl Snapshot {
    pub fn with_notifications(mut self, notifications: &[Notification]) -> Self {
        self.notifications = notifications.to_vec();
        self
    }

    /// Attach the live-activity state. The ambient activity is the one the
    /// island is currently anchored to.
    pub fn with_activities(
        mut self,
        ambient: Option<&crate::live::Activity>,
        recent: &[crate::live::Activity],
    ) -> Self {
        self.activity = ambient.cloned().map(Box::new);
        self.activities = recent.to_vec();
        self
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Milliseconds since the epoch. Timers need finer resolution than a
/// countdown rendered once a second can offer, and the pomodoro ring wants
/// to move smoothly rather than tick.
pub fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_carries_the_protocol_version() {
        let snapshot = Snapshot::new(
            Session {
                compositor: Compositor::Niri,
                wayland_display: Some("wayland-1".into()),
                started_at_unix: 1,
            },
            Features::default(),
        );
        assert_eq!(snapshot.version, PROTOCOL_VERSION);
    }

    #[test]
    fn snapshot_default_is_still_a_well_formed_message() {
        assert!(serde_json::to_string(&Snapshot::default()).is_ok());
    }

    #[test]
    fn compositor_serialises_in_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Compositor::Hyprland).unwrap(),
            r#""hyprland""#
        );
    }

    #[test]
    fn detecting_a_compositor_outside_a_session_yields_unknown() {
        // The test harness runs with a scrubbed environment, so this asserts the
        // no-session branch rather than a specific compositor.
        if std::env::var_os("NIRI_SOCKET").is_none()
            && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
        {
            assert_eq!(Compositor::detect(), Compositor::Unknown);
        }
    }
}

pub const MAX_NOTIFICATIONS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloseReason {
    Expired,
    Dismissed,
    ClosedByCaller,
    Other,
}

impl CloseReason {
    pub fn as_code(&self) -> u32 {
        match self {
            Self::Expired => 1,
            Self::Dismissed => 2,
            Self::ClosedByCaller => 3,
            Self::Other => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HintValue {
    Byte(u8),
    Bool(bool),
    Int(i32),
    Str(String),
    Strs(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hints: BTreeMap<String, HintValue>,
    pub expire_timeout: i32,
    pub received_at_unix: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urgency: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub transient: bool,
    #[serde(default)]
    pub resident: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_image: bool,
}

impl Notification {
    pub fn urgency_level(&self) -> u8 {
        self.urgency.unwrap_or(1)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Notifier {
    next_id: u32,
    notifications: Vec<Notification>,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            notifications: Vec::new(),
        }
    }

    pub fn push(&mut self, incoming: Notification) -> u32 {
        let id = if incoming.id != 0 && self.get(incoming.id).is_some() {
            self.notifications.retain(|n| n.id != incoming.id);
            incoming.id
        } else {
            let id = self.next_id;
            self.next_id = self.next_id.saturating_add(1);
            id
        };

        let mut stored = incoming;
        stored.id = id;
        self.notifications.push(stored);

        while self.notifications.len() > MAX_NOTIFICATIONS {
            self.notifications.remove(0);
        }

        id
    }

    pub fn get(&self, id: u32) -> Option<&Notification> {
        self.notifications.iter().find(|n| n.id == id)
    }

    pub fn list(&self) -> &[Notification] {
        &self.notifications
    }

    pub fn dismiss(&mut self, id: u32) -> Option<Notification> {
        let found = self.notifications.iter().position(|n| n.id == id)?;
        Some(self.notifications.remove(found))
    }

    pub fn dismiss_all(&mut self) -> Vec<Notification> {
        std::mem::take(&mut self.notifications)
    }

    pub fn expired_ids(&self, now: u64) -> Vec<u32> {
        self.notifications
            .iter()
            .filter(|n| {
                n.expire_timeout > 0
                    && n.received_at_unix
                        .saturating_add(n.expire_timeout as u64 / 1000)
                        .saturating_add(1)
                        < now
            })
            .map(|n| n.id)
            .collect()
    }
}

#[cfg(test)]
mod notifier_tests {
    use super::*;

    fn note(app: &str, summary: &str) -> Notification {
        Notification {
            id: 0,
            app_name: app.into(),
            app_icon: String::new(),
            summary: summary.into(),
            body: String::new(),
            actions: Vec::new(),
            hints: BTreeMap::new(),
            expire_timeout: 0,
            received_at_unix: 100,
            desktop_entry: None,
            urgency: None,
            category: None,
            transient: false,
            resident: false,
            has_image: false,
        }
    }

    fn with_id(mut n: Notification, id: u32) -> Notification {
        n.id = id;
        n
    }

    fn with_expiry(mut n: Notification, ms: i32) -> Notification {
        n.expire_timeout = ms;
        n
    }

    #[test]
    fn push_assigns_monotonic_ids() {
        let mut n = Notifier::new();
        assert_eq!(n.push(note("a", "one")), 1);
        assert_eq!(n.push(note("b", "two")), 2);
        assert_eq!(n.list().len(), 2);
    }

    #[test]
    fn replaces_in_place_and_keeps_the_original_id() {
        let mut n = Notifier::new();
        let first = n.push(note("a", "one"));
        let second = n.push(note("b", "two"));
        let _unrelated = n.push(note("c", "REPLACED"));
        assert_eq!(n.list().len(), 3);

        let replaced = n.push(with_id(note("c", "again"), first));
        assert_eq!(replaced, first);
        assert_eq!(n.list().len(), 3);
        assert_eq!(n.get(first).unwrap().summary, "again");
        assert_eq!(n.get(second).unwrap().summary, "two");
    }

    #[test]
    fn a_replace_of_an_unknown_id_is_an_append() {
        let mut n = Notifier::new();
        let id = n.push(with_id(note("a", "x"), 999));
        assert_eq!(n.list().len(), 1);
        assert_ne!(id, 999);
    }

    #[test]
    fn dismissing_by_replaces_id_says_goodbye_to_the_original() {
        let mut n = Notifier::new();
        let first = n.push(note("a", "one"));
        n.push(note("b", "two"));
        let gone = n.dismiss(first).unwrap();
        assert_eq!(gone.summary, "one");
        assert_eq!(n.list().len(), 1);
        assert!(n.dismiss(first).is_none());
    }

    #[test]
    fn dismiss_all_empties_the_store() {
        let mut n = Notifier::new();
        n.push(note("a", "one"));
        n.push(note("b", "two"));
        assert_eq!(n.dismiss_all().len(), 2);
        assert!(n.list().is_empty());
    }

    #[test]
    fn zero_expiry_means_never_expire() {
        let mut n = Notifier::new();
        n.push(note("a", "forever"));
        assert!(n.expired_ids(1_000_000).is_empty());
    }

    #[test]
    fn a_positive_expiry_expires_after_the_timeout() {
        let mut n = Notifier::new();
        n.push(with_expiry(note("a", "gone"), 5000));
        assert!(n.expired_ids(100).is_empty());
        let expired = n.expired_ids(200_000);
        assert_eq!(expired, vec![1]);
    }

    #[test]
    fn expiry_never_reports_the_same_id_twice() {
        let mut n = Notifier::new();
        n.push(with_expiry(note("a", "gone"), 1000));
        assert_eq!(n.expired_ids(1_000_000).len(), 1);
        assert!(n.dismiss(1).is_some());
        assert!(n.expired_ids(1_000_000).is_empty());
    }

    #[test]
    fn the_store_stays_bounded() {
        let mut n = Notifier::new();
        for i in 0..(MAX_NOTIFICATIONS + 40) {
            n.push(note("a", &format!("{i}")));
        }
        assert_eq!(n.list().len(), MAX_NOTIFICATIONS);
    }

    #[test]
    fn urgency_defaults_to_normal() {
        assert_eq!(note("a", "one").urgency_level(), 1);
    }

    #[test]
    fn an_explicit_urgency_wins() {
        let mut item = note("a", "one");
        item.urgency = Some(2);
        assert_eq!(item.urgency_level(), 2);
    }

    #[test]
    fn close_reason_codes_match_the_spec() {
        assert_eq!(CloseReason::Expired.as_code(), 1);
        assert_eq!(CloseReason::Dismissed.as_code(), 2);
        assert_eq!(CloseReason::ClosedByCaller.as_code(), 3);
        assert_eq!(CloseReason::Other.as_code(), 4);
    }

    #[test]
    fn notifications_survive_a_json_round_trip() {
        let mut n = Notifier::new();
        let mut item = note("a", "bold");
        item.urgency = Some(2);
        item.category = Some("im.received".into());
        item.hints.insert("urgency".into(), HintValue::Byte(2));
        item.actions.push(Action {
            key: "default".into(),
            label: "Open".into(),
        });
        n.push(item);

        let json = serde_json::to_string(n.list()).unwrap();
        let back: Vec<Notification> = serde_json::from_str(&json).unwrap();
        assert_eq!(back[0].urgency_level(), 2);
        assert_eq!(back[0].category.as_deref(), Some("im.received"));
        assert_eq!(back[0].actions.len(), 1);
        assert_eq!(back[0].actions[0].key, "default");
        assert_eq!(back[0].hints.get("urgency"), Some(&HintValue::Byte(2)));
    }

    #[test]
    fn a_snapshot_carries_its_notifications() {
        let mut n = Notifier::new();
        n.push(note("a", "hello"));
        let snapshot =
            Snapshot::new(Session::default(), Features::default()).with_notifications(n.list());
        assert_eq!(snapshot.notifications.len(), 1);
        assert_eq!(snapshot.notifications[0].summary, "hello");
    }

    #[test]
    fn each_hint_variant_serialises_under_its_own_key() {
        assert_eq!(
            serde_json::to_string(&HintValue::Byte(2)).unwrap(),
            r#"{"Byte":2}"#
        );
        assert_eq!(
            serde_json::to_string(&HintValue::Str("x".into())).unwrap(),
            r#"{"Str":"x"}"#
        );
    }

    #[test]
    fn every_hint_variant_round_trips() {
        let variants = [
            HintValue::Byte(9),
            HintValue::Bool(true),
            HintValue::Int(-3),
            HintValue::Str("hello".into()),
            HintValue::Strs(vec!["a".into(), "b".into()]),
        ];
        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let back: HintValue = serde_json::from_str(&json).unwrap();
            assert_eq!(back, original);
        }
    }
}
