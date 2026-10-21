//! Speech catalogue and the speech-by-speech guided walk.

use crate::model::Side;
use serde::{Deserialize, Serialize};
/// The 8 speeches that get flowed, left → right. Crossfires are not flowed.
#[derive(Clone, Copy)]
pub struct SpeechCol {
    pub full: &'static str,
    pub side: Side,
    pub minutes: u32,
}

pub const SPEECH_COLS: [SpeechCol; 8] = [
    SpeechCol {
        full: "Pro Constructive",
        side: Side::Pro,
        minutes: 4,
    },
    SpeechCol {
        full: "Con Constructive",
        side: Side::Con,
        minutes: 4,
    },
    SpeechCol {
        full: "Pro Rebuttal",
        side: Side::Pro,
        minutes: 4,
    },
    SpeechCol {
        full: "Con Rebuttal",
        side: Side::Con,
        minutes: 4,
    },
    SpeechCol {
        full: "Pro Summary",
        side: Side::Pro,
        minutes: 3,
    },
    SpeechCol {
        full: "Con Summary",
        side: Side::Con,
        minutes: 3,
    },
    SpeechCol {
        full: "Pro Final Focus",
        side: Side::Pro,
        minutes: 2,
    },
    SpeechCol {
        full: "Con Final Focus",
        side: Side::Con,
        minutes: 2,
    },
];

/// One step of the guided walk through the whole round (includes crossfires).
#[derive(Clone, Copy)]
pub struct GuideStep {
    pub title: &'static str,
    pub side: Option<Side>,
    pub minutes: u32,
    /// Which flow column this step writes in, if any.
    pub flow_col: Option<usize>,
    /// Momo-chan's one-line encouragement.
    pub bubble: &'static str,
    pub listen_for: &'static str,
    pub do_it: &'static str,
    pub watch_out: &'static str,
}

/// Sheet a guide step writes on. Rebuttals answer the opponent's case,
/// so they are written on the opponent's sheet; every other speech
/// writes on the speaker's sheet. `flow_col` is canonical, so 2/3 are
/// Pro/Con Rebuttal in any speaking order. Crossfires write nowhere
/// (both fields are `None`), so the caller leaves the sheet alone.
#[must_use]
pub fn sheet_for(step: &GuideStep) -> Option<Side> {
    match step.flow_col {
        Some(2) => Some(Side::Con),
        Some(3) => Some(Side::Pro),
        _ => step.side,
    }
}

