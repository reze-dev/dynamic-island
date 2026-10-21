//! The pomodoro timer.
//!
//! A pomodoro is a work phase followed by a break, and a break every fourth
//! round is long. The state machine is deliberately tiny: the phase, whether
//! it is running, and when the current run started. Everything else is
//! derived, which means the timer survives a daemon restart — the wall clock
//! is the only thing that has to be right.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimerPhase {
    /// Not started.
    Idle,
    /// A focus round.
    Work,
    /// The short break between rounds.
    ShortBreak,
    /// The long break after every fourth round.
    LongBreak,
}

impl TimerPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Work => "focus",
            Self::ShortBreak => "short break",
            Self::LongBreak => "long break",
        }
    }

    /// True for the phases that are not work.
    pub const fn is_break(self) -> bool {
        matches!(self, Self::ShortBreak | Self::LongBreak)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerState {
    pub phase: TimerPhase,
    #[serde(default)]
    pub running: bool,
    /// When the current unbroken run started, in milliseconds since the
    /// epoch. `None` while paused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    /// Time already banked from earlier runs of this phase.
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default = "default_work")]
    pub work_minutes: u32,
    #[serde(default = "default_short_break")]
    pub short_break_minutes: u32,
    #[serde(default = "default_long_break")]
    pub long_break_minutes: u32,
    #[serde(default)]
    pub rounds_completed: u32,
    /// The moment the phase last changed, in seconds since the epoch.
    #[serde(default)]
    pub changed_at_unix: u64,
}

fn default_work() -> u32 {
    25
}

fn default_short_break() -> u32 {
    5
}

fn default_long_break() -> u32 {
    15
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            phase: TimerPhase::Idle,
            running: false,
            started_at_ms: None,
            elapsed_ms: 0,
            work_minutes: default_work(),
            short_break_minutes: default_short_break(),
            long_break_minutes: default_long_break(),
            rounds_completed: 0,
            changed_at_unix: 0,
        }
    }
}

impl TimerState {
    /// The length of the current phase, in milliseconds.
    pub fn phase_ms(&self) -> u64 {
        match self.phase {
            TimerPhase::Idle => 0,
            TimerPhase::Work => u64::from(self.work_minutes) * 60_000,
            TimerPhase::ShortBreak => u64::from(self.short_break_minutes) * 60_000,
            TimerPhase::LongBreak => u64::from(self.long_break_minutes) * 60_000,
        }
    }

    /// How long the current phase has been running, including any banked time
    /// from before a pause.
    pub fn elapsed_total_ms(&self, now_ms: u64) -> u64 {
        let running = match self.started_at_ms {
            Some(started) if self.running => now_ms.saturating_sub(started),
            _ => 0,
        };
        self.elapsed_ms.saturating_add(running)
    }

    /// How long is left in the phase, or `None` when there is no phase
    /// running.
    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> {
        let total = self.phase_ms();
        if total == 0 {
            return None;
        }
        Some(total.saturating_sub(self.elapsed_total_ms(now_ms)))
    }

    /// How far through the phase we are, from 0 to 1.
    pub fn progress(&self, now_ms: u64) -> f32 {
        let total = self.phase_ms();
        if total == 0 {
            return 0.0;
        }
        (self.elapsed_total_ms(now_ms) as f32 / total as f32).clamp(0.0, 1.0)
    }

    /// Start, or restart, the current phase.
    pub fn start(&mut self, now_ms: u64, now_unix: u64) {
        if self.phase == TimerPhase::Idle {
            self.phase = TimerPhase::Work;
        }
        self.elapsed_ms = 0;
        self.started_at_ms = Some(now_ms);
        self.running = true;
        self.changed_at_unix = now_unix;
    }

    /// Bank the time so far and stop the clock.
    pub fn pause(&mut self, now_ms: u64) {
        if !self.running {
            return;
        }
        self.elapsed_ms = self.elapsed_total_ms(now_ms);
        self.started_at_ms = None;
        self.running = false;
    }

    /// Resume a paused phase from where it stopped.
    pub fn resume(&mut self, now_ms: u64) {
        if self.running || self.phase == TimerPhase::Idle {
            return;
        }
        self.started_at_ms = Some(now_ms);
        self.running = true;
    }

    /// Stop the timer and return it to idle.
    pub fn stop(&mut self) {
        self.phase = TimerPhase::Idle;
        self.running = false;
        self.started_at_ms = None;
        self.elapsed_ms = 0;
    }

