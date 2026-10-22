//! Round progress that is not flow content: where the guided walk is,
//! the speech remainder, each team's remaining prep balance, the sheet
//! on screen, and focus mode (issue #17).
//!
//! Saved under its own `localStorage` key, separate from the round JSON:
//! it changes on every timer tick, while the flow changes per keystroke,
//! and an old save without progress must still load its flow untouched.
use serde::{Deserialize, Serialize};

use crate::model::{GUIDE, Side};
use crate::timer::{PREP_SECS, TimerBank};

pub const PROGRESS_STORAGE_KEY: &str = "flower-progress-v1";

/// Fields are private so that every value is built by `capture`,
/// `from_saved_json`, or `Default` — all of which clamp via `normalised`.
/// Deserialisation goes through `SavedProgress` for the same reason.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(from = "SavedProgress")]
pub struct RoundProgress {
    /// Index into `GUIDE`.
    step: usize,
    tab: Side,
    focus_mode: bool,
    /// Speech timer remainder (always the on-screen countdown).
    speech_secs: i32,
    pro_prep_secs: i32,
    con_prep_secs: i32,
}

/// Wire shape of a progress save. `#[serde(default)]`: a save missing a
/// field falls back to the fresh-round value for that field instead of
/// discarding everything.
#[derive(Deserialize)]
#[serde(default)]
struct SavedProgress {
    step: usize,
    tab: Side,
    focus_mode: bool,
    speech_secs: i32,
    pro_prep_secs: i32,
    con_prep_secs: i32,
    /// Pre-chip-countdown save shape: accepted and ignored, so old
    /// saves still load without resurrecting the mode switch.
    active_prep: Option<Side>,
}

impl Default for SavedProgress {
    fn default() -> Self {
        let fresh = RoundProgress::default();
        Self {
            step: fresh.step,
            tab: fresh.tab,
            focus_mode: fresh.focus_mode,
            speech_secs: fresh.speech_secs,
            pro_prep_secs: fresh.pro_prep_secs,
            con_prep_secs: fresh.con_prep_secs,
            active_prep: None,
        }
    }
}

impl From<SavedProgress> for RoundProgress {
    fn from(saved: SavedProgress) -> Self {
        Self {
            step: saved.step,
            tab: saved.tab,
            focus_mode: saved.focus_mode,
            speech_secs: saved.speech_secs,
            pro_prep_secs: saved.pro_prep_secs,
            con_prep_secs: saved.con_prep_secs,
        }
        .normalised()
    }
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
        }
    }
}

impl RoundProgress {
    /// Snapshot the live App signals. `displayed` is the on-screen
    /// speech value; `bank` holds every remainder at once.
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
            speech_secs: displayed.max(0),
            pro_prep_secs: bank.prep_secs(Side::Pro),
            con_prep_secs: bank.prep_secs(Side::Con),
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
        serde_json::from_str(json)
    }

    /// Index into `GUIDE` of the step the walk is on.
    #[must_use]
    pub const fn step(&self) -> usize {
        self.step
    }

    #[must_use]
    pub const fn tab(&self) -> Side {
        self.tab
    }

    #[must_use]
    pub const fn focus_mode(&self) -> bool {
        self.focus_mode
    }

    /// Clamp every field to a value the UI can use directly: a step that
    /// exists in `GUIDE`, a non-negative speech remainder no longer than
    /// the longest speech, and prep balances within `0..=PREP_SECS`.
    #[must_use]
    fn normalised(mut self) -> Self {
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
        TimerBank::restored(self.speech_secs, self.pro_prep_secs, self.con_prep_secs)
    }

    /// Seconds the speech timer display should show on load.
    #[must_use]
    pub const fn displayed_secs(&self) -> i32 {
        self.speech_secs
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
    fn saved_progress_round_trips_step_and_balances() {
        // Repro from issue #17: Con Rebuttal (step 4), 2:00 of Pro prep
        // burned, Con sheet on screen, speech paused at 1:30.
        let saved = RoundProgress {
            step: 4,
            tab: Side::Con,
            focus_mode: false,
            speech_secs: 90,
            pro_prep_secs: 60,
            con_prep_secs: PREP_SECS,
        };
        let json = serde_json::to_string(&saved).unwrap_or_default();
        let Ok(loaded) = RoundProgress::from_saved_json(&json) else {
            panic!("saved progress must reload");
        };
        assert_eq!(loaded, saved);
        // The speech display shows its own 1:30 remainder; prep balances
        // ride along untouched.
        assert_eq!(loaded.displayed_secs(), 90);
        let bank = loaded.timer_bank();
        assert_eq!(bank.speech_secs(), 90);
        assert_eq!(bank.prep_secs(Side::Pro), 60);
        assert_eq!(bank.prep_secs(Side::Con), PREP_SECS);
    }

    #[test]
    fn old_prep_mode_save_still_loads_without_resurrecting_it() {
        let Ok(loaded) =
            RoundProgress::from_saved_json(r#"{"step":4,"active_prep":"Pro","pro_prep_secs":60}"#)
        else {
            panic!("old progress shape must load");
        };
        assert_eq!(loaded.step, 4);
        assert_eq!(
            loaded.displayed_secs(),
            (GUIDE[0].minutes * 60).cast_signed()
        );
        assert_eq!(loaded.pro_prep_secs, 60);
        assert_eq!(loaded.timer_bank().prep_secs(Side::Pro), 60);
    }

    #[test]
    fn capture_banks_speech_display_and_prep_balances() {
        let mut bank = TimerBank::restored(240, PREP_SECS, PREP_SECS);
        bank.set_prep(Side::Pro, 100);
        let p = RoundProgress::capture(4, Side::Con, true, 75, bank);
        assert_eq!(p.step, 4);
        assert_eq!(p.speech_secs, 75, "speech display must be banked");
        assert_eq!(p.pro_prep_secs, 100, "prep balance must ride along");
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
    fn direct_serde_load_is_clamped_too() {
        let Ok(loaded) = serde_json::from_str::<RoundProgress>(r#"{"step":999}"#) else {
            panic!("out-of-range progress must still load");
        };
        assert_eq!(loaded.step(), GUIDE.len() - 1);
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
