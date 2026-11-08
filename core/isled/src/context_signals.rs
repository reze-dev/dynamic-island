//! D-Bus signal sources for the context engine.
//!
//! Each source is a small, failure-tolerant reader: when the bus, the
//! service, or the property is missing, the source returns `None` and the
//! engine keeps working. Nothing here ever panics on a missing daemon.
//!
//! * **logind** — `org.freedesktop.login1` `IdleHint` + `SessionNew` /
//!   `SessionRemoved`. Used for Away/Idle transitions.
//! * **UPower** — `org.freedesktop.UPower` `Device::Percentage` /
//!   `State` (1 = charging, 2 = discharging). Used for LowBattery.
//! * **NetworkManager** — `org.freedesktop.NetworkManager`
//!   `NetworkingEnabled` / `PrimaryConnectionState`. Used for Network
//!   degraded/recovered. (SSID-level detail comes from the panel via
//!   Quickshell; the engine only needs degraded/good.)
//! * **PipeWire nodes** — meeting/presentation detection is compositor
//!   window-title heuristics plus a best-effort `pw-cli` node scan for
//!   stream names containing meeting apps. No video is read.

use isle_core::{Context, Signal};
use zbus::zvariant::OwnedValue;

/// Fold one D-Bus source update into the engine. Returns the signal and
/// whether it changed the context.
pub fn fold_logind(
    ctx: &mut Context,
    idle: Option<bool>,
    session_present: Option<bool>,
    now: u64,
) -> bool {
    let mut changed = false;
    if let Some(idle) = idle {
        changed |= ctx.fold(&Signal::Idle { idle }, now);
    }
    if let Some(present) = session_present {
        changed |= ctx.fold(&Signal::Session { present }, now);
    }
    changed
}

pub fn fold_battery(
    ctx: &mut Context,
    percent: Option<u8>,
    charging: Option<bool>,
    now: u64,
) -> bool {
    let percent = percent.unwrap_or(100);
    let charging = charging.unwrap_or(false);
    ctx.fold(&Signal::Battery { percent, charging }, now)
}

pub fn fold_network(ctx: &mut Context, degraded: Option<bool>, now: u64) -> bool {
    ctx.fold(
        &Signal::Network {
            degraded: degraded.unwrap_or(false),
        },
        now,
    )
}

pub fn fold_fullscreen(ctx: &mut Context, active: bool, now: u64) -> bool {
    ctx.fold(&Signal::Fullscreen { active }, now)
}

pub fn fold_meeting(ctx: &mut Context, title: &str, app_id: &str, now: u64) -> bool {
    if isle_core::context::looks_like_meeting(title, app_id) {
        ctx.fold(
            &Signal::Meeting {
                title: format!("{title} ({app_id})"),
            },
            now,
        )
    } else {
        false
    }
}

pub fn fold_calendar(ctx: &mut Context, title: &str, seconds: i64, now: u64) -> bool {
    ctx.fold(
        &Signal::CalendarSoon {
            title: title.into(),
            seconds,
        },
        now,
    )
}

pub fn fold_todo(ctx: &mut Context, text: &str, now: u64) -> bool {
    ctx.fold(&Signal::TodoDue { text: text.into() }, now)
}

// ------------------------------------------------------------- logind

/// Read `IdleHint` through the bus. `None` when logind is unreachable.
/// Best-effort: the subprocess fallback covers hosts where the session
/// proxy path is less predictable.
pub async fn read_logind_idle() -> Option<bool> {
    let connection = zbus::Connection::session().await.ok()?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/self",
        "org.freedesktop.DBus.Properties",
    )
    .await
    .ok()?;
    let value: OwnedValue = proxy
        .call("Get", &("org.freedesktop.login1.Session", "IdleHint"))
        .await
        .ok()?;
    bool::try_from(value).ok()
}