/// Flowed steps take title, side and minutes from `SPEECH_COLS` so the
/// timer, sheet headers and guide cannot disagree (issue #35).
pub const GUIDE: [GuideStep; 11] = [
    GuideStep {
        title: SPEECH_COLS[0].full,
        side: Some(SPEECH_COLS[0].side),
        minutes: SPEECH_COLS[0].minutes,
        flow_col: Some(0),
        bubble: "First speech! Just catch the big ideas — nobody flows every word. ♡",
        listen_for: "“Our first contention is…” — each contention is a new row on the Pro sheet.",
        do_it: "Start one row per contention. Write 3–6 words: the claim + why it matters (the “impact”).",
        watch_out: "Don't write full sentences. Tone + signposting tell you what matters most.",
    },
    GuideStep {
        title: SPEECH_COLS[1].full,
        side: Some(SPEECH_COLS[1].side),
        minutes: SPEECH_COLS[1].minutes,
        flow_col: Some(1),
        bubble: "Flip to the Con sheet — their case gets its own page, just like paper!",
        listen_for: "Con's own contentions. They may also start answering Pro — but their case comes first.",
        do_it: "One row per Con contention on the Con sheet, same as before. Claim + impact, short and sweet.",
        watch_out: "Keep the sheets separate in your head: blue rows = Pro's case, green rows = Con's case.",
    },
    GuideStep {
        title: "Crossfire",
        side: None,
        minutes: 3,
        flow_col: None,
        bubble: "Pencils down, relax! ☕ Crossfire is questions — you don't have to flow it.",
        listen_for: "A clear concession (“yes, that's true”) or a debater unable to answer.",
        do_it: "Only jot a note in the Crossfire Notes box below if something big is admitted.",
        watch_out: "Don't decide anything yet — crossfire alone doesn't win rounds, the flow does.",
    },
    GuideStep {
        title: SPEECH_COLS[2].full,
        side: Some(SPEECH_COLS[2].side),
        minutes: SPEECH_COLS[2].minutes,
        flow_col: Some(2),
        bubble: "Now they answer each other! Write answers in the box directly RIGHT of the argument. ➜",
        listen_for: "“They say…, but…” — Pro is now answering the Con case.",
        do_it: "On the Con sheet, write Pro's answer in the Pro Reb column, on the same row as the Con argument it answers.",
        watch_out: "A Con row with a blank Pro Reb box = that argument went unanswered. Leave it blank on purpose!",
    },
    GuideStep {
        title: SPEECH_COLS[3].full,
        side: Some(SPEECH_COLS[3].side),
        minutes: SPEECH_COLS[3].minutes,
        flow_col: Some(3),
        bubble: "Same thing, other direction — Con answers Pro on the Pro sheet. You’ve got this!",
        listen_for: "Con answering each Pro contention, and defending their own case.",
        do_it: "On the Pro sheet, fill the Con Reb column row-by-row. Blank box = dropped argument — tap 💤 on the original.",
        watch_out: "New evidence names/dates? Just write the author's last name, e.g. “Smith '24”.",
    },
    GuideStep {
        title: "Crossfire",
        side: None,
        minutes: 3,
        flow_col: None,
        bubble: "Second crossfire — breathe, stretch, sip water. 🌷",
        listen_for: "Either side clarifying what their strongest argument is.",
        do_it: "Star ⭐ anything on your flow that keeps coming up — it's probably deciding the round.",
        watch_out: "Speakers may sound confident about arguments that aren't on the flow. Trust your paper.",
    },
    GuideStep {
        title: SPEECH_COLS[4].full,
        side: Some(SPEECH_COLS[4].side),
        minutes: SPEECH_COLS[4].minutes,
        flow_col: Some(4),
        bubble: "Summaries = “extend” time. If they carry an argument forward, draw your arrow ➜!",
        listen_for: "“Extend our…” and picking their best 1–2 arguments to focus on.",
        do_it: "Write the extension in the Pro Sum column on the same row, and tap ➜ extended on it.",
        watch_out: "Arguments NOT mentioned here are usually gone for good — Pro can't revive them later.",
    },
    GuideStep {
        title: SPEECH_COLS[5].full,
        side: Some(SPEECH_COLS[5].side),
        minutes: SPEECH_COLS[5].minutes,
        flow_col: Some(5),
        bubble: "Con does the same — which of their arguments survive?",
        listen_for: "“Extend…” plus answers to Pro's summary choices.",
        do_it: "Fill the Con Sum column on both sheets where Con speaks to that row. Mark ➜ extended.",
        watch_out: "Compare: which side still has live, answered-for arguments? That picture is the ballot forming.",
    },
    GuideStep {
        title: "Grand Crossfire",
        side: None,
        minutes: 3,
        flow_col: None,
        bubble: "Everyone's in! Last questions — almost there, you're doing great. ✨",
        listen_for: "“Why should the judge vote for you?” — they'll name their voting issues.",
        do_it: "Write those 1–2 named reasons in Crossfire Notes. Compare them with your ⭐ rows.",
        watch_out: "Grand crossfire claims still need to exist on the flow to count.",
    },
    GuideStep {
        title: SPEECH_COLS[6].full,
        side: Some(SPEECH_COLS[6].side),
        minutes: SPEECH_COLS[6].minutes,
        flow_col: Some(6),
        bubble: "Final Focus = “judge, vote for us because…”. Listen for weighing! ⚖️",
        listen_for: "Weighing words: bigger, sooner, more likely, “even if they win…”.",
        do_it: "Write their #1 reason in the Pro FF column on that argument's row. Star ⭐ it.",
        watch_out: "No brand-new arguments should appear here — if it's not on the flow already, be skeptical.",
    },
    GuideStep {
        title: SPEECH_COLS[7].full,
        side: Some(SPEECH_COLS[7].side),
        minutes: SPEECH_COLS[7].minutes,
        flow_col: Some(7),
        bubble: "Last speech! After this, the ballot below walks you through deciding.",
        listen_for: "Con's #1 reason + their weighing against Pro's #1 reason.",
        do_it: "Fill the Con FF column, star ⭐ their key argument, then scroll to “Your Decision”.",
        watch_out: "You don't need a perfect flow — decide from the rows with answers, extensions and weighing.",
    },
];

/// Who speaks first (issue #21). NSDA PF decides this by coin flip, so
/// the speaking sequence cannot be hard-coded Pro-first.
///
/// Storage invariant: a row's cells are always indexed by speech
/// identity (`SPEECH_COLS`: Pro Constructive is column 0 whatever the
/// order), never by speaking position. This order only changes the
/// sequence columns and guide steps are presented in, so flipping it
/// can never scramble notes already written under a speech.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum SpeakingOrder {
    #[default]
    ProFirst,
    ConFirst,
}

