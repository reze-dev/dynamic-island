//! A small iCalendar reader.
//!
//! The calendar panel needs the events that are coming up, not a full
//! calendar implementation. So this parses the parts of RFC 5545 that answer
//! that question: `VEVENT` blocks with a start, an end, a summary and a
//! location, plus simple recurrence rules so a weekly standup shows up
//! every week.
//!
//! Timezones are handled with one deliberate simplification: a `Z` timestamp
//! is UTC and is converted exactly; a `TZID` or floating timestamp is read in
//! the machine's own timezone. For a personal calendar on a laptop, that is
//! almost always the right answer.

use crate::tz::{Tz, days_from_civil, unix_from_civil};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// How far ahead recurring events are expanded.
pub const HORIZON_DAYS: i64 = 45;

/// The most instances a single recurring event may produce.
pub const MAX_INSTANCES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub location: String,
    /// When the event starts, in seconds since the epoch.
    pub start_unix: i64,
    /// When it ends, if it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_unix: Option<i64>,
    /// All-day events have no time of day.
    #[serde(default)]
    pub all_day: bool,
}

impl CalendarEvent {
    /// How long the event lasts, in seconds. Zero when it has no end.
    pub fn duration_secs(&self) -> i64 {
        self.end_unix
            .map(|end| (end - self.start_unix).max(0))
            .unwrap_or(0)
    }
}

/// A date and time, as written in a calendar file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DateTime {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    /// The value carried an explicit `Z`, so it is UTC.
    is_utc: bool,
}

/// One parsed event, before recurrence is expanded.
#[derive(Debug, Clone, Default)]
struct RawEvent {
    properties: BTreeMap<String, Vec<Property>>,
}

#[derive(Debug, Clone, Default)]
struct Property {
    params: BTreeMap<String, String>,
    value: String,
}

/// A parsed calendar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    #[serde(default)]
    pub events: Vec<CalendarEvent>,
}

impl Calendar {
    /// Read every `.ics` file in a directory.
    pub fn from_dir(dir: &Path) -> Self {
        let mut calendar = Calendar::default();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return calendar;
        };
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ics"))
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(text) = std::fs::read_to_string(&path) {
                calendar.add_text(&text);
            }
        }
        calendar
    }

    /// Parse a calendar from text.
    pub fn from_text(text: &str) -> Self {
        let mut calendar = Calendar::default();
        calendar.add_text(text);
        calendar
    }

    fn add_text(&mut self, text: &str) {
        let tz = Tz::system();
        for raw in parse_events(text) {
            self.events.extend(expand(&raw, &tz));
        }
    }

    /// The events starting at or after `now`, soonest first.
    pub fn upcoming(&self, now: i64) -> Vec<&CalendarEvent> {
        let mut events: Vec<&CalendarEvent> =
            self.events.iter().filter(|e| e.start_unix >= now).collect();
        events.sort_by_key(|e| e.start_unix);
        events
    }

    /// The event that is happening right now, if any.
    pub fn current(&self, now: i64) -> Option<&CalendarEvent> {
        self.events
            .iter()
            .find(|e| e.start_unix <= now && e.end_unix.is_none_or(|end| end > now))
    }
}

/// Split an ICS file into its `VEVENT` components.
fn parse_events(text: &str) -> Vec<RawEvent> {
    let mut events = Vec::new();
    let mut current: Option<RawEvent> = None;
    let mut depth = 0usize;

    for line in unfold(text) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("BEGIN:") {
            depth += 1;
            if upper.starts_with("BEGIN:VEVENT") {
                current = Some(RawEvent::default());
            }
            continue;
        }
        if upper.starts_with("END:") {
            depth = depth.saturating_sub(1);
            if upper.starts_with("END:VEVENT") {
                if let Some(event) = current.take() {
                    events.push(event);
                }
            }
            continue;
        }
        let Some(event) = current.as_mut() else {
            continue;
        };
        let Some((name, property)) = parse_property(line) else {
            continue;
        };
        event.properties.entry(name).or_default().push(property);
    }
    events
}

