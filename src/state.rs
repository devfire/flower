//! Granular per-cell signals: each textarea owns an `RwSignal`, so typing
//! never re-creates DOM nodes or loses the caret. A plain `RoundData`
//! snapshot is rebuilt (untracked) only when saving / exporting.

use crate::model::{
    CellData, DroppedArgument, Mark, RoundData, RowData, SPEECH_COLS, Side, SpeakingOrder,
};
use leptos::prelude::*;

pub use crate::persistence::{load_initial, save, save_progress};
pub use crate::platform::{copy_text, now_ms, print_page, timer_expired_feedback, unlock_audio};

#[derive(Clone, Copy)]
pub struct CellState {
    pub text: RwSignal<String>,
    pub mark: RwSignal<Mark>,
}

#[derive(Clone)]
pub struct RowState {
    pub id: u32,
    pub title: RwSignal<String>,
    pub cells: [CellState; SPEECH_COLS.len()],
}

impl RowState {
    fn from_data(data: &RowData) -> Self {
        // Exhaustive destructure: a new `RowData` field is a compile
        // error here until it is wired into a signal.
        let RowData { id, title, cells } = data;
        Self {
            id: *id,
            title: RwSignal::new(title.clone()),
            cells: cells.each_ref().map(|c| {
                let CellData { text, mark } = c;
                CellState {
                    text: RwSignal::new(text.clone()),
                    mark: RwSignal::new(*mark),
                }
            }),
        }
    }

    /// The single row construction list. `tracked` selects `get()`
    /// (subscribes the calling reactive scope, for derived views) over
    /// `get_untracked()` (saving/export); there is no second twin list
    /// to drift out of sync.
    fn to_data(&self, tracked: bool) -> RowData {
        RowData {
            id: self.id,
            title: read_signal(self.title, tracked),
            cells: self.cells.each_ref().map(|c| CellData {
                text: read_signal(c.text, tracked),
                mark: read_signal(c.mark, tracked),
            }),
        }
    }
}

/// Read `signal` tracked or untracked, per `tracked`.
fn read_signal<T: Clone + Send + Sync + 'static>(signal: RwSignal<T>, tracked: bool) -> T {
    if tracked {
        signal.get()
    } else {
        signal.get_untracked()
    }
}

fn read_rows(rows: RwSignal<Vec<RowState>>, tracked: bool) -> Vec<RowData> {
    read_signal(rows, tracked)
        .iter()
        .map(|row| row.to_data(tracked))
        .collect()
}

#[derive(Clone)]
pub struct RoundState {
    pub topic: RwSignal<String>,
    pub pro_team: RwSignal<String>,
    pub con_team: RwSignal<String>,
    pub speaking_order: RwSignal<SpeakingOrder>,
    pub pro_rows: RwSignal<Vec<RowState>>,
    pub con_rows: RwSignal<Vec<RowState>>,
    pub notes: RwSignal<String>,
    pub winner: RwSignal<Option<Side>>,
    pub rfd: RwSignal<String>,
    pub next_id: RwSignal<u32>,
    /// True after the last `save` failed (storage full/blocked/unavailable),
    /// so the UI can stop claiming the round is saved.
    pub save_failed: RwSignal<bool>,
    /// Set when the stored round could not be read: explains that the raw
    /// save was preserved, and blocks `save` from overwriting it until the
    /// judge dismisses it (or explicitly replaces the round).
    pub recovery_notice: RwSignal<Option<String>>,
}

impl RoundState {
    pub fn from_data(data: RoundData) -> Self {
        // Defence in depth: every entry path (saved, sample, default) gets
        // the cell-count / id invariants before the UI can index by column.
        // Saved data reaches here only via `from_saved_json`, which already
        // rejected id-exhausted rounds; other callers pass default/sample
        // data. A pathological in-code round falls back to the blank
        // default rather than trapping the wasm app with duplicate ids.
        let data = data.try_normalised().unwrap_or_default();
        // Exhaustive destructure: a new `RoundData` field is a compile
        // error here until it gets a signal below.
        let RoundData {
            topic,
            pro_team,
            con_team,
            speaking_order,
            pro_rows,
            con_rows,
            notes,
            winner,
            rfd,
            next_id,
        } = data;
        Self {
            topic: RwSignal::new(topic),
            pro_team: RwSignal::new(pro_team),
            con_team: RwSignal::new(con_team),
            speaking_order: RwSignal::new(speaking_order),
            pro_rows: RwSignal::new(pro_rows.iter().map(RowState::from_data).collect()),
            con_rows: RwSignal::new(con_rows.iter().map(RowState::from_data).collect()),
            notes: RwSignal::new(notes),
            winner: RwSignal::new(winner),
            rfd: RwSignal::new(rfd),
            next_id: RwSignal::new(next_id),
            save_failed: RwSignal::new(false),
            recovery_notice: RwSignal::new(None),
        }
    }

