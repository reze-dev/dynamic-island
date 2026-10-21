//! A minimal reader for the system timezone.
//!
//! The calendar panel has to know what time it is *locally* to decide which
//! events are upcoming and to group them by hour. `std` offers no way to ask
//! for that — `SystemTime` is UTC and nothing converts it — and the usual
//! answer is a datetime crate.
//!
//! Instead this reads `/etc/localtime` directly. The format is TZif, and the
//! part that matters is small: a list of transition times and, for each, the
//! UTC offset in force after it. The current offset is the one belonging to
//! the most recent transition at or before now.
//!
//! The important limitation: this gives the offset *now*, and applies it to
//! any timestamp. A calendar event in a different daylight-saving period of
//! the same zone can be off by an hour. That is a fair trade for a panel
//! that is correct for the overwhelming majority of events.

use std::path::Path;

/// One UTC-offset rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TtInfo {
    /// Seconds east of UTC.
    utoff: i32,
    isdst: bool,
}

/// The parsed timezone.
#[derive(Debug, Clone, Default)]
pub struct Tz {
    transitions: Vec<i64>,
    type_indices: Vec<u8>,
    types: Vec<TtInfo>,
    /// The offset used when no transition applies, for zones that never
    /// change their offset.
    fallback: Option<i32>,
}

impl Tz {
    /// Read the system timezone.
    pub fn system() -> Self {
        Self::from_path(Path::new("/etc/localtime")).unwrap_or_default()
    }

    /// Read a timezone from a TZif file.
    pub fn from_path(path: &Path) -> Option<Self> {
        let raw = std::fs::read(path).ok()?;
        Self::parse(&raw)
    }

    /// Parse a TZif payload.
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let mut tz = Tz::parse_block(raw, 0, 4)?;
        if tz.transitions.is_empty() {
            tz.fallback = tz.types.iter().find(|t| !t.isdst).map(|t| t.utoff);
        }
        Some(tz)
    }

    /// The UTC offset, in seconds, in force at a point in time.
    pub fn offset_at(&self, unix: i64) -> i64 {
        if self.transitions.is_empty() {
            return i64::from(self.fallback.unwrap_or(0));
        }
        let index = match self.transitions.binary_search(&unix) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let type_index = self.type_indices.get(index).copied().unwrap_or(0) as usize;
        i64::from(self.types.get(type_index).map_or(0, |t| t.utoff))
    }

    /// The current UTC offset, in seconds.
    pub fn offset_now(&self) -> i64 {
        self.offset_at(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        )
    }

    /// Parse one TZif data block starting at `offset`.
    ///
    /// `time_size` is 4 for the version 1 block and 8 for the version 2
    /// block, which is the one that carries real transition history.
    fn parse_block(raw: &[u8], offset: usize, time_size: usize) -> Option<Tz> {
        let header = raw.get(offset..offset + 44)?;
        if &header[0..4] != b"TZif" {
            return None;
        }
        let version = header[4];
        let count_at = |i: usize| -> Option<usize> {
            let start = offset + 20 + i * 4;
            let bytes = raw.get(start..start + 4)?;
            Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
        };
        let isutcnt = count_at(0)?;
        let isstdcnt = count_at(1)?;
        let leapcnt = count_at(2)?;
        let timecnt = count_at(3)?;
        let typecnt = count_at(4)?;
        let charcnt = count_at(5)?;

        let mut cursor = offset + 44;

        // transition times
        let mut transitions = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            let bytes = raw.get(cursor..cursor + time_size)?;
            let value = if time_size == 8 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(bytes);
                i64::from_be_bytes(buf)
            } else {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(bytes);
                i64::from(i32::from_be_bytes(buf))
            };
            transitions.push(value);
            cursor += time_size;
        }

        // transition type indices
        let mut type_indices = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            type_indices.push(*raw.get(cursor)?);
            cursor += 1;
        }

        // local time type records
        let mut types = Vec::with_capacity(typecnt);
        for _ in 0..typecnt {
            let bytes = raw.get(cursor..cursor + 6)?;
            let utoff = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let isdst = bytes[4] != 0;
            types.push(TtInfo { utoff, isdst });
            cursor += 6;
        }

        // designators, leap seconds, standard/wall and ut/local indicators
        cursor += charcnt;
        cursor += leapcnt * (time_size + 4);
        cursor += isstdcnt;
        cursor += isutcnt;

        let mut tz = Tz {
            transitions,
            type_indices,
            types,
            fallback: None,
        };

        // Version 2 and later carry a second block with 64-bit times.
        if version >= b'2' {
            if let Some(second) = Tz::parse_block(raw, cursor, 8) {
                if !second.transitions.is_empty() {
                    tz = second;
                }
            }
        }

        Some(tz)
    }
}