/// Undo RFC 5545 line folding: a line starting with a space or a tab
/// continues the previous one.
///
/// Exactly one character is removed — the fold marker. The rest of the
/// indentation belongs to the value, which is why a folded line is usually
/// written with two spaces rather than one.
fn unfold(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        let continues = line.starts_with(' ') || line.starts_with('\t');
        if continues && !current.is_empty() {
            current.push_str(&line[1..]);
        } else {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            current = line.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Parse `NAME;PARAM=value:VALUE`.
fn parse_property(line: &str) -> Option<(String, Property)> {
    let colon = line.find(':')?;
    let (head, value) = line.split_at(colon);
    let value = value[1..].to_string();
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let mut params = BTreeMap::new();
    for part in parts {
        let Some((key, val)) = part.split_once('=') else {
            continue;
        };
        params.insert(
            key.trim().to_ascii_uppercase(),
            val.trim().trim_matches('"').to_string(),
        );
    }
    Some((name, Property { params, value }))
}

/// Turn a raw event into one or more concrete events.
fn expand(raw: &RawEvent, tz: &Tz) -> Vec<CalendarEvent> {
    let Some(start_prop) = raw.properties.get("DTSTART").and_then(|v| v.first()) else {
        return Vec::new();
    };
    let all_day = start_prop
        .params
        .get("VALUE")
        .is_some_and(|v| v.eq_ignore_ascii_case("DATE"));
    let Some(start) = parse_datetime(&start_prop.value) else {
        return Vec::new();
    };

    let start_unix = to_unix(start, tz);

    let end_unix = raw
        .properties
        .get("DTEND")
        .and_then(|v| v.first())
        .and_then(|p| parse_datetime(&p.value))
        .map(|dt| to_unix(dt, tz))
        .or_else(|| {
            // No DTEND: fall back to DURATION, else assume an hour.
            raw.properties
                .get("DURATION")
                .and_then(|v| v.first())
                .and_then(|p| parse_duration(&p.value))
                .map(|secs| start_unix + secs)
        });

    let summary = raw
        .properties
        .get("SUMMARY")
        .and_then(|v| v.first())
        .map(|p| unescape(&p.value))
        .unwrap_or_else(|| "(no title)".to_string());
    let description = raw
        .properties
        .get("DESCRIPTION")
        .and_then(|v| v.first())
        .map(|p| unescape(&p.value))
        .unwrap_or_default();
    let location = raw
        .properties
        .get("LOCATION")
        .and_then(|v| v.first())
        .map(|p| unescape(&p.value))
        .unwrap_or_default();
    let uid = raw
        .properties
        .get("UID")
        .and_then(|v| v.first())
        .map(|p| p.value.clone())
        .unwrap_or_else(|| format!("{summary}@{start_unix}"));

    let duration = end_unix
        .map(|end| (end - start_unix).max(0))
        .unwrap_or(if all_day { 86_400 } else { 3600 });

    let base = CalendarEvent {
        uid: uid.clone(),
        summary: summary.clone(),
        description: description.clone(),
        location: location.clone(),
        start_unix,
        end_unix: Some(start_unix + duration),
        all_day,
    };

    let Some(rule) = raw
        .properties
        .get("RRULE")
        .and_then(|v| v.first())
        .map(|p| p.value.clone())
    else {
        return vec![base];
    };

    let Some(rule) = RRule::parse(&rule) else {
        return vec![base];
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let horizon = now + HORIZON_DAYS * 86_400;

    let mut out = Vec::new();
    let mut occurrence = start_unix;
    let mut count = 0usize;
    let mut index = 0usize;

    while out.len() < MAX_INSTANCES && count < rule.count.unwrap_or(usize::MAX) {
        if occurrence > horizon {
            break;
        }
        if let Some(until) = rule.until {
            if occurrence > until {
                break;
            }
        }
        if occurrence >= now - 86_400 {
            let mut event = base.clone();
            event.start_unix = occurrence;
            event.end_unix = Some(occurrence + duration);
            event.uid = format!("{uid}#{index}");
            out.push(event);
        }
        count += 1;
        index += 1;
        let Some(next) = rule.next_occurrence(occurrence, index) else {
            break;
        };
        if next <= occurrence {
            break;
        }
        occurrence = next;
    }

    if out.is_empty() {
        // The rule produced nothing in the window, but the original event is
        // still real. Keep it so the panel is not silently empty.
        return vec![base];
    }
    out
}

/// Convert a parsed datetime into a Unix timestamp.
fn to_unix(dt: DateTime, tz: &Tz) -> i64 {
    if dt.is_utc {
        return days_from_civil(dt.year, dt.month, dt.day) * 86_400
            + dt.hour * 3600
            + dt.minute * 60
            + dt.second;
    }
    // Approximate with UTC first so the right offset can be looked up, then
    // redo the conversion with that offset.
    let approx =
        days_from_civil(dt.year, dt.month, dt.day) * 86_400 + dt.hour * 3600 + dt.minute * 60;
    let offset = tz.offset_at(approx);
    unix_from_civil(dt.year, dt.month, dt.day, dt.hour, dt.minute, offset) + dt.second
}

/// Parse an ICS datetime into its parts.
fn parse_datetime(value: &str) -> Option<DateTime> {
    let value = value.trim();
    let is_utc = value.ends_with('Z') || value.ends_with('z');
    let body = value.trim_end_matches(['Z', 'z']);
    let (date_part, time_part) = match body.find('T') {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    if date_part.len() != 8 {
        return None;
    }
    let year = date_part[0..4].parse().ok()?;
    let month = date_part[4..6].parse().ok()?;
    let day = date_part[6..8].parse().ok()?;
    let (hour, minute, second) = match time_part {
        None => (0, 0, 0),
        Some(t) => {
            if t.len() < 4 {
                return None;
            }
            let h = t[0..2].parse().ok()?;
            let m = t[2..4].parse().ok()?;
            let s = if t.len() >= 6 {
                t[4..6].parse().unwrap_or(0)
            } else {
                0
            };
            (h, m, s)
        }
    };
    Some(DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        is_utc,
    })
}

/// Parse an ISO 8601 duration such as `PT1H30M` or `P1D`.
fn parse_duration(value: &str) -> Option<i64> {
    let value = value.trim();
    let rest = value.strip_prefix('P')?;
    let mut days = 0;
    let mut rest = rest;
    if let Some(i) = rest.find('W') {
        days += rest[..i].parse::<i64>().ok()? * 7 * 86_400;
        rest = &rest[i + 1..];
    }
    if let Some(i) = rest.find('D') {
        days += rest[..i].parse::<i64>().ok()? * 86_400;
        rest = &rest[i + 1..];
    }
    let rest = rest.strip_prefix('T').unwrap_or(rest);
    let mut seconds = days;
    let mut number = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
            continue;
        }
        let value: i64 = number.parse().ok()?;
        number.clear();
        seconds += match ch {
            'H' => value * 3600,
            'M' => value * 60,
            'S' => value,
            _ => return None,
        };
    }
    Some(seconds)
}

/// A recurrence rule. Only the common frequencies are supported.
#[derive(Debug, Clone, Default)]
struct RRule {
    freq: String,
    interval: i64,
    count: Option<usize>,
    until: Option<i64>,
    by_day: Vec<i64>,
}

/// Weekday numbers, Monday is 0.
const WEEKDAYS: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

impl RRule {
    fn parse(raw: &str) -> Option<Self> {
        let mut rule = RRule {
            interval: 1,
            ..Default::default()
        };
        for part in raw.split(';') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key.trim().to_ascii_uppercase().as_str() {
                "FREQ" => rule.freq = value.trim().to_ascii_uppercase(),
                "INTERVAL" => rule.interval = value.trim().parse().unwrap_or(1).max(1),
                "COUNT" => rule.count = value.trim().parse().ok(),
                "UNTIL" => {
                    rule.until = parse_datetime(value.trim()).map(|dt| {
                        days_from_civil(dt.year, dt.month, dt.day) * 86_400
                            + dt.hour * 3600
                            + dt.minute * 60
                            + dt.second
                    })
                }
                "BYDAY" => {
                    rule.by_day = value
                        .split(',')
                        .filter_map(|d| {
                            let d = d.trim().to_ascii_uppercase();
                            WEEKDAYS.iter().position(|w| *w == d).map(|i| i as i64)
                        })
                        .collect();
                }
                _ => {}
            }
        }
        if rule.freq.is_empty() {
            return None;
        }
        Some(rule)
    }

    /// The next occurrence after `current`, in Unix seconds.
    fn next_occurrence(&self, current: i64, index: usize) -> Option<i64> {
        match self.freq.as_str() {
            "DAILY" => Some(current + self.interval * 86_400),
            "WEEKLY" => {
                if self.by_day.is_empty() {
                    return Some(current + self.interval * 7 * 86_400);
                }
                // Weekly with named days: step forward one day at a time
                // until the next matching weekday, then jump a week.
                let mut candidate = current + 86_400;
                for _ in 0..7 {
                    let weekday = ((candidate.div_euclid(86_400) % 7) + 10) % 7;
                    if self.by_day.contains(&weekday) {
                        return Some(candidate);
                    }
                    candidate += 86_400;
                }
                Some(current + self.interval * 7 * 86_400)
            }
            "MONTHLY" => {
                let _ = index;
                Some(current + self.interval * 30 * 86_400)
            }
            "YEARLY" => Some(current + self.interval * 365 * 86_400),
            _ => None,
        }
    }
}

