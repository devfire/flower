//! Dropped/marked-argument analysis and the decision summary.

use crate::guide::{SPEECH_COLS, SpeakingOrder};
use crate::model::{Mark, RoundData, RowData, Side};
/// An argument the opponent had a fair chance to answer — their rebuttal
/// speech is over — yet every opponent answer box on its row is blank,
/// or one the judge explicitly marked 💤 dropped.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DroppedArgument {
    pub owner: Side,
    pub answerer: Side,
    /// Row title if the judge gave one, else the argument's own text.
    pub name: String,
    /// Judge tapped 💤 on this row: surface it even if an answer box has
    /// text (the judge judged it not an answer) or rebuttal is not over.
    pub explicit: bool,
    /// Any cell on the row is marked ➜ extended.
    pub extended: bool,
    /// Any cell on the row is marked ⭐ key.
    pub key: bool,
}

/// A row carrying a judge-placed mark, for the decision summary.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MarkedArgument {
    pub owner: Side,
    /// Row title if given, else the argument's own text.
    pub name: String,
    /// Speech (`SPEECH_COLS` full name) of the first cell with the mark
    /// in the round's speaking order.
    pub speech: &'static str,
}

fn row_display_name(row: &RowData, origin_col: usize) -> String {
    let title = row.title.trim();
    if !title.is_empty() {
        return title.to_owned();
    }
    if let Some(origin) = row.cells.get(origin_col)
        && !origin.text.trim().is_empty()
    {
        return origin.text.trim().to_owned();
    }
    row.cells
        .iter()
        .find(|c| !c.text.trim().is_empty())
        .map_or_else(
            || "Unnamed argument".to_owned(),
            |c| c.text.trim().to_owned(),
        )
}

impl RoundData {
    /// Arguments that look dropped at guide step `guide_step` (an index
    /// into this round's `SpeakingOrder::guide`). Pure so the ballot hints can be unit-tested and the
    /// view only has to feed it the tracked row/order inputs.
    ///
    /// Inferred drop: opponent's rebuttal is already over and *every*
    /// opponent-speech box in a completed-or-current speech on the row
    /// is blank — an answer in
    /// Constructive, Summary, or Final Focus counts just as much as one
    /// in Rebuttal. Explicit drop: the judge marked a cell 💤; that is
    /// surfaced immediately, rebuttal gate and answer text notwithstanding,
    /// because the judge — not the text heuristic — decided it.
    #[must_use]
    pub fn dropped_arguments(&self, guide_step: usize) -> Vec<DroppedArgument> {
        // Guide steps that ended before the current step, in this
        // round's speaking order (issue #21).
        let order = self.speaking_order;
        let guide = order.guide();
        let completed: Vec<usize> = guide
            .iter()
            .take(guide_step.min(guide.len()))
            .filter_map(|g| g.flow_col)
            .collect();
        // Speeches completed or happening now: text in one of these can
        // be an answer, text typed ahead into a future speech cannot.
        let eligible: Vec<usize> = guide
            .iter()
            .take(guide_step.saturating_add(1).min(guide.len()))
            .filter_map(|g| g.flow_col)
            .collect();
        let mut dropped = Vec::new();
        for (rows, owner) in [(&self.pro_rows, Side::Pro), (&self.con_rows, Side::Con)] {
            let answerer = owner.other();
            // Derived, not hard-coded to Pro-first constants: the
            // owner's Constructive is its origin, and the gate is the
            // answerer's Rebuttal having ended in this order.
            let origin_col = SpeakingOrder::origin_col(owner);
            let rebuttal_col = SpeakingOrder::rebuttal_col(answerer);
            let origin_pos = order.position_of(origin_col).unwrap_or(0);
            for row in rows {
                let has_mark = |mark: Mark| row.cells.iter().any(|c| c.mark == mark);
                let explicit = has_mark(Mark::Dropped);
                let extended = has_mark(Mark::Extended);
                let key = has_mark(Mark::Key);
                if explicit {
                    dropped.push(DroppedArgument {
                        owner,
                        answerer,
                        name: row_display_name(row, origin_col),
                        explicit: true,
                        extended,
                        key,
                    });
                    continue;
                }
                // Inferred only: the answerer has not had its rebuttal yet.
                if !completed.contains(&rebuttal_col) {
                    continue;
                }
                let origin = row.cells.get(origin_col).map_or("", |c| c.text.trim());
                if origin.is_empty() {
                    continue;
                }
                // Any opponent box in a later speech counts, even in
                // the speech happening now: text there means the answer
                // was given. Text in a speech that has not happened yet
                // does not count. “Later” is speaking position, not
                // canonical column — when Con speaks first, Pro's
                // Constructive (canonical 0) answers Con's (canonical 1).
                let has_answer = row.cells.iter().enumerate().any(|(col, cell)| {
                    eligible.contains(&col)
                        && order.position_of(col).is_some_and(|pos| pos > origin_pos)
                        && SPEECH_COLS.get(col).is_some_and(|s| s.side == answerer)
                        && !cell.text.trim().is_empty()
                });
                if !has_answer {
                    dropped.push(DroppedArgument {
                        owner,
                        answerer,
                        name: row_display_name(row, origin_col),
                        explicit: false,
                        extended,
                        key,
                    });
                }
            }
        }
        dropped
    }

    /// Rows carrying `mark` (💤/➜/⭐), in flow order, for the decision
    /// summary. The reported speech is the first marked cell in this
    /// round's speaking order, not canonical column order.
    /// `Mark::None` yields nothing — it is absence of a mark.
    #[must_use]
    pub fn marked_arguments(&self, mark: Mark) -> Vec<MarkedArgument> {
        if mark == Mark::None {
            return Vec::new();
        }
        let mut marked = Vec::new();
        for (rows, owner) in [(&self.pro_rows, Side::Pro), (&self.con_rows, Side::Con)] {
            let origin_col = SpeakingOrder::origin_col(owner);
            for row in rows {
                if let Some(speech) = self
                    .speaking_order
                    .column_order()
                    .into_iter()
                    .find(|&col| row.cells.get(col).is_some_and(|c| c.mark == mark))
                    .and_then(|col| SPEECH_COLS.get(col))
                    .map(|s| s.full)
                {
                    marked.push(MarkedArgument {
                        owner,
                        name: row_display_name(row, origin_col),
                        speech,
                    });
                }
            }
        }
        marked
    }

    /// Plain-text decision summary copied to the clipboard: ballot fields
    /// plus the rows the judge starred, extended, or marked dropped —
    /// the basis of the decision, previously invisible to the summary.
    #[must_use]
    pub fn decision_text(&self) -> String {
        use std::fmt::Write as _;
        let winner = self.winner.map_or("—", Side::label);
        let mut text = format!(
            "Flower — PF decision\nTopic: {}\nPro: {} · Con: {}\nWinner: {}\nReason: {}",
            self.topic, self.pro_team, self.con_team, winner, self.rfd
        );
        for (mark, heading) in [
            (Mark::Key, "⭐ Key arguments:"),
            (Mark::Extended, "➜ Extended arguments:"),
            (Mark::Dropped, "💤 Marked dropped:"),
        ] {
            let rows = self.marked_arguments(mark);
            if !rows.is_empty() {
                let _ = write!(text, "\n{heading}");
                for row in rows {
                    let _ = write!(
                        text,
                        "\n- {}: “{}” ({})",
                        row.owner.label(),
                        row.name,
                        row.speech
                    );
                }
            }
        }
        text
    }
}
