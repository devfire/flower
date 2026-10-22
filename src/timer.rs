//! Pure timer bookkeeping: one speech timer plus a separate 3:00 prep
//! balance per team (PF prep is a whole-round budget, not per-use).
//!
//! The bank holds all three remainders at once. The main timer always
//! shows the speech countdown; each prep chip shows its own balance and
//! ticks it down in place, so starting prep never swaps the display or
//! needs a "back to speech" step. Only the explicit per-team reset
//! restores a prep balance to 3:00.

use crate::model::Side;

/// Each team's prep budget for the whole round, in seconds.
pub const PREP_SECS: i32 = 3 * 60;

/// Format a countdown as `M:SS`. Negative input (an overrun timer)
/// displays as `0:00`, never a negative minute.
#[must_use]
pub fn fmt_time(secs: i32) -> String {
    let s = secs.max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

/// Whole seconds to display for a millisecond remainder, rounded up so
/// a partially elapsed second still counts. Non-positive (and NaN, via
/// the saturating cast below) remainders display as 0; remainders past
/// `i32::MAX` seconds saturate there instead of wrapping.
#[must_use]
// The cast is safe by construction: `secs` is clamped to
// `0..=i32::MAX` above it, and Rust float-to-int `as` saturates (NaN
// becomes 0) rather than wrapping. f64 has no `TryFrom` conversion.
#[allow(clippy::cast_possible_truncation)]
pub fn ceil_secs(remaining_ms: f64) -> i32 {
    let secs = (remaining_ms / 1_000.0).ceil();
    if secs <= 0.0 {
        0
    } else if secs >= f64::from(i32::MAX) {
        i32::MAX
    } else {
        secs as i32
    }
}

/// Display value a running tick should publish, or `None` when it is
/// the value already on screen. Ticks fire 4×/second but the displayed
/// second changes only 1×/second; returning `None` lets the caller skip
/// the signal writes (and the per-tick persist) that would otherwise
/// notify subscribers with an identical value. Callers handle expiry
/// (`remaining_ms <= 0.0`) before calling this: expiry must run even
/// when the display already reads 0.
#[must_use]
pub fn tick_secs(remaining_ms: f64, displayed: i32) -> Option<i32> {
    let secs = ceil_secs(remaining_ms);
    (secs != displayed).then_some(secs)
}

/// Parallel countdown remainders — the shared `*_secs` suffix is the
/// point, so the pedantic field-names lint is a false positive here.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimerBank {
    speech_secs: i32,
    pro_prep_secs: i32,
    con_prep_secs: i32,
}