    /// Move to the next phase if the current one has run out. Returns the new
    /// phase when it changed.
    pub fn advance_if_elapsed(&mut self, now_ms: u64, now_unix: u64) -> Option<TimerPhase> {
        if !self.running || self.phase == TimerPhase::Idle {
            return None;
        }
        let remaining = self.remaining_ms(now_ms)?;
        if remaining > 0 {
            return None;
        }
        self.phase = match self.phase {
            TimerPhase::Work => {
                self.rounds_completed = self.rounds_completed.saturating_add(1);
                if self.rounds_completed.is_multiple_of(4) {
                    TimerPhase::LongBreak
                } else {
                    TimerPhase::ShortBreak
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => TimerPhase::Work,
            TimerPhase::Idle => return None,
        };
        self.elapsed_ms = 0;
        self.started_at_ms = Some(now_ms);
        self.changed_at_unix = now_unix;
        Some(self.phase)
    }

    /// Change the phase lengths. Only sensible while the timer is idle.
    pub fn set_durations(&mut self, work: u32, short_break: u32, long_break: u32) {
        if work > 0 {
            self.work_minutes = work;
        }
        if short_break > 0 {
            self.short_break_minutes = short_break;
        }
        if long_break > 0 {
            self.long_break_minutes = long_break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn a_fresh_timer_is_idle_with_no_time_remaining() {
        let t = TimerState::default();
        assert_eq!(t.phase, TimerPhase::Idle);
        assert!(!t.running);
        assert_eq!(t.remaining_ms(0), None);
    }

    #[test]
    fn starting_begins_a_work_phase() {
        let mut t = TimerState::default();
        t.start(0, 0);
        assert_eq!(t.phase, TimerPhase::Work);
        assert!(t.running);
        assert_eq!(t.remaining_ms(0), Some(25 * MIN));
    }

    #[test]
    fn the_clock_counts_down() {
        let mut t = TimerState::default();
        t.start(0, 0);
        assert_eq!(t.remaining_ms(5 * MIN), Some(20 * MIN));
    }

    #[test]
    fn pausing_banks_the_time_and_resuming_continues_it() {
        let mut t = TimerState::default();
        t.start(0, 0);
        t.pause(5 * MIN);
        assert!(!t.running);
        assert_eq!(t.remaining_ms(100 * MIN), Some(20 * MIN));
        t.resume(100 * MIN);
        assert_eq!(t.remaining_ms(105 * MIN), Some(15 * MIN));
    }

    #[test]
    fn a_work_phase_advances_to_a_short_break() {
        let mut t = TimerState::default();
        t.start(0, 0);
        let phase = t.advance_if_elapsed(25 * MIN, 0);
        assert_eq!(phase, Some(TimerPhase::ShortBreak));
        assert_eq!(t.rounds_completed, 1);
        assert_eq!(t.remaining_ms(25 * MIN), Some(5 * MIN));
    }

    #[test]
    fn every_fourth_round_earns_a_long_break() {
        let mut t = TimerState::default();
        for _ in 0..3 {
            t.start(0, 0);
            t.advance_if_elapsed(25 * MIN, 0);
            t.advance_if_elapsed(30 * MIN, 0);
        }
        t.start(0, 0);
        assert_eq!(
            t.advance_if_elapsed(25 * MIN, 0),
            Some(TimerPhase::LongBreak)
        );
        assert_eq!(t.rounds_completed, 4);
    }

    #[test]
    fn a_break_advances_back_to_work() {
        let mut t = TimerState::default();
        t.start(0, 0);
        t.advance_if_elapsed(25 * MIN, 0);
        assert_eq!(t.advance_if_elapsed(30 * MIN, 0), Some(TimerPhase::Work));
    }

    #[test]
    fn a_phase_that_has_not_finished_does_not_advance() {
        let mut t = TimerState::default();
        t.start(0, 0);
        assert_eq!(t.advance_if_elapsed(24 * MIN, 0), None);
        assert_eq!(t.phase, TimerPhase::Work);
    }

    #[test]
    fn a_paused_timer_does_not_advance() {
        let mut t = TimerState::default();
        t.start(0, 0);
        t.pause(0);
        assert_eq!(t.advance_if_elapsed(60 * MIN, 0), None);
    }

    #[test]
    fn stopping_returns_the_timer_to_idle() {
        let mut t = TimerState::default();
        t.start(0, 0);
        t.stop();
        assert_eq!(t.phase, TimerPhase::Idle);
        assert_eq!(t.elapsed_ms, 0);
    }

    #[test]
    fn progress_runs_from_zero_to_one() {
        let mut t = TimerState::default();
        t.start(0, 0);
        assert_eq!(t.progress(0), 0.0);
        assert!((t.progress(12 * MIN + 30_000) - 0.5).abs() < 0.001);
        assert_eq!(t.progress(25 * MIN), 1.0);
    }

    #[test]
    fn durations_can_be_changed() {
        let mut t = TimerState::default();
        t.set_durations(50, 10, 20);
        assert_eq!(t.work_minutes, 50);
        assert_eq!(t.short_break_minutes, 10);
        assert_eq!(t.long_break_minutes, 20);
        t.set_durations(0, 0, 0);
        assert_eq!(t.work_minutes, 50);
    }

    #[test]
    fn the_timer_survives_a_round_trip_through_json() {
        let mut timer = TimerState::default();
        timer.start(1_000, 5);
        timer.set_durations(30, 5, 15);
        let json = serde_json::to_string(&timer).unwrap();
        let back: TimerState = serde_json::from_str(&json).unwrap();
        assert!(back.running);
        assert_eq!(back.work_minutes, 30);
        assert_eq!(back.started_at_ms, Some(1_000));
    }
}