    pub fn snapshot(&self) -> RoundData {
        self.snapshot_with_tracking(false)
    }

    /// Tracked dropped-argument derivation for the ballot hints: reads
    /// (subscribes the caller to) only the inputs `dropped_arguments`
    /// actually uses — speaking order and the row titles/cells — so
    /// typing in the topic, team, notes, winner, or RFD fields never
    /// re-derives hints. The other `RoundData` fields are inert defaults
    /// the derivation never reads; every field is named so a new field
    /// forces a decision here instead of silently defaulting.
    pub fn dropped_arguments_tracked(&self, guide_step: usize) -> Vec<DroppedArgument> {
        RoundData {
            topic: String::new(),
            pro_team: String::new(),
            con_team: String::new(),
            speaking_order: self.speaking_order.get(),
            pro_rows: read_rows(self.pro_rows, true),
            con_rows: read_rows(self.con_rows, true),
            notes: String::new(),
            winner: None,
            rfd: String::new(),
            next_id: 0,
        }
        .dropped_arguments(guide_step)
    }

    /// The single `RoundData` construction list; `tracked` only selects
    /// how each signal is read (see `RowState::to_data`).
    fn snapshot_with_tracking(&self, tracked: bool) -> RoundData {
        RoundData {
            topic: read_signal(self.topic, tracked),
            pro_team: read_signal(self.pro_team, tracked),
            con_team: read_signal(self.con_team, tracked),
            speaking_order: read_signal(self.speaking_order, tracked),
            pro_rows: read_rows(self.pro_rows, tracked),
            con_rows: read_rows(self.con_rows, tracked),
            notes: read_signal(self.notes, tracked),
            winner: read_signal(self.winner, tracked),
            rfd: read_signal(self.rfd, tracked),
            next_id: read_signal(self.next_id, tracked),
        }
    }

    pub fn rows(&self, side: Side) -> RwSignal<Vec<RowState>> {
        match side {
            Side::Pro => self.pro_rows,
            Side::Con => self.con_rows,
        }
    }

    pub fn replace(&self, data: RoundData) {
        // An explicit replace (example / new round) is the judge choosing to
        // discard the unreadable save, so it lifts the overwrite block.
        self.recovery_notice.set(None);
        let fresh = Self::from_data(data);
        // Exhaustive destructure: a new `RoundState` field is a compile
        // error here until it is copied (or deliberately not copied).
        // `save_failed`/`recovery_notice` are session state, not round
        // data: a fresh round must not clear a save-failure warning, and
        // `recovery_notice` was already cleared above.
        let Self {
            topic,
            pro_team,
            con_team,
            speaking_order,
            pro_rows,
            con_rows,
            notes,
            winner,
            rfd,
            next_id,
            save_failed: _,
            recovery_notice: _,
        } = fresh;
        self.topic.set(topic.get_untracked());
        self.pro_team.set(pro_team.get_untracked());
        self.con_team.set(con_team.get_untracked());
        self.speaking_order.set(speaking_order.get_untracked());
        self.pro_rows.set(pro_rows.get_untracked());
        self.con_rows.set(con_rows.get_untracked());
        self.notes.set(notes.get_untracked());
        self.winner.set(winner.get_untracked());
        self.rfd.set(rfd.get_untracked());
        self.next_id.set(next_id.get_untracked());
    }