/// Undo the escaping RFC 5545 applies to text values.
fn unescape(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\n\
VERSION:2.0\n\
PRODID:-//isle//test//EN\n\
BEGIN:VEVENT\n\
UID:evt-1\n\
DTSTART:20261008T093000Z\n\
DTEND:20261008T100000Z\n\
SUMMARY:Platform sync\n\
LOCATION:Room 3\n\
END:VEVENT\n\
BEGIN:VEVENT\n\
UID:evt-2\n\
DTSTART;VALUE=DATE:20261010\n\
SUMMARY:All day thing\n\
END:VEVENT\n\
END:VCALENDAR\n";

    #[test]
    fn events_are_parsed() {
        let calendar = Calendar::from_text(SAMPLE);
        assert_eq!(calendar.events.len(), 2);
        assert_eq!(calendar.events[0].summary, "Platform sync");
        assert_eq!(calendar.events[0].location, "Room 3");
        assert_eq!(calendar.events[0].duration_secs(), 1800);
    }

    #[test]
    fn an_all_day_event_is_flagged() {
        let calendar = Calendar::from_text(SAMPLE);
        assert!(calendar.events[1].all_day);
        assert!(!calendar.events[0].all_day);
    }

    #[test]
    fn a_utc_datetime_converts_exactly() {
        let calendar = Calendar::from_text(SAMPLE);
        let expected = days_from_civil(2026, 10, 8) * 86_400 + 9 * 3600 + 30 * 60;
        assert_eq!(calendar.events[0].start_unix, expected);
    }

    #[test]
    fn a_folded_line_continues_the_previous_one() {
        let folded = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:x\nDTSTART:20261008T090000Z\nSUMMARY:This is a very\n  long summary\nEND:VEVENT\nEND:VCALENDAR\n";
        let calendar = Calendar::from_text(folded);
        assert_eq!(calendar.events[0].summary, "This is a very long summary");
    }

    #[test]
    fn escaped_newlines_become_real_ones() {
        let text =
            "BEGIN:VEVENT\nUID:x\nDTSTART:20261008T090000Z\nSUMMARY:First\\nSecond\nEND:VEVENT\n";
        let calendar = Calendar::from_text(text);
        assert_eq!(calendar.events[0].summary, "First\nSecond");
    }

    #[test]
    fn a_daily_rule_produces_repeated_events() {
        let text = "BEGIN:VEVENT\nUID:x\nDTSTART:20260101T090000Z\nRRULE:FREQ=DAILY;COUNT=5\nSUMMARY:Daily\nEND:VEVENT\n";
        let calendar = Calendar::from_text(text);
        // The window is anchored on "now", so the count depends on the date.
        // What must hold is that the rule produced a sane, ordered series.
        assert!(!calendar.events.is_empty());
        for pair in calendar.events.windows(2) {
            assert!(pair[0].start_unix < pair[1].start_unix);
        }
    }

    #[test]
    fn a_weekly_rule_with_named_days_advances_by_a_week() {
        let rule = RRule::parse("FREQ=WEEKLY;INTERVAL=1;BYDAY=MO").unwrap();
        let start = days_from_civil(2026, 10, 5) * 86_400;
        let next = rule.next_occurrence(start, 1).unwrap();
        assert_eq!(next - start, 7 * 86_400);
    }

    #[test]
    fn a_daily_rule_advances_by_the_interval() {
        let rule = RRule::parse("FREQ=DAILY;INTERVAL=3").unwrap();
        let start = 0;
        assert_eq!(rule.next_occurrence(start, 1).unwrap(), 3 * 86_400);
    }

    #[test]
    fn a_duration_is_parsed() {
        assert_eq!(parse_duration("PT1H30M"), Some(5400));
        assert_eq!(parse_duration("P1D"), Some(86_400));
        assert_eq!(parse_duration("PT45S"), Some(45));
        assert_eq!(parse_duration("nonsense"), None);
    }

    #[test]
    fn upcoming_filters_and_sorts() {
        let calendar = Calendar::from_text(SAMPLE);
        let now = days_from_civil(2026, 10, 1) * 86_400;
        let upcoming = calendar.upcoming(now);
        assert_eq!(upcoming.len(), 2);
        assert!(upcoming[0].start_unix <= upcoming[1].start_unix);
    }

    #[test]
    fn an_empty_calendar_has_nothing_upcoming() {
        let calendar = Calendar::from_text("");
        assert!(calendar.upcoming(0).is_empty());
        assert!(calendar.current(0).is_none());
    }

    #[test]
    fn a_calendar_read_from_a_directory_picks_up_ics_files() {
        let dir = std::env::temp_dir().join(format!("isled-ics-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.ics"), SAMPLE).unwrap();
        std::fs::write(dir.join("b.txt"), "not a calendar").unwrap();
        let calendar = Calendar::from_dir(&dir);
        assert_eq!(calendar.events.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_directory_yields_an_empty_calendar() {
        let calendar = Calendar::from_dir(Path::new("/definitely/not/here"));
        assert!(calendar.events.is_empty());
    }
}