impl TimerBank {
    /// Rebuild a bank from saved remainders (issue #17), clamped to
    /// `0..=PREP_SECS` for prep: a hand-edited or stale save can never
    /// resurrect more prep than a team starts with, or a negative
    /// speech remainder.
    #[must_use]
    pub fn restored(speech_secs: i32, pro_prep_secs: i32, con_prep_secs: i32) -> Self {
        Self {
            speech_secs: speech_secs.max(0),
            pro_prep_secs: pro_prep_secs.clamp(0, PREP_SECS),
            con_prep_secs: con_prep_secs.clamp(0, PREP_SECS),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn speech_secs(self) -> i32 {
        self.speech_secs
    }

    #[must_use]
    pub fn prep_secs(self, side: Side) -> i32 {
        match side {
            Side::Pro => self.pro_prep_secs,
            Side::Con => self.con_prep_secs,
        }
    }

    /// Bank the live speech display value. Call whenever the speech
    /// timer freezes or ticks.
    pub fn bank_speech(&mut self, displayed: i32) {
        self.speech_secs = displayed.max(0);
    }

    /// Start a fresh speech at `full_secs`, discarding the old speech
    /// remainder (advancing a speech is meant to do that). Prep balances
    /// are untouched.
    pub fn start_speech(&mut self, full_secs: i32) -> i32 {
        self.speech_secs = full_secs.max(0);
        self.speech_secs
    }

    /// Restore one team's prep balance to the full 3:00 — the only way a
    /// prep balance ever goes back up. The speech display is never
    /// touched here: callers keep showing the speech remainder.
    pub fn reset_prep(&mut self, side: Side) {
        match side {
            Side::Pro => self.pro_prep_secs = PREP_SECS,
            Side::Con => self.con_prep_secs = PREP_SECS,
        }
    }

    /// Write one team's prep balance directly. The prep chip's own
    /// countdown ticks this down while the speech timer stays on screen.
    pub fn set_prep(&mut self, side: Side, secs: i32) {
        let secs = secs.clamp(0, PREP_SECS);
        match side {
            Side::Pro => self.pro_prep_secs = secs,
            Side::Con => self.con_prep_secs = secs,
        }
    }
    /// Restore both prep balances (round replacement only).
    pub fn reset_all_prep(&mut self) {
        self.pro_prep_secs = PREP_SECS;
        self.con_prep_secs = PREP_SECS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_time_pads_seconds_and_rolls_minutes() {
        assert_eq!(fmt_time(0), "0:00");
        assert_eq!(fmt_time(9), "0:09");
        assert_eq!(fmt_time(59), "0:59");
        assert_eq!(fmt_time(60), "1:00");
        assert_eq!(fmt_time(61), "1:01");
        assert_eq!(fmt_time(240), "4:00");
        assert_eq!(fmt_time(PREP_SECS), "3:00");
        assert_eq!(fmt_time(600), "10:00");
    }

    #[test]
    fn ceil_secs_rounds_up_and_clamps() {
        assert_eq!(ceil_secs(0.0), 0);
        assert_eq!(ceil_secs(-1_500.0), 0);
        assert_eq!(ceil_secs(f64::NAN), 0);
        assert_eq!(ceil_secs(1.0), 1);
        assert_eq!(ceil_secs(1_000.0), 1);
        assert_eq!(ceil_secs(1_001.0), 2);
        assert_eq!(ceil_secs(59_500.0), 60);
        assert_eq!(ceil_secs(f64::INFINITY), i32::MAX);
        assert_eq!(ceil_secs(f64::from(i32::MAX) * 1_000.0), i32::MAX);
    }

    #[test]
    fn tick_secs_suppresses_ticks_within_the_displayed_second() {
        // 250 ms ticks inside the same displayed second publish nothing.
        assert_eq!(tick_secs(60_000.0, 60), None);
        assert_eq!(tick_secs(59_750.0, 60), None);
        assert_eq!(tick_secs(59_500.0, 60), None);
        assert_eq!(tick_secs(59_250.0, 60), None);
        // Crossing into the next second publishes exactly that second.
        assert_eq!(tick_secs(59_000.0, 60), Some(59));
        assert_eq!(tick_secs(58_999.0, 60), Some(59));
        assert_eq!(tick_secs(1_000.0, 2), Some(1));
        assert_eq!(tick_secs(1.0, 1), None);
        // A stale display (pause/resume, clock jump) still republishes.
        assert_eq!(tick_secs(59_500.0, 42), Some(60));
    }

    #[test]
    fn fmt_time_clamps_overrun_to_zero() {
        assert_eq!(fmt_time(-1), "0:00");
        assert_eq!(fmt_time(-90), "0:00");
        assert_eq!(fmt_time(i32::MIN), "0:00");
    }

    #[test]
    fn used_prep_keeps_remaining_while_speech_ticks() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        // Burn 1:00 of Pro prep, then tick the speech timer: both
        // balances coexist, prep is not a display mode anymore.
        bank.set_prep(Side::Pro, 120);
        bank.bank_speech(90);
        assert_eq!(bank.prep_secs(Side::Pro), 120);
        assert_eq!(bank.speech_secs(), 90);
    }

    #[test]
    fn pro_and_con_prep_balances_are_independent() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.set_prep(Side::Pro, 100);
        bank.set_prep(Side::Con, 45);
        assert_eq!(bank.prep_secs(Side::Pro), 100);
        assert_eq!(bank.prep_secs(Side::Con), 45);
    }

    #[test]
    fn prep_reset_is_per_team_and_never_touches_speech() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.set_prep(Side::Pro, 100);
        bank.bank_speech(50);
        bank.reset_prep(Side::Pro);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
        assert_eq!(
            bank.prep_secs(Side::Con),
            PREP_SECS,
            "Con balance must not move"
        );
        assert_eq!(bank.speech_secs(), 50);
    }

    #[test]
    fn resetting_prep_leaves_speech_balance_alone() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.bank_speech(42);
        bank.reset_prep(Side::Pro);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
        assert_eq!(bank.speech_secs(), 42);
    }

    #[test]
    fn expired_prep_stays_at_zero_until_explicitly_reset() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.set_prep(Side::Pro, 0);
        bank.bank_speech(90);
        assert_eq!(bank.prep_secs(Side::Pro), 0);
        bank.reset_prep(Side::Pro);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
    }

    #[test]
    fn new_speech_discards_speech_remainder_but_keeps_prep() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.set_prep(Side::Pro, 111);
        assert_eq!(bank.start_speech(180), 180);
        assert_eq!(bank.prep_secs(Side::Pro), 111);
    }

    #[test]
    fn banked_values_are_clamped_to_sane_ranges() {
        let bank = TimerBank::restored(-5, 999, -3);
        assert_eq!(bank.speech_secs(), 0);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
        assert_eq!(bank.prep_secs(Side::Con), 0);
    }
}