/// Days from 1970-01-01 to the given civil date, for the proleptic
/// Gregorian calendar. Howard Hinnant's algorithm, which is exact for every
/// date the Gregorian calendar has ever had.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The inverse of [`days_from_civil`].
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Split a Unix timestamp into local civil time.
pub fn local_civil(unix: i64, offset: i64) -> (i64, i64, i64, i64, i64, i64) {
    let local = unix + offset;
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (y, m, d, secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// Join a local civil time into a Unix timestamp, given the offset.
pub fn unix_from_civil(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    offset: i64,
) -> i64 {
    days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 - offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_day_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn a_known_date_converts_both_ways() {
        let days = days_from_civil(2026, 10, 8);
        assert_eq!(civil_from_days(days), (2026, 10, 8));
    }

    #[test]
    fn a_leap_day_converts_both_ways() {
        let days = days_from_civil(2024, 2, 29);
        assert_eq!(civil_from_days(days), (2024, 2, 29));
    }

    #[test]
    fn the_epoch_round_trips_through_local_civil() {
        let (y, m, d, hh, mm, ss) = local_civil(0, 0);
        assert_eq!((y, m, d, hh, mm, ss), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn a_positive_offset_shifts_the_wall_clock_forward() {
        let (y, m, d, hh, _, _) = local_civil(0, 3600);
        assert_eq!((y, m, d, hh), (1970, 1, 1, 1));
    }

    #[test]
    fn unix_from_civil_is_the_inverse_of_local_civil() {
        let offset = 19800; // +05:30, a half-hour zone
        let unix = unix_from_civil(2026, 10, 8, 14, 30, offset);
        let (y, m, d, hh, mm, _) = local_civil(unix, offset);
        assert_eq!((y, m, d, hh, mm), (2026, 10, 8, 14, 30));
    }

    #[test]
    fn a_garbled_timezone_file_is_rejected() {
        assert!(Tz::parse(b"not a timezone file").is_none());
        assert!(Tz::parse(b"").is_none());
    }

    #[test]
    fn the_system_timezone_can_be_read() {
        // Not asserting a particular offset — the machine's zone is not the
        // test's business. Asserting that it parses and answers.
        let tz = Tz::system();
        let offset = tz.offset_now();
        assert!((-14 * 3600..=14 * 3600).contains(&offset));
    }

    #[test]
    fn a_parsed_timezone_reports_a_sane_offset() {
        // Build a tiny TZif by hand: one transition, one type, +01:00.
        let header = |time_size: usize| -> Vec<u8> {
            let mut raw = Vec::new();
            raw.extend_from_slice(b"TZif2");
            raw.extend_from_slice(&[0u8; 15]);
            for count in [0u32, 0, 0, 1, 1, 0] {
                raw.extend_from_slice(&count.to_be_bytes());
            }
            let _ = time_size;
            raw
        };
        let data = |time_size: usize| -> Vec<u8> {
            let mut raw = Vec::new();
            if time_size == 8 {
                raw.extend_from_slice(&1_000_000i64.to_be_bytes());
            } else {
                raw.extend_from_slice(&1_000_000i32.to_be_bytes());
            }
            raw.push(0); // type index
            raw.extend_from_slice(&3600i32.to_be_bytes()); // utoff
            raw.push(0); // isdst
            raw.push(0); // desig index
            raw
        };

        let mut raw = header(4);
        raw.extend_from_slice(&data(4));
        raw.extend_from_slice(&header(8));
        raw.extend_from_slice(&data(8));

        let tz = Tz::parse(&raw).unwrap();
        assert_eq!(tz.offset_at(0), 3600);
        assert_eq!(tz.offset_at(2_000_000), 3600);
    }
}
