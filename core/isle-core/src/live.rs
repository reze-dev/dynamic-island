//! The live-activity engine.
//!
//! Everything the island can react to becomes an [`Activity`]. Activities are
//! ranked by kind, carry a tone (which the island turns into a dot colour),
//! and pass through a queue that knows how long each one may hold the
//! island's attention.
//!
//! Two ideas do the work:
//!
//! * **ambient** — a sticky activity describes a persistent condition: music
//!   playing, a timer running, a meeting joined. It colours the idle dot and
//!   is what the hover peek shows.
//! * **trigger** — a transient activity is an *event*: a notification
//!   arrived, the clipboard burst. It grows the island, is shown for a dwell
//!   time, then gets out of the way.
//!
//! The two are orthogonal. "Now playing" is both ambient (it keeps colouring
//! the dot) and a trigger (it grew the island when the track changed).

use serde::{Deserialize, Serialize};

/// How long a transient activity may hold the island open, in milliseconds.
pub const DEFAULT_DWELL_MS: u64 = 5_000;

/// How many recent activities are remembered. The last one is what the peek
/// view falls back to when nothing is ambient.
pub const MAX_RECENT: usize = 24;

/// The nature of a trigger.
///
/// The ordering of this enum is the collision rule from the plan: media beats
/// notifications, notifications beat timers, and so on down to generic system
/// noise. It is deliberately *not* the order of declaration but the order of
/// [`ActivityKind::priority`], because the declaration order reads better as
/// a catalogue of what the island understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivityKind {
    /// Something is playing through MPRIS.
    Media,
    /// A notification arrived.
    Notification,
    /// The pomodoro timer is running.
    Timer,
    /// A screen cast is running, or a meeting was detected.
    Meeting,
    /// The battery is low.
    LowBattery,
    /// A focus session is in progress.
    Focus,
    /// A todo is due.
    Todo,
    /// The clipboard changed.
    Clipboard,
    /// The network degraded or came back.
    Network,
    /// Anything else the machine wants to say.
    System,
}

impl ActivityKind {
    /// Higher wins a collision.
    pub const fn priority(self) -> u8 {
        match self {
            Self::Media => 100,
            Self::Notification => 90,
            Self::Timer => 80,
            Self::Meeting => 70,
            Self::LowBattery => 60,
            Self::Focus => 50,
            Self::Todo => 40,
            Self::Clipboard => 30,
            Self::Network => 20,
            Self::System => 10,
        }
    }

    /// The tone a kind gets unless a caller overrides it. The island maps the
    /// tone onto a dot colour.
    pub const fn default_tone(self) -> ActivityTone {
        match self {
            Self::Media => ActivityTone::Info,
            Self::Notification => ActivityTone::Warn,
            Self::Timer => ActivityTone::Good,
            Self::Meeting => ActivityTone::Critical,
            Self::LowBattery => ActivityTone::Critical,
            Self::Focus => ActivityTone::Info,
            Self::Todo => ActivityTone::Neutral,
            Self::Clipboard => ActivityTone::Neutral,
            Self::Network => ActivityTone::Warn,
            Self::System => ActivityTone::Neutral,
        }
    }
}

/// How alarmed the island should look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivityTone {
    /// Nothing is wrong. A dim dot.
    Neutral,
    /// Informational. The accent colour.
    Info,
    /// Something good. Green.
    Good,
    /// Worth a look. Amber.
    Warn,
    /// Needs attention now. Red.
    Critical,
}

/// One thing the island can be showing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// Assigned by the queue on insert.
    pub id: u64,
    pub kind: ActivityKind,
    pub tone: ActivityTone,
    /// The headline. One line, never wrapped.
    pub title: String,
    /// The supporting line under the headline.
    #[serde(default)]
    pub subtitle: String,
    /// A glyph name the UI knows how to draw.
    #[serde(default)]
    pub glyph: String,
    /// Optional third line, shown in the triggered state when there is room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub created_at_unix: u64,
    /// How long the island stays open for this activity, in milliseconds.
    #[serde(default = "default_dwell_ms")]
    pub dwell_ms: u64,
    /// A sticky activity describes a condition rather than an event. It stays
    /// in the ambient pool and keeps colouring the idle dot.
    #[serde(default)]
    pub sticky: bool,
    /// What produced this activity — a notification id, an MPRIS player, a
    /// network interface. Sticky activities with the same kind and source
    /// replace each other rather than pile up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

fn default_dwell_ms() -> u64 {
    DEFAULT_DWELL_MS
}

