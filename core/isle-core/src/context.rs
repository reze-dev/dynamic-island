//! The context reducer and scorer.
//!
//! The compositor adapter and the D-Bus signals tell us about the world;
//! this module turns those observations into a `ContextState` (what is
//! happening now) and scores candidate activities so the island knows what
//! to say. It is pure logic — no I/O, no D-Bus, no sockets — so it can be
//! tested headlessly.

use serde::{Deserialize, Serialize};

use crate::live::{Activity, ActivityKind, ActivityTone};

/// What the machine is doing, reduced to one word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextState {
    #[default]
    Idle,
    Busy,
    Presenting,
    Meeting,
    LowBattery,
    Away,
    Fullscreen,
}

/// One observation from a signal source, before it becomes state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Signal {
    /// logind `IdleHint` (session idle).
    Idle { idle: bool },
    /// logind `SessionNew`/`SessionRemoved`.
    Session { present: bool },
    /// UPower battery level (0..=100) and charging state.
    Battery { percent: u8, charging: bool },
    /// A window is fullscreen (from the compositor).
    Fullscreen { active: bool },
    /// A window title looks like a meeting (zoom, meet, teams…).
    Meeting { title: String },
    /// A calendar event starts within the given seconds.
    CalendarSoon { title: String, seconds: i64 },
    /// The network degraded (or came back).
    Network { degraded: bool },
    /// The clipboard changed N times in the last window (burst).
    ClipboardBurst { count: u32 },
    /// A todo item is due (from the data layer).
    TodoDue { text: String },
}

