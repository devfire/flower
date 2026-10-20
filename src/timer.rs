//! Pure timer bookkeeping: one speech timer plus a separate 3:00 prep
//! balance per team (PF prep is a whole-round budget, not per-use).
//!
//! The UI keeps a single displayed countdown; this bank remembers the
//! remainder of whichever timer is *not* on screen, so switching between
//! a speech and either team's prep never destroys another timer's state.
//! Only the explicit per-team reset restores a prep balance to 3:00.

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerKind {
    Speech,
    Prep(Side),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimerBank {
    active: TimerKind,
    speech_secs: i32,
    pro_prep_secs: i32,
    con_prep_secs: i32,
}

impl TimerBank {
    #[must_use]
    pub fn new(speech_secs: i32) -> Self {
        Self {
            active: TimerKind::Speech,
            speech_secs: speech_secs.max(0),
            pro_prep_secs: PREP_SECS,
            con_prep_secs: PREP_SECS,
        }
    }

    /// Rebuild a bank from saved remainders (issue #17), clamped to the
    /// same ranges `bank_displayed` enforces: a hand-edited or stale save
    /// can never resurrect more prep than a team starts with, or a
    /// negative speech remainder.
    #[must_use]
    pub fn restored(speech_secs: i32, pro_prep_secs: i32, con_prep_secs: i32, active: TimerKind) -> Self {
        let mut bank = Self::new(speech_secs);
        bank.active = active;
        bank.pro_prep_secs = pro_prep_secs.clamp(0, PREP_SECS);
        bank.con_prep_secs = con_prep_secs.clamp(0, PREP_SECS);
        bank
    }

    #[must_use]
    pub fn active(self) -> TimerKind {
        self.active
    }

    /// Seconds the display should show for `kind`, given the live display
    /// value `displayed` when `kind` is the active timer.
    #[must_use]
    pub fn secs_for(self, kind: TimerKind, displayed: i32) -> i32 {
        if kind == self.active {
            displayed.max(0)
        } else {
            match kind {
                TimerKind::Speech => self.speech_secs,
                TimerKind::Prep(side) => self.prep_secs(side),
            }
        }
    }

    #[must_use]
    pub fn prep_secs(self, side: Side) -> i32 {
        match side {
            Side::Pro => self.pro_prep_secs,
            Side::Con => self.con_prep_secs,
        }
    }

    /// Bank the live display value into the active timer's slot. Call
    /// whenever the display freezes or ticks, before switching away.
    pub fn bank_displayed(&mut self, displayed: i32) {
        match self.active {
            TimerKind::Speech => self.speech_secs = displayed.max(0),
            TimerKind::Prep(Side::Pro) => {
                self.pro_prep_secs = displayed.clamp(0, PREP_SECS);
            }
            TimerKind::Prep(Side::Con) => {
                self.con_prep_secs = displayed.clamp(0, PREP_SECS);
            }
        }
    }

    /// Make `kind` the displayed timer and return its remaining seconds.
    /// The outgoing timer must already be banked via `bank_displayed`.
    pub fn switch_to(&mut self, kind: TimerKind) -> i32 {
        self.active = kind;
        match kind {
            TimerKind::Speech => self.speech_secs,
            TimerKind::Prep(side) => self.prep_secs(side),
        }
    }

    /// Start a fresh speech at `full_secs`, discarding the old speech
    /// remainder (advancing a speech is meant to do that). Prep balances
    /// are untouched.
    pub fn start_speech(&mut self, full_secs: i32) -> i32 {
        self.speech_secs = full_secs.max(0);
        self.active = TimerKind::Speech;
        self.speech_secs
    }

    /// Restore one team's prep balance to the full 3:00 — the only way a
    /// prep balance ever goes back up. Returns the value the display
    /// should show if that team's prep is currently on screen, else the
    /// untouched active timer's banked value via `secs_for`.
    pub fn reset_prep(&mut self, side: Side) {
        match side {
            Side::Pro => self.pro_prep_secs = PREP_SECS,
            Side::Con => self.con_prep_secs = PREP_SECS,
        }
    }

    /// Restore both prep balances (new round only).
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
    fn fmt_time_clamps_overrun_to_zero() {
        assert_eq!(fmt_time(-1), "0:00");
        assert_eq!(fmt_time(-90), "0:00");
        assert_eq!(fmt_time(i32::MIN), "0:00");
    }

    #[test]
    fn used_prep_resumes_remaining_after_a_speech() {
        let mut bank = TimerBank::new(240);
        // Enter Pro prep at the full 3:00, use 1:00, then go do a speech.
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Pro)), PREP_SECS);
        bank.bank_displayed(120);
        assert_eq!(bank.switch_to(TimerKind::Speech), 240);
        // Back to Pro prep: 2:00 left, not a fresh 3:00.
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Pro)), 120);
    }

    #[test]
    fn pro_and_con_prep_balances_are_independent() {
        let mut bank = TimerBank::new(240);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        bank.bank_displayed(100);
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Con)), PREP_SECS);
        bank.bank_displayed(45);
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Pro)), 100);
        assert_eq!(bank.prep_secs(Side::Con), 45);
    }

    #[test]
    fn speech_remainder_survives_a_prep_detour() {
        let mut bank = TimerBank::new(240);
        bank.bank_displayed(90); // speech paused with 1:30 left
        bank.switch_to(TimerKind::Prep(Side::Con));
        bank.bank_displayed(150);
        assert_eq!(bank.switch_to(TimerKind::Speech), 90);
    }

    #[test]
    fn prep_reset_is_per_team_and_only_via_reset_prep() {
        let mut bank = TimerBank::new(240);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        bank.bank_displayed(100);
        bank.switch_to(TimerKind::Prep(Side::Con));
        bank.bank_displayed(50);
        bank.reset_prep(Side::Pro);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
        assert_eq!(bank.prep_secs(Side::Con), 50, "Con balance must not move");
        // Merely re-entering a used prep timer never tops it up.
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Con)), 50);
    }

    #[test]
    fn expired_prep_stays_at_zero_until_explicitly_reset() {
        let mut bank = TimerBank::new(240);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        bank.bank_displayed(0);
        bank.switch_to(TimerKind::Speech);
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Pro)), 0);
        bank.reset_prep(Side::Pro);
        assert_eq!(bank.switch_to(TimerKind::Prep(Side::Pro)), PREP_SECS);
    }

    #[test]
    fn new_speech_discards_speech_remainder_but_keeps_prep() {
        let mut bank = TimerBank::new(240);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        bank.bank_displayed(111);
        assert_eq!(bank.start_speech(180), 180);
        assert_eq!(bank.active(), TimerKind::Speech);
        assert_eq!(bank.prep_secs(Side::Pro), 111);
    }

    #[test]
    fn banked_values_are_clamped_to_sane_ranges() {
        let mut bank = TimerBank::new(-5);
        assert_eq!(bank.secs_for(TimerKind::Speech, -5), 0);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        bank.bank_displayed(999);
        assert_eq!(bank.prep_secs(Side::Pro), PREP_SECS);
        bank.bank_displayed(-3);
        assert_eq!(bank.prep_secs(Side::Pro), 0);
    }
}
