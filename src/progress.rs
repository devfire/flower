//! Round progress that is not flow content: where the guided walk is,
//! each team's remaining prep balance, the speech remainder, the sheet
//! on screen, and focus mode (issue #17).
//!
//! Saved under its own `localStorage` key, separate from the round JSON:
//! it changes on every timer tick, while the flow changes per keystroke,
//! and an old save without progress must still load its flow untouched.

use serde::{Deserialize, Serialize};

use crate::model::{GUIDE, Side};
use crate::timer::{PREP_SECS, TimerBank, TimerKind};

pub const PROGRESS_STORAGE_KEY: &str = "flower-progress-v1";

/// `#[serde(default)]`: a progress save missing a field falls back to the
/// fresh-round value for that field instead of discarding everything.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RoundProgress {
    /// Index into `GUIDE`.
    pub step: usize,
    pub tab: Side,
    pub focus_mode: bool,
    /// Banked speech remainder; the displayed value when a prep timer is
    /// on screen instead.
    pub speech_secs: i32,
    pub pro_prep_secs: i32,
    pub con_prep_secs: i32,
    /// Which prep timer is on screen, or `None` for the speech timer.
    pub active_prep: Option<Side>,
}

impl Default for RoundProgress {
    fn default() -> Self {
        Self {
            step: 0,
            tab: Side::Pro,
            focus_mode: true,
            speech_secs: (GUIDE[0].minutes * 60).cast_signed(),
            pro_prep_secs: PREP_SECS,
            con_prep_secs: PREP_SECS,
            active_prep: None,
        }
    }
}

impl RoundProgress {
    /// Snapshot the live App signals. `displayed` is the on-screen timer
    /// value; `bank` holds the remainders of the timers not on screen.
    #[must_use]
    pub fn capture(
        step: usize,
        tab: Side,
        focus_mode: bool,
        displayed: i32,
        bank: TimerBank,
    ) -> Self {
        Self {
            step,
            tab,
            focus_mode,
            speech_secs: bank.secs_for(TimerKind::Speech, displayed),
            pro_prep_secs: bank.secs_for(TimerKind::Prep(Side::Pro), displayed),
            con_prep_secs: bank.secs_for(TimerKind::Prep(Side::Con), displayed),
            active_prep: match bank.active() {
                TimerKind::Speech => None,
                TimerKind::Prep(side) => Some(side),
            },
        }
        .normalised()
    }

    /// Parse saved progress, repairing out-of-range values a stale or
    /// hand-edited save may contain. Only genuinely unparseable JSON (or a
    /// field of the wrong type) is an `Err`; callers fall back to
    /// `RoundProgress::default()` then, losing no flow content.
    ///
    /// # Errors
    /// Returns the `serde_json` error when `json` is not readable progress.
    pub fn from_saved_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json).map(Self::normalised)
    }

    /// Clamp every field to a value the UI can use directly: a step that
    /// exists in `GUIDE`, a non-negative speech remainder no longer than
    /// the longest speech, and prep balances within `0..=PREP_SECS`.
    #[must_use]
    pub fn normalised(mut self) -> Self {
        self.step = self.step.min(GUIDE.len() - 1);
        let max_speech = GUIDE.iter().map(|g| g.minutes * 60).max().unwrap_or(0);
        self.speech_secs = self.speech_secs.clamp(0, max_speech.cast_signed());
        self.pro_prep_secs = self.pro_prep_secs.clamp(0, PREP_SECS);
        self.con_prep_secs = self.con_prep_secs.clamp(0, PREP_SECS);
        self
    }

    /// The timer bank to restore on load.
    #[must_use]
    pub fn timer_bank(&self) -> TimerBank {
        let active = self.active_prep.map_or(TimerKind::Speech, TimerKind::Prep);
        TimerBank::restored(
            self.speech_secs,
            self.pro_prep_secs,
            self.con_prep_secs,
            active,
        )
    }

    /// Seconds the timer display should show on load: the on-screen
    /// timer's remainder, whichever kind that is.
    #[must_use]
    pub fn displayed_secs(&self) -> i32 {
        match self.active_prep {
            None => self.speech_secs,
            Some(Side::Pro) => self.pro_prep_secs,
            Some(Side::Con) => self.con_prep_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_progress_starts_at_first_speech_with_full_prep() {
        let p = RoundProgress::default();
        assert_eq!(p.step, 0);
        assert_eq!(p.tab, Side::Pro);
        assert!(p.focus_mode);
        assert_eq!(p.displayed_secs(), (GUIDE[0].minutes * 60).cast_signed());
        assert_eq!(p.pro_prep_secs, PREP_SECS);
        assert_eq!(p.con_prep_secs, PREP_SECS);
    }

    #[test]
    fn saved_progress_round_trips_step_balances_and_active_timer() {
        // Repro from issue #17: Con Rebuttal (step 4), 2:00 of Pro prep
        // burned, Con sheet on screen, speech paused at 1:30.
        let saved = RoundProgress {
            step: 4,
            tab: Side::Con,
            focus_mode: false,
            speech_secs: 90,
            pro_prep_secs: 60,
            con_prep_secs: PREP_SECS,
            active_prep: Some(Side::Pro),
        };
        let json = serde_json::to_string(&saved).unwrap_or_default();
        let Ok(loaded) = RoundProgress::from_saved_json(&json) else {
            panic!("saved progress must reload");
        };
        assert_eq!(loaded, saved);
        // On-screen timer is Pro prep: display shows its 1:00, while the
        // banked speech remainder survives behind it.
        assert_eq!(loaded.displayed_secs(), 60);
        let bank = loaded.timer_bank();
        assert_eq!(bank.active(), TimerKind::Prep(Side::Pro));
        assert_eq!(bank.secs_for(TimerKind::Speech, 60), 90);
        assert_eq!(bank.prep_secs(Side::Con), PREP_SECS);
    }

    #[test]
    fn capture_banks_displayed_value_into_active_timer_only() {
        let mut bank = TimerBank::new(240);
        bank.switch_to(TimerKind::Prep(Side::Pro));
        let p = RoundProgress::capture(4, Side::Con, true, 75, bank);
        assert_eq!(p.step, 4);
        assert_eq!(p.active_prep, Some(Side::Pro));
        assert_eq!(p.pro_prep_secs, 75, "displayed prep value must be banked");
        assert_eq!(p.speech_secs, 240, "banked speech remainder must survive");
        assert_eq!(p.con_prep_secs, PREP_SECS);
    }

    #[test]
    fn out_of_range_saved_values_are_clamped_not_trusted() {
        let p = RoundProgress {
            step: 999,
            speech_secs: -30,
            pro_prep_secs: 9_999,
            con_prep_secs: -1,
            ..RoundProgress::default()
        }
        .normalised();
        assert_eq!(p.step, GUIDE.len() - 1);
        assert_eq!(p.speech_secs, 0);
        assert_eq!(p.pro_prep_secs, PREP_SECS);
        assert_eq!(p.con_prep_secs, 0);
    }

    #[test]
    fn missing_fields_default_and_corrupt_json_is_an_error() {
        let Ok(loaded) = RoundProgress::from_saved_json(r#"{"step":3}"#) else {
            panic!("partial progress must load");
        };
        assert_eq!(loaded.step, 3);
        assert_eq!(loaded.pro_prep_secs, PREP_SECS);
        assert!(loaded.focus_mode);
        assert!(RoundProgress::from_saved_json("{nope").is_err());
        assert!(RoundProgress::from_saved_json(r#"{"step":"four"}"#).is_err());
    }
}
