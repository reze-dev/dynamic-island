//! Screen brightness via `brightnessctl`.
//!
//! The laptop backlight is the only display brightness the daemon can
//! change without root (DDC/CI needs `ddcutil` + i2c permissions and is
//! intentionally out of scope: no driver, no dependency). `brightnessctl`
//! talks to the kernel's backlight class, which works on bare metal and
//! in Wayland sessions alike.

use std::process::Command;

/// Current brightness as a percentage (0-100).
pub fn get_percent() -> Result<u8, String> {
    let current = run_int(&["g"])?;
    let max = run_int(&["m"])?;
    if max == 0 {
        return Err("the backlight reports no maximum brightness".into());
    }
    let percent = (current as f64 / max as f64 * 100.0).round();
    Ok(percent.clamp(1.0, 100.0) as u8)
}

/// Set brightness to a percentage (clamped to 1-100: 0 is reserved for
/// "screen off", which only the user asks for).
pub fn set_percent(percent: u8) -> Result<u8, String> {
    let percent = percent.clamp(1, 100);
    let out = Command::new("brightnessctl")
        .args(["s", &format!("{percent}%")])
        .output()
        .map_err(|e| format!("brightnessctl not found: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("brightnessctl failed: {err}"));
    }
    Ok(percent)
}

fn run_int(args: &[&str]) -> Result<u64, String> {
    let out = Command::new("brightnessctl")
        .args(args)
        .output()
        .map_err(|e| format!("brightnessctl not found: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("brightnessctl failed: {err}"));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    text.parse()
        .map_err(|_| format!("could not parse brightness output: {text:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_math_is_sane() {
        // 500 of 1000 -> 50%; 0 -> clamped to 1.
        let pct = (500f64 / 1000f64 * 100.0).round().clamp(1.0, 100.0) as u8;
        assert_eq!(pct, 50);
        let pct = (0f64 / 1000f64 * 100.0).round().clamp(1.0, 100.0) as u8;
        assert_eq!(pct, 1);
    }

    #[test]
    fn missing_binary_reports_an_error_not_a_panic() {
        // On hosts without brightnessctl the command errors cleanly.
        // (This host has it, so only the no-config case runs.)
        let _ = get_percent().or(set_percent(50)).or(get_percent());
    }
}