/// Subprocess fallback for hosts without a Z-Bus session yet (tests).
pub fn read_logind_idle_fallback() -> Option<bool> {
    let output = std::process::Command::new("loginctl")
        .args(["show-session", "self", "-p", "IdleHint"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if text.contains("IdleHint=yes") {
        Some(true)
    } else if text.contains("IdleHint=no") {
        Some(false)
    } else {
        None
    }
}

// -------------------------------------------------------------- UPower

pub async fn read_upower_battery() -> Option<(u8, bool)> {
    let connection = zbus::Connection::session().await.ok()?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT0",
        "org.freedesktop.DBus.Properties",
    )
    .await
    .ok()?;
    let pct: OwnedValue = proxy
        .call("Get", &("org.freedesktop.UPower.Device", "Percentage"))
        .await
        .ok()?;
    let raw_percent = f64::try_from(pct).ok()?;
    let percent = (raw_percent.clamp(0.0, 1.0) * 100.0) as u8;
    let state_value: OwnedValue = proxy
        .call("Get", &("org.freedesktop.UPower.Device", "State"))
        .await
        .ok()?;
    let state = u32::try_from(state_value).unwrap_or(0);
    let charging = state == 1;
    Some((percent, charging))
}

/// Subprocess fallback: `upower -i /org/freedesktop/UPower/devices/battery_BAT0`.
pub fn read_upower_battery_fallback() -> Option<(u8, bool)> {
    let output = std::process::Command::new("upower")
        .args(["-i", "/org/freedesktop/UPower/devices/battery_BAT0"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let percent = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("percentage:"))
        .and_then(|v| v.trim().trim_end_matches('%').trim().parse::<f64>().ok())
        .map(|v| v.clamp(0.0, 100.0) as u8)?;
    let charging = text.contains("state: charging") || text.contains("charging");
    Some((percent, charging))
}

// -------------------------------------------------- NetworkManager

pub async fn read_nm_degraded() -> Option<bool> {
    let connection = zbus::Connection::session().await.ok()?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.DBus.Properties",
    )
    .await
    .ok()?;
    let networking: OwnedValue = proxy
        .call(
            "Get",
            &("org.freedesktop.NetworkManager", "NetworkingEnabled"),
        )
        .await
        .ok()?;
    let enabled = bool::try_from(networking).unwrap_or(false);
    if !enabled {
        return Some(true);
    }
    let state_value: OwnedValue = proxy
        .call("Get", &("org.freedesktop.NetworkManager", "State"))
        .await
        .ok()?;
    let state = u32::try_from(state_value).unwrap_or(0);
    // NM state: 0 unknown, 10 connected, 20 connecting, 30 connected (local),
    // 40 disconnected. Treat 0/20/40 as degraded.
    Some(matches!(state, 0 | 20 | 40))
}

/// Subprocess fallback through `nmcli`.
pub fn read_nm_degraded_fallback() -> Option<bool> {
    let output = std::process::Command::new("nmcli")
        .args(["general", "status"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    Some(text.contains("disconnected") || text.contains("connecting"))
}

// ------------------------------------------------------------- PipeWire

/// Best-effort screencast/meeting node scan. Returns `true` when a
/// meeting-looking stream name is present.
pub fn pipewire_has_meeting_stream() -> bool {
    let output = std::process::Command::new("pw-cli")
        .args(["ls", "Node"])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    ["zoom", "meet", "teams", "webex", "discord"]
        .iter()
        .any(|k| text.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_is_a_noop_when_the_source_is_missing() {
        // None of these should panic or assert.
        let mut ctx = Context::default();
        assert!(!fold_logind(&mut ctx, None, None, 1));
        assert_eq!(ctx.state, isle_core::ContextState::Idle);
    }

    #[test]
    fn a_low_battery_folds_into_the_context() {
        let mut ctx = Context::default();
        assert!(fold_battery(&mut ctx, Some(10), Some(false), 1));
        assert_eq!(ctx.state, isle_core::ContextState::LowBattery);
    }

    #[test]
    fn a_fullscreen_fold_scores() {
        let mut ctx = Context::default();
        fold_fullscreen(&mut ctx, true, 1);
        assert_eq!(ctx.state, isle_core::ContextState::Fullscreen);
    }

    #[test]
    fn the_fallback_readers_never_panic() {
        assert!(read_logind_idle_fallback().is_none() || read_logind_idle_fallback().is_some());
        assert!(!pipewire_has_meeting_stream() || pipewire_has_meeting_stream());
    }
}