impl Signal {
    /// Parse a signal from its wire form. Fixtures and tests build signals
    /// with the constructors; the wire map exists so the UI (or a script)
    /// can inject one over IPC.
    pub fn from_map(map: &serde_json::Value) -> Option<Self> {
        let kind = map.get("kind").and_then(|v| v.as_str())?;
        Some(match kind {
            "idle" => Self::Idle {
                idle: map.get("idle").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "session" => Self::Session {
                present: map.get("present").and_then(|v| v.as_bool()).unwrap_or(true),
            },
            "battery" => Self::Battery {
                percent: map.get("percent").and_then(|v| v.as_u64()).unwrap_or(100) as u8,
                charging: map.get("charging").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "fullscreen" => Self::Fullscreen {
                active: map.get("active").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "meeting" => Self::Meeting {
                title: map.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "calendar-soon" => Self::CalendarSoon {
                title: map.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                seconds: map.get("seconds").and_then(|v| v.as_i64()).unwrap_or(0),
            },
            "network" => Self::Network {
                degraded: map.get("degraded").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "clipboard-burst" => Self::ClipboardBurst {
                count: map.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            },
            "todo-due" => Self::TodoDue {
                text: map.get("text").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            _ => return None,
        })
    }
}

/// The folded result of every signal so far.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub state: ContextState,
    /// When the last transition happened (unix seconds).
    pub changed_at_unix: u64,
    /// The payload of the last signal (for the UI to show).
    pub last_title: String,
    pub last_subtitle: String,
}

impl Context {
    /// Fold one signal into the state. Returns whether the state changed.
    pub fn fold(&mut self, signal: &Signal, now_unix: u64) -> bool {
        let before = self.state;
        match signal {
            Signal::Idle { idle } => {
                self.state = if *idle {
                    ContextState::Away
                } else {
                    ContextState::Idle
                };
                self.last_title = if *idle {
                    "The machine is idle".into()
                } else {
                    "Back at the keyboard".into()
                };
            }
            Signal::Session { present } => {
                self.state = if *present {
                    ContextState::Idle
                } else {
                    ContextState::Away
                };
            }
            Signal::Battery { percent, charging } => {
                if *charging {
                    self.state = ContextState::Busy;
                    self.last_title = "Charging".into();
                } else if *percent <= 20 {
                    self.state = ContextState::LowBattery;
                    self.last_title = format!("Battery at {percent}%");
                } else if *percent <= 40 {
                    self.state = ContextState::Busy;
                    self.last_title = format!("Battery at {percent}%");
                } else if self.state == ContextState::LowBattery {
                    self.state = ContextState::Idle;
                }
            }
            Signal::Fullscreen { active } => {
                if *active {
                    self.state = ContextState::Fullscreen;
                    self.last_title = "Fullscreen window".into();
                } else if self.state == ContextState::Fullscreen {
                    self.state = ContextState::Busy;
                }
            }
            Signal::Meeting { title } => {
                self.state = ContextState::Meeting;
                self.last_title = title.clone();
                self.last_subtitle = "Meeting detected".into();
            }
            Signal::CalendarSoon { title, seconds } => {
                self.state = ContextState::Busy;
                self.last_title = title.clone();
                self.last_subtitle = if *seconds <= 0 {
                    "starting now".into()
                } else if *seconds < 60 {
                    format!("starts in {seconds}s")
                } else {
                    format!("starts in {}m", seconds / 60)
                };
            }
            Signal::Network { degraded } => {
                if *degraded {
                    self.state = ContextState::Busy;
                    self.last_subtitle = "Network is struggling".into();
                } else if self.state == ContextState::Busy {
                    self.state = ContextState::Idle;
                }
            }
            Signal::ClipboardBurst { .. } => {}
            Signal::TodoDue { text } => {
                // Sticky: the todo owns the pill until done.
                self.state = ContextState::Busy;
                self.last_title = text.clone();
                self.last_subtitle = "Todo due".into();
            }
        }
        let changed = self.state != before;
        if changed {
            self.changed_at_unix = now_unix;
        }
        changed
    }

    pub fn is_awake(&self) -> bool {
        !matches!(self.state, ContextState::Away)
    }

    pub fn is_presenting(&self) -> bool {
        matches!(
            self.state,
            ContextState::Presenting | ContextState::Fullscreen
        )
    }
}

/// Score a candidate activity in this context. Higher wins. The score is
/// the kind's base priority plus context modulation:
/// * away mutes everything except criticals
/// * fullscreen suppresses non-urgent kinds (notifications, network)
/// * meeting boosts meeting-adjacent kinds
pub fn score(ctx: &Context, activity: &Activity) -> i32 {
    let mut total = activity.kind.priority() as i32;
    match ctx.state {
        ContextState::Away => {
            if activity.tone != ActivityTone::Critical {
                return -1000;
            }
        }
        ContextState::Fullscreen => match activity.kind {
            ActivityKind::Notification | ActivityKind::Network | ActivityKind::System => {
                total -= 50;
            }
            ActivityKind::Meeting | ActivityKind::LowBattery | ActivityKind::Timer => {
                total += 10;
            }
            _ => {}
        },
        ContextState::Meeting => match activity.kind {
            ActivityKind::Meeting => total += 20,
            ActivityKind::Timer | ActivityKind::Todo => total -= 10,
            _ => {}
        },
        ContextState::LowBattery => match activity.kind {
            ActivityKind::LowBattery => total += 20,
            ActivityKind::Media => total -= 10,
            _ => {}
        },
        _ => {}
    }
    total
}

/// The activity the island should suggest now, if any.
pub fn suggest(ctx: &Context, candidates: &[Activity]) -> Option<Activity> {
    candidates
        .iter()
        .max_by_key(|a| score(ctx, a))
        .cloned()
}

/// Whether a candidate should be quiet in this context (fullscreen hides
/// notifications; away hides everything but criticals).
pub fn is_quiet(ctx: &Context, activity: &Activity) -> bool {
    match ctx.state {
        ContextState::Away => activity.tone != ActivityTone::Critical,
        ContextState::Fullscreen => matches!(
            activity.kind,
            ActivityKind::Notification | ActivityKind::Network | ActivityKind::System
        ),
        _ => false,
    }
}

/// Heuristic: does this window title look like a meeting/presenting app?
/// Matches the apps people actually present from, case-insensitively.
pub fn looks_like_meeting(title: &str, app_id: &str) -> bool {
    let hay = format!("{title} {app_id}").to_lowercase();
    const KEYS: &[&str] = &[
        "zoom",
        "meet",
        "teams",
        "webex",
        "gotomeeting",
        "skype",
        "discord",
        "slack",
        "present",
        "screen share",
        "screenshare",
        "wireless",
    ];
    KEYS.iter().any(|k| hay.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::ActivityKind;

    fn act(kind: ActivityKind) -> Activity {
        Activity::new(kind, "x", 0)
    }

    #[test]
    fn an_idle_fold_flips_to_away_and_back() {
        let mut ctx = Context::default();
        assert!(ctx.fold(&Signal::Idle { idle: true }, 1));
        assert_eq!(ctx.state, ContextState::Away);
        assert!(ctx.fold(&Signal::Idle { idle: false }, 2));
        assert_eq!(ctx.state, ContextState::Idle);
    }

    #[test]
    fn low_battery_flips_the_state() {
        let mut ctx = Context::default();
        ctx.fold(&Signal::Battery {
            percent: 15,
            charging: false,
        }, 1);
        assert_eq!(ctx.state, ContextState::LowBattery);
    }

    #[test]
    fn fullscreen_suppresses_notifications_in_scoring() {
        let ctx = Context {
            state: ContextState::Fullscreen,
            ..Context::default()
        };
        let notif = act(ActivityKind::Notification);
        let meeting = act(ActivityKind::Meeting);
        assert!(is_quiet(&ctx, &notif));
        assert!(!is_quiet(&ctx, &meeting));
        assert!(score(&ctx, &meeting) > score(&ctx, &notif));
    }

    #[test]
    fn away_mutes_everything_but_criticals() {
        let ctx = Context {
            state: ContextState::Away,
            ..Context::default()
        };
        let media = act(ActivityKind::Media);
        assert!(is_quiet(&ctx, &media));
        let critical = Activity::new(ActivityKind::LowBattery, "x", 0)
            .with_tone(ActivityTone::Critical);
        assert!(!is_quiet(&ctx, &critical));
    }

    #[test]
    fn suggestion_picks_the_highest_score() {
        let ctx = Context::default();
        let cands = vec![
            act(ActivityKind::System),
            act(ActivityKind::Notification),
            act(ActivityKind::Media),
        ];
        let best = suggest(&ctx, &cands).unwrap();
        assert_eq!(best.kind, ActivityKind::Media);
    }

    #[test]
    fn meeting_titles_are_detected() {
        assert!(looks_like_meeting("Zoom Meeting", "zoom"));
        assert!(looks_like_meeting("Weekly sync — Teams", "teams"));
        assert!(!looks_like_meeting("GitHub — Mozilla Firefox", "firefox"));
    }

    #[test]
    fn signals_round_trip_through_json() {
        let s = Signal::Battery {
            percent: 42,
            charging: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn signal_from_map_parses_wire_forms() {
        let wire = serde_json::json!({"kind": "idle", "idle": true});
        assert_eq!(
            Signal::from_map(&wire),
            Some(Signal::Idle { idle: true })
        );
        assert_eq!(Signal::from_map(&serde_json::json!({"kind": "bogus"})), None);
    }
}