    pub fn add_row(&self, side: Side) {
        let id = self.next_id.get_untracked();
        // `next_id == u32::MAX` (a hand-edited save may set it) has no
        // successor: adding the row would panic on `id + 1` in debug or
        // wrap to an id already in use in release. Refuse instead.
        let Some(next) = id.checked_add(1) else {
            return;
        };
        self.next_id.set(next);
        self.rows(side)
            .update(|rows| rows.push(RowState::from_data(&RowData::new(id, ""))));
        save(self);
    }

    pub fn remove_row(&self, side: Side, id: u32) {
        self.rows(side).update(|rows| rows.retain(|r| r.id != id));
        save(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mark;

    fn distinctive_round() -> RoundData {
        let mut data = RoundData {
            topic: "Resolved: distinctive".to_owned(),
            pro_team: "Pro team".to_owned(),
            con_team: "Con team".to_owned(),
            speaking_order: SpeakingOrder::ConFirst,
            pro_rows: vec![RowData::new(10, "Pro contention"), RowData::new(11, "")],
            con_rows: vec![RowData::new(20, "Con contention")],
            notes: "a note".to_owned(),
            winner: Some(Side::Con),
            rfd: "Con wins on weighing".to_owned(),
            next_id: 99,
        };
        data.pro_rows[0].cells[0].text = "pro cell".to_owned();
        data.pro_rows[0].cells[0].mark = Mark::Dropped;
        data.con_rows[0].cells[1].text = "con cell".to_owned();
        data.con_rows[0].cells[1].mark = Mark::Key;
        data
    }

    /// Field-by-field equality (`RoundData` has no `PartialEq`): every
    /// field is named, so a snapshot/replace that drops one fails here.
    fn assert_round_eq(actual: &RoundData, expected: &RoundData) {
        assert_eq!(actual.topic, expected.topic);
        assert_eq!(actual.pro_team, expected.pro_team);
        assert_eq!(actual.con_team, expected.con_team);
        assert_eq!(actual.speaking_order, expected.speaking_order);
        assert_eq!(actual.notes, expected.notes);
        assert_eq!(actual.winner, expected.winner);
        assert_eq!(actual.rfd, expected.rfd);
        assert_eq!(actual.next_id, expected.next_id);
        for (actual_rows, expected_rows) in [
            (&actual.pro_rows, &expected.pro_rows),
            (&actual.con_rows, &expected.con_rows),
        ] {
            assert_eq!(actual_rows.len(), expected_rows.len());
            for (actual_row, expected_row) in actual_rows.iter().zip(expected_rows) {
                assert_eq!(actual_row.id, expected_row.id);
                assert_eq!(actual_row.title, expected_row.title);
                assert_eq!(actual_row.cells.len(), expected_row.cells.len());
                for (actual_cell, expected_cell) in actual_row.cells.iter().zip(&expected_row.cells)
                {
                    assert_eq!(actual_cell.text, expected_cell.text);
                    assert_eq!(actual_cell.mark, expected_cell.mark);
                }
            }
        }
    }

    #[test]
    fn snapshot_covers_every_field() {
        let data = distinctive_round();
        let state = RoundState::from_data(data.clone());
        assert_round_eq(&state.snapshot(), &data);
    }

    #[test]
    fn tracked_dropped_arguments_match_full_derivation() {
        // Con-first `distinctive_round`: equality across steps proves the
        // narrow tracked derivation (speaking order + rows only) agrees
        // with the full-data derivation, speaking order included.
        let data = distinctive_round();
        let state = RoundState::from_data(data.clone());
        for step in 0..=crate::model::GUIDE.len() {
            assert_eq!(
                state.dropped_arguments_tracked(step),
                data.dropped_arguments(step),
                "step {step}"
            );
        }
    }

    #[test]
    fn replace_copies_every_round_field_but_keeps_session_state() {
        let state = RoundState::from_data(RoundData::default());
        state.save_failed.set(true);
        state
            .recovery_notice
            .set(Some("unreadable save".to_owned()));
        let data = distinctive_round();
        state.replace(data.clone());
        assert_round_eq(&state.snapshot(), &data);
        // Session state is not round data: replace clears the recovery
        // block (explicit discard) but not a save-failure warning.
        assert_eq!(state.recovery_notice.get_untracked(), None);
        assert!(state.save_failed.get_untracked());
    }
}