impl SpeakingOrder {
    /// Canonical `SPEECH_COLS` indices in speaking order: pairs swap
    /// (Constructives, Rebuttals, Summaries, Final Focuses).
    #[must_use]
    pub fn column_order(self) -> [usize; SPEECH_COLS.len()] {
        match self {
            Self::ProFirst => [0, 1, 2, 3, 4, 5, 6, 7],
            Self::ConFirst => [1, 0, 3, 2, 5, 4, 7, 6],
        }
    }

    /// Speaking position (0 = first speech) of a canonical column.
    #[must_use]
    pub fn position_of(self, col: usize) -> Option<usize> {
        self.column_order().iter().position(|&c| c == col)
    }

    /// 1-based CSS grid column presenting canonical column `col` in
    /// this speaking order. Grid column 1 holds the argument title, so
    /// speaking position 0 maps to grid column 2. Rendering every cell
    /// once in canonical order and placing it with this column lets a
    /// speaking-order flip re-sequence the existing DOM nodes via CSS
    /// instead of remounting them (issue #26). Unknown columns keep
    /// their canonical slot.
    #[must_use]
    pub fn grid_column(self, col: usize) -> usize {
        self.position_of(col).unwrap_or(col) + 2
    }

    /// Canonical column where `owner`'s case is first written: that
    /// side's Constructive, whichever speech of the round it is.
    #[must_use]
    pub fn origin_col(owner: Side) -> usize {
        SPEECH_COLS
            .iter()
            .position(|s| s.side == owner)
            .unwrap_or(0)
    }

    /// Canonical column of `answerer`'s Rebuttal: that side's second
    /// speech in `SPEECH_COLS`.
    #[must_use]
    pub fn rebuttal_col(answerer: Side) -> usize {
        SPEECH_COLS
            .iter()
            .enumerate()
            .filter(|(_, s)| s.side == answerer)
            .nth(1)
            .map_or(0, |(col, _)| col)
    }

    /// The guided walk in speaking order. Steps keep their canonical
    /// `flow_col`; only their sequence (and order-sensitive bubble
    /// wording such as “first speech” / “last speech”) changes.
    #[must_use]
    pub fn guide(self) -> [GuideStep; GUIDE.len()] {
        if self == Self::ProFirst {
            return GUIDE;
        }
        // Flowed steps take speeches in `column_order`, so pairs swap exactly
        // as the columns do; crossfires keep their slots.
        let order = self.column_order();
        let mut next = 0;
        let mut guide = GUIDE.map(|step| {
            if step.flow_col.is_none() {
                return step;
            }
            let col = order[next];
            next += 1;
            GUIDE
                .iter()
                .copied()
                .find(|s| s.flow_col == Some(col))
                .unwrap_or(step)
        });
        // Order-sensitive wording is keyed by speech, not by walk position.
        edit_speech(&mut guide, 1, |s| {
            s.bubble = "First speech! Just catch the big ideas — nobody flows every word. ♡";
            s.listen_for =
                "“Our first contention is…” — each contention is a new row on the Con sheet.";
            s.do_it = "Start one row per contention. Write 3–6 words: the claim + why it matters (the “impact”).";
        });
        edit_speech(&mut guide, 0, |s| {
            s.bubble = "Flip to the Pro sheet — their case gets its own page, just like paper!";
        });
        edit_speech(&mut guide, 3, |s| {
            s.bubble = "Now they answer each other! Write answers in the box directly RIGHT of the argument. ➜";
        });
        edit_speech(&mut guide, 2, |s| {
            s.bubble =
                "Same thing, other direction — Pro answers Con on the Con sheet. You’ve got this!";
        });
        edit_speech(&mut guide, 5, |s| {
            s.bubble =
                "Summaries = “extend” time. If they carry an argument forward, draw your arrow ➜!";
        });
        edit_speech(&mut guide, 4, |s| {
            s.bubble = "Pro does the same — which of their arguments survive?";
        });
        edit_speech(&mut guide, 7, |s| {
            s.bubble = "Final Focus = “judge, vote for us because…”. Listen for weighing! ⚖️";
            s.listen_for = "Con's #1 reason + weighing words: bigger, sooner, more likely, “even if they win…”";
        });
        edit_speech(&mut guide, 6, |s| {
            s.bubble = "Last speech! After this, the ballot below walks you through deciding.";
        });
        guide
    }
}

/// Applies `edit` to the walk's step for canonical speech column `col`.
fn edit_speech(guide: &mut [GuideStep], col: usize, edit: impl FnOnce(&mut GuideStep)) {
    if let Some(step) = guide.iter_mut().find(|s| s.flow_col == Some(col)) {
        edit(step);
    }
}