impl Activity {
    /// A transient activity with the kind's default tone and dwell.
    pub fn new(kind: ActivityKind, title: impl Into<String>, now_unix: u64) -> Self {
        Self {
            id: 0,
            kind,
            tone: kind.default_tone(),
            title: title.into(),
            subtitle: String::new(),
            glyph: String::new(),
            detail: None,
            created_at_unix: now_unix,
            dwell_ms: DEFAULT_DWELL_MS,
            sticky: false,
            source: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    pub fn with_tone(mut self, tone: ActivityTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn with_glyph(mut self, glyph: impl Into<String>) -> Self {
        self.glyph = glyph.into();
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Make this a persistent condition rather than an event.
    pub fn sticky(mut self) -> Self {
        self.sticky = true;
        self
    }

    pub fn with_dwell_ms(mut self, dwell_ms: u64) -> Self {
        self.dwell_ms = dwell_ms;
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// True once a transient activity has outlived its dwell. Sticky
    /// activities never expire — they are a condition, not an event.
    pub fn expired(&self, now_unix: u64) -> bool {
        if self.sticky {
            return false;
        }
        if self.dwell_ms == 0 {
            return true;
        }
        let age_ms = now_unix
            .saturating_sub(self.created_at_unix)
            .saturating_mul(1000);
        age_ms >= self.dwell_ms
    }
}

/// The ordered set of activities the island knows about.
///
/// The queue keeps two views: the *ambient* pool of sticky activities, ranked
/// by priority, and a bounded list of *recent* activities, newest first.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityQueue {
    #[serde(default)]
    ambient: Vec<Activity>,
    #[serde(default)]
    recent: Vec<Activity>,
    #[serde(default = "one")]
    next_id: u64,
}

fn one() -> u64 {
    1
}

impl Default for ActivityQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityQueue {
    pub fn new() -> Self {
        Self {
            ambient: Vec::new(),
            recent: Vec::new(),
            next_id: 1,
        }
    }

    /// Insert an activity and return the id it was given.
    ///
    /// A sticky activity replaces any previous activity with the same kind and
    /// source — "now playing" is one thing, not a growing list of songs. A
    /// transient activity is always appended.
    pub fn push(&mut self, activity: Activity) -> u64 {
        let mut activity = activity;
        activity.id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let id = activity.id;

        if activity.sticky {
            self.ambient
                .retain(|a| !(a.kind == activity.kind && a.source == activity.source));
            self.ambient.push(activity.clone());
            self.ambient
                .sort_by_key(|a| std::cmp::Reverse(a.kind.priority()));
        }

        self.recent.insert(0, activity);
        self.recent.truncate(MAX_RECENT);
        id
    }

    /// The highest-priority persistent condition, if any. This is what
    /// colours the idle dot and what the hover peek shows.
    pub fn ambient(&self) -> Option<&Activity> {
        self.ambient.first()
    }

    /// Every persistent condition, ranked.
    pub fn ambient_all(&self) -> &[Activity] {
        &self.ambient
    }

    /// Everything the queue has seen, newest first.
    pub fn recent(&self) -> &[Activity] {
        &self.recent
    }

    /// The most recent activity of any kind. The peek view falls back to this
    /// when nothing is ambient, so the island can still say what it last saw.
    pub fn last(&self) -> Option<&Activity> {
        self.recent.first()
    }

    /// The newest transient activity that has not outlived its dwell.
    pub fn live_trigger(&self, now_unix: u64) -> Option<&Activity> {
        self.recent
            .iter()
            .find(|a| !a.sticky && !a.expired(now_unix))
    }

    pub fn get(&self, id: u64) -> Option<&Activity> {
        self.recent.iter().find(|a| a.id == id)
    }

    /// Forget an activity. Returns whether anything was removed.
    pub fn dismiss(&mut self, id: u64) -> bool {
        let before = self.ambient.len() + self.recent.len();
        self.ambient.retain(|a| a.id != id);
        self.recent.retain(|a| a.id != id);
        before != self.ambient.len() + self.recent.len()
    }

    pub fn clear(&mut self) {
        self.ambient.clear();
        self.recent.clear();
    }

    /// Drop transient activities that have outlived their dwell and return
    /// their ids. Sticky ones stay: they describe a condition, not an event.
    pub fn expire(&mut self, now_unix: u64) -> Vec<u64> {
        let mut gone = Vec::new();
        self.recent.retain(|a| {
            if a.sticky || !a.expired(now_unix) {
                true
            } else {
                gone.push(a.id);
                false
            }
        });
        gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(now: u64) -> Activity {
        Activity::new(ActivityKind::Media, "Glasshouse", now)
            .with_subtitle("Aether")
            .with_glyph("music")
            .sticky()
            .with_source("spotify")
    }

    fn notification(now: u64) -> Activity {
        Activity::new(ActivityKind::Notification, "Build completed", now)
            .with_subtitle("northstar")
            .with_glyph("bell")
    }

    #[test]
    fn priority_follows_the_collision_rule() {
        assert!(ActivityKind::Media.priority() > ActivityKind::Notification.priority());
        assert!(ActivityKind::Notification.priority() > ActivityKind::Timer.priority());
        assert!(ActivityKind::Timer.priority() > ActivityKind::Meeting.priority());
        assert!(ActivityKind::Meeting.priority() > ActivityKind::LowBattery.priority());
        assert!(ActivityKind::LowBattery.priority() > ActivityKind::Focus.priority());
        assert!(ActivityKind::Focus.priority() > ActivityKind::Todo.priority());
        assert!(ActivityKind::Todo.priority() > ActivityKind::Clipboard.priority());
        assert!(ActivityKind::Clipboard.priority() > ActivityKind::Network.priority());
        assert!(ActivityKind::Network.priority() > ActivityKind::System.priority());
    }

    #[test]
    fn every_kind_has_a_default_tone() {
        for kind in [
            ActivityKind::Media,
            ActivityKind::Notification,
            ActivityKind::Timer,
            ActivityKind::Meeting,
            ActivityKind::LowBattery,
            ActivityKind::Focus,
            ActivityKind::Todo,
            ActivityKind::Clipboard,
            ActivityKind::Network,
            ActivityKind::System,
        ] {
            assert_eq!(Activity::new(kind, "x", 0).tone, kind.default_tone());
        }
    }

    #[test]
    fn ids_are_monotonic() {
        let mut q = ActivityQueue::new();
        assert_eq!(q.push(notification(0)), 1);
        assert_eq!(q.push(notification(0)), 2);
        assert_eq!(q.push(notification(0)), 3);
    }

    #[test]
    fn a_sticky_activity_replaces_its_own_predecessor() {
        let mut q = ActivityQueue::new();
        q.push(media(0));
        q.push(media(10));
        assert_eq!(q.ambient().map(|a| a.id), Some(2));
        assert_eq!(q.ambient_all().len(), 1);
    }

    #[test]
    fn a_sticky_activity_keeps_its_history() {
        let mut q = ActivityQueue::new();
        q.push(media(0));
        q.push(media(10));
        assert_eq!(q.recent().len(), 2);
        assert_eq!(q.last().map(|a| a.id), Some(2));
    }

    #[test]
    fn ambient_is_ranked_by_priority() {
        let mut q = ActivityQueue::new();
        q.push(Activity::new(ActivityKind::System, "disk", 0).sticky());
        q.push(media(0));
        assert_eq!(q.ambient().map(|a| a.kind), Some(ActivityKind::Media));
    }

    #[test]
    fn a_transient_activity_is_not_ambient() {
        let mut q = ActivityQueue::new();
        q.push(notification(0));
        assert!(q.ambient().is_none());
        assert_eq!(
            q.live_trigger(0).map(|a| a.kind),
            Some(ActivityKind::Notification)
        );
    }

    #[test]
    fn a_transient_activity_stops_being_a_live_trigger_after_its_dwell() {
        let mut q = ActivityQueue::new();
        q.push(Activity::new(ActivityKind::Notification, "hi", 0).with_dwell_ms(5_000));
        assert!(q.live_trigger(4).is_some());
        assert!(q.live_trigger(5).is_none());
    }

    #[test]
    fn expiry_removes_transients_but_keeps_ambient() {
        let mut q = ActivityQueue::new();
        q.push(media(0));
        q.push(notification(0));
        let gone = q.expire(60);
        assert_eq!(gone.len(), 1);
        assert!(q.ambient().is_some());
        assert_eq!(q.recent().len(), 1);
    }

    #[test]
    fn a_sticky_activity_never_expires() {
        let mut q = ActivityQueue::new();
        q.push(media(0));
        assert!(q.expire(u64::MAX / 2).is_empty());
        assert!(q.ambient().is_some());
    }

    #[test]
    fn dismissing_removes_from_both_views() {
        let mut q = ActivityQueue::new();
        let id = q.push(media(0));
        assert!(q.dismiss(id));
        assert!(q.ambient().is_none());
        assert!(q.recent().is_empty());
        assert!(!q.dismiss(id));
    }

    #[test]
    fn the_recent_list_is_bounded() {
        let mut q = ActivityQueue::new();
        for _ in 0..(MAX_RECENT + 10) {
            q.push(notification(0));
        }
        assert_eq!(q.recent().len(), MAX_RECENT);
    }

    #[test]
    fn an_activity_round_trips_through_json() {
        let mut q = ActivityQueue::new();
        q.push(media(0));
        q.push(notification(0));
        let json = serde_json::to_string(&q).unwrap();
        let back: ActivityQueue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ambient().map(|a| a.kind), Some(ActivityKind::Media));
        assert_eq!(back.recent().len(), 2);
        assert_eq!(back.last().map(|a| a.id), q.last().map(|a| a.id));
    }

    #[test]
    fn a_tone_can_be_overridden() {
        let a = Activity::new(ActivityKind::Notification, "x", 0).with_tone(ActivityTone::Critical);
        assert_eq!(a.tone, ActivityTone::Critical);
    }

    #[test]
    fn the_kind_serialises_as_kebab_case() {
        let json = serde_json::to_string(&ActivityKind::LowBattery).unwrap();
        assert_eq!(json, "\"low-battery\"");
    }
}
