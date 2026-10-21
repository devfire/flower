//! Round data + the speech-by-speech guide.
//!
//! Flowing rules encoded here come from the NSDA "Intro to Flowing" judge
//! training: one page per side, one column per speech, responses written
//! directly to the right of the argument they answer, a blank box means the
//! argument was dropped, and an arrow means it was extended.

use serde::{Deserialize, Serialize};

pub use crate::analysis::DroppedArgument;
pub use crate::guide::{GUIDE, SPEECH_COLS, SpeakingOrder, sheet_for};
pub use crate::round_save::{
    STORAGE_BACKUP_KEY, STORAGE_KEY, STORAGE_REVISION_KEY, next_revision, stored_revision,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Side {
    Pro,
    Con,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Side::Pro => "Pro",
            Side::Con => "Con",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Side::Pro => Side::Con,
            Side::Con => Side::Pro,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Mark {
    #[default]
    None,
    /// No answer was given — leave the box blank, this flag remembers why.
    Dropped,
    /// Carried ("extended") into a later speech — draw the arrow.
    Extended,
    /// This is the argument the round may be decided on.
    Key,
}

impl Mark {
    pub fn label(self) -> &'static str {
        match self {
            Mark::None => "",
            Mark::Dropped => "💤 dropped",
            Mark::Extended => "➜ extended",
            Mark::Key => "⭐ key",
        }
    }
}

/// `#[serde(default)]`: a save missing a field loads with that field
/// defaulted instead of failing the whole round (see `RoundData::normalised`).
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CellData {
    pub text: String,
    pub mark: Mark,
}

/// Serde bridge for `RowData::cells`: saves may hold any number of cells,
/// so load pads short rows and drops extra ones, leaving the array length
/// (the compile-time invariant) as the only shape the rest of the code sees.
mod cells_serde {
    use super::{CellData, SPEECH_COLS};
    use serde::{Deserialize, Deserializer, de::Error as _};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[CellData; SPEECH_COLS.len()], D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut cells = Vec::<CellData>::deserialize(deserializer)?;
        cells.resize_with(SPEECH_COLS.len(), CellData::default);
        cells
            .try_into()
            .map_err(|_| D::Error::custom("row cells could not be repaired to one per speech"))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RowData {
    pub id: u32,
    pub title: String,
    /// One cell per speech, by construction: the array length is
    /// `SPEECH_COLS.len()`, so a row with any other count will not compile.
    /// Loaded saves are padded or truncated by `cells_serde` on the way in.
    #[serde(deserialize_with = "cells_serde::deserialize")]
    pub cells: [CellData; SPEECH_COLS.len()],
}

impl Default for RowData {
    fn default() -> Self {
        Self::new(0, "")
    }
}

impl RowData {
    pub fn new(id: u32, title: &str) -> Self {
        Self {
            id,
            title: title.to_owned(),
            cells: std::array::from_fn(|_| CellData::default()),
        }
    }
}

/// Serde bridge for `RoundData::winner`: the pre-`Option<Side>` save
/// format was a bare string, so keep writing that string (old builds
/// can still read new saves) and accept it on load, mapping anything
/// other than `Pro`/`Con` to no winner instead of failing the round.
mod winner_serde {
    use super::Side;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Side>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Option::<String>::deserialize(deserializer)?;
        Ok(match raw.as_deref() {
            Some("Pro") => Some(Side::Pro),
            Some("Con") => Some(Side::Con),
            _ => None,
        })
    }

    // `serialize_with` contract: serde passes `&Option<Side>`.
    #[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(winner: &Option<Side>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(winner.map_or("", Side::label))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoundData {
    pub topic: String,
    pub pro_team: String,
    pub con_team: String,
    /// Coin-flip speaking order; absent in older saves, which were
    /// necessarily Pro-first, so the serde default is correct for them.
    pub speaking_order: SpeakingOrder,
    pub pro_rows: Vec<RowData>,
    pub con_rows: Vec<RowData>,
    pub crossfire_notes: String,
    /// Ballot winner; `None` until the judge picks a side. Old saves
    /// stored `""`/`"Pro"`/`"Con"` (any other string was a phantom
    /// vote); `winner_serde` maps those onto this type on load and
    /// keeps the same on-disk strings on save.
    #[serde(
        deserialize_with = "winner_serde::deserialize",
        serialize_with = "winner_serde::serialize"
    )]
    pub winner: Option<Side>,
    pub rfd: String,
    pub next_id: u32,
}

impl Default for RoundData {
    fn default() -> Self {
        Self {
            topic: String::new(),
            pro_team: String::new(),
            con_team: String::new(),
            speaking_order: SpeakingOrder::ProFirst,
            pro_rows: vec![RowData::new(1, ""), RowData::new(2, "")],
            con_rows: vec![RowData::new(3, ""), RowData::new(4, "")],
            crossfire_notes: String::new(),
            winner: None,
            rfd: String::new(),
            next_id: 5,
        }
    }
}

impl RoundData {
    pub fn sample() -> Self {
        let mut r = Self {
            topic: "Resolved: The United States should ban single-use plastics.".to_owned(),
            pro_team: "Pro — Maya & Jun".to_owned(),
            con_team: "Con — Leo & Priya".to_owned(),
            ..Self::default()
        };
        let mut pro1 = RowData::new(10, "C1: Oceans 🌊");
        "Plastic → oceans, harms sea life".clone_into(&mut pro1.cells[0].text);
        "“Recycling fixes it” — only 9% recycled".clone_into(&mut pro1.cells[3].text);
        pro1.cells[3].mark = Mark::Extended;
        "➜ Extend: harm is permanent".clone_into(&mut pro1.cells[4].text);
        pro1.cells[4].mark = Mark::Extended;
        let mut pro2 = RowData::new(11, "C2: Health 🧸");
        "Microplastics → drinking water".clone_into(&mut pro2.cells[0].text);
        pro2.cells[0].mark = Mark::Key;
        let mut con1 = RowData::new(12, "C1: Economy 💰");
        "Ban → costs ↑ for small shops".clone_into(&mut con1.cells[1].text);
        "Shops adapt — paper is cheap now".clone_into(&mut con1.cells[2].text);
        let mut con2 = RowData::new(13, "C2: Alternatives 🌱");
        "Compostables replace plastic".clone_into(&mut con2.cells[1].text);
        con2.cells[1].mark = Mark::Dropped;
        r.pro_rows = vec![pro1, pro2];
        r.con_rows = vec![con1, con2];
        r.next_id = 20;
        r
    }
}

pub const SHORTHAND: [(&str, &str); 16] = [
    ("→", "leads to"),
    ("↑", "increases"),
    ("↓", "decreases"),
    ("∴", "therefore"),
    ("≠", "not / no link"),
    ("Δ", "change"),
    ("w/", "with"),
    ("b/c", "because"),
    ("gov", "government"),
    ("$", "money / economy"),
    ("✚", "health"),
    ("🌍", "environment"),
    ("⚖️", "weigh this"),
    ("⭐", "key argument"),
    ("💤", "dropped"),
    ("➜", "extend ➜"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_is_empty_and_sample_is_not() {
        assert!(RoundData::default().is_empty());
        assert!(!RoundData::sample().is_empty());
    }

    #[test]
    fn whitespace_alone_is_empty_but_any_content_is_not() {
        let mut round = RoundData {
            topic: "  \n ".to_owned(),
            ..RoundData::default()
        };
        round.pro_rows[0].cells[0].text = " ".to_owned();
        assert!(round.is_empty());

        round.con_rows[0].cells[1].text = "note".to_owned();
        assert!(!round.is_empty());
    }

    #[test]
    fn marks_titles_and_notes_count_as_content() {
        let mut by_mark = RoundData::default();
        by_mark.pro_rows[0].cells[0].mark = Mark::Dropped;
        assert!(!by_mark.is_empty());

        let mut by_title = RoundData::default();
        by_title.con_rows[0].title = "C1".to_owned();
        assert!(!by_title.is_empty());

        let by_rfd = RoundData {
            rfd: "Pro wins on weighing".to_owned(),
            ..RoundData::default()
        };
        assert!(!by_rfd.is_empty());
    }

    fn saved_json(round: &RoundData) -> serde_json::Value {
        serde_json::to_value(round).unwrap_or(serde_json::Value::Null)
    }

    fn load(value: &serde_json::Value) -> Result<RoundData, serde_json::Error> {
        RoundData::from_saved_json(&serde_json::to_string(value).unwrap_or_default())
    }

    #[test]
    fn missing_any_single_top_level_field_loads_with_rest_intact() {
        let mut sample = RoundData::sample();
        sample.rfd = "Pro wins on weighing".to_owned();
        sample.crossfire_notes = "Con conceded 9%".to_owned();
        sample.winner = Some(Side::Pro);
        let fields = [
            "topic",
            "pro_team",
            "con_team",
            "speaking_order",
            "pro_rows",
            "con_rows",
            "crossfire_notes",
            "winner",
            "rfd",
            "next_id",
        ];
        for field in fields {
            let mut value = saved_json(&sample);
            if let Some(obj) = value.as_object_mut() {
                obj.remove(field);
            }
            let Ok(loaded) = load(&value) else {
                panic!("save missing {field} must still load");
            };
            if field != "topic" {
                assert_eq!(
                    loaded.topic, sample.topic,
                    "topic lost when {field} missing"
                );
            }
            if field != "rfd" {
                assert_eq!(loaded.rfd, sample.rfd, "rfd lost when {field} missing");
            }
            if field != "pro_rows" {
                assert_eq!(loaded.pro_rows[0].title, sample.pro_rows[0].title);
                assert_eq!(loaded.pro_rows[0].cells.len(), SPEECH_COLS.len());
            }
            // Repaired or not, next_id must stay usable for new rows.
            let max_id = loaded
                .pro_rows
                .iter()
                .chain(&loaded.con_rows)
                .map(|r| r.id)
                .max()
                .unwrap_or(0);
            assert!(
                loaded.next_id > max_id,
                "next_id unusable when {field} missing"
            );
        }
    }

    #[test]
    fn missing_nested_fields_default_instead_of_failing_the_round() {
        // Row without title, cell without mark, round without next_id.
        let json = r#"{"topic":"T","pro_rows":[{"id":7,"cells":[{"text":"kept"}]}]}"#;
        let Ok(loaded) = RoundData::from_saved_json(json) else {
            panic!("partially-missing save must load");
        };
        assert_eq!(loaded.topic, "T");
        assert_eq!(loaded.pro_rows[0].title, "");
        assert_eq!(loaded.pro_rows[0].cells[0].text, "kept");
        assert_eq!(loaded.pro_rows[0].cells[0].mark, Mark::None);
        assert!(loaded.next_id > 7);
    }

    #[test]
    fn rows_with_wrong_cell_counts_are_normalised_not_panicked_on() {
        let cells = |n: usize| -> Vec<serde_json::Value> {
            (0..n)
                .map(|i| serde_json::json!({ "text": format!("c{i}") }))
                .collect()
        };
        let Ok(wide) = serde_json::from_value::<RowData>(serde_json::json!({
            "id": 1, "title": "wide", "cells": cells(10),
        })) else {
            panic!("over-wide rows must load");
        };
        let Ok(narrow) = serde_json::from_value::<RowData>(serde_json::json!({
            "id": 2, "title": "narrow", "cells": cells(3),
        })) else {
            panic!("short rows must load");
        };
        assert_eq!(wide.cells[7].text, "c7", "extra cells are dropped on load");
        assert_eq!(narrow.cells[2].text, "c2", "short rows are padded on load");
        let Some(round) = RoundData {
            pro_rows: vec![wide, narrow],
            ..RoundData::default()
        }
        .try_normalised() else {
            panic!("ordinary ids must normalise");
        };
        assert_eq!(round.pro_rows[0].cells.len(), SPEECH_COLS.len());
        assert_eq!(round.pro_rows[0].cells[7].text, "c7");
        assert_eq!(round.pro_rows[1].cells.len(), SPEECH_COLS.len());
        assert_eq!(round.pro_rows[1].cells[2].text, "c2");
        assert_eq!(round.pro_rows[1].cells[7].text, "");
    }

    #[test]
    fn duplicate_row_ids_are_repaired_and_next_id_clamped_above_them() {
        let Some(round) = RoundData {
            pro_rows: vec![RowData::new(2, "a"), RowData::new(2, "b")],
            con_rows: vec![RowData::new(2, "c"), RowData::new(9, "d")],
            next_id: 1,
            ..RoundData::default()
        }
        .try_normalised() else {
            panic!("ordinary ids must normalise");
        };
        let ids: Vec<u32> = round
            .pro_rows
            .iter()
            .chain(&round.con_rows)
            .map(|r| r.id)
            .collect();
        let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate ids remain: {ids:?}");
        assert_eq!(round.pro_rows[0].id, 2, "first occurrence keeps its id");
        assert!(round.next_id > *ids.iter().max().unwrap_or(&0));
    }

    #[test]
    fn max_row_ids_take_the_unreadable_save_path_not_duplicate_ids() {
        // No u32 exists above MAX, so neither a duplicate repair nor a
        // usable next_id is possible: the save must be rejected (and
        // preserved by the recovery path), never loaded with duplicates.
        let duplicate_max = r#"{"pro_rows":[{"id":4294967295},{"id":4294967295}]}"#;
        assert!(RoundData::from_saved_json(duplicate_max).is_err());
        let lone_max = r#"{"pro_rows":[{"id":4294967295}]}"#;
        assert!(RoundData::from_saved_json(lone_max).is_err());
        // One below the ceiling still loads, with next_id exactly at it.
        let Ok(loaded) = RoundData::from_saved_json(r#"{"pro_rows":[{"id":4294967294}]}"#) else {
            panic!("MAX-1 save must load");
        };
        assert_eq!(loaded.next_id, u32::MAX);
    }

    #[test]
    fn autosave_revision_only_advances_over_the_revision_this_tab_knows() {
        // Legacy save (no revision key) and a tab that just wrote both
        // count as being in step with storage.
        assert_eq!(stored_revision(None), Some(0));
        assert_eq!(stored_revision(Some("7")), Some(7));
        assert_eq!(next_revision(0, 0), Some(1));
        assert_eq!(next_revision(7, 7), Some(8));
        // Another tab saved first (newer), or the counter went backwards
        // (older tab / removed key): refuse rather than overwrite.
        assert_eq!(next_revision(0, 1), None);
        assert_eq!(next_revision(7, 3), None);
        // A corrupt counter is unknown, never silently reset to 0.
        assert_eq!(stored_revision(Some("banana")), None);
        assert_eq!(stored_revision(Some("")), None);
    }

    #[test]
    fn unparseable_or_wrongly_typed_saves_are_errors_not_blank_rounds() {
        assert!(RoundData::from_saved_json("{not json").is_err());
        assert!(RoundData::from_saved_json("{\"topic\":42}").is_err());
        assert!(RoundData::from_saved_json("[1,2,3]").is_err());
        assert!(RoundData::from_saved_json("{\"winner\":42}").is_err());
    }

    #[test]
    fn legacy_winner_strings_load_as_sides_and_unknown_ones_as_no_winner() {
        for (raw, expected) in [
            ("Pro", Some(Side::Pro)),
            ("Con", Some(Side::Con)),
            ("", None),
            ("pro", None),
            ("Banana", None),
        ] {
            let json = format!(r#"{{"winner":"{raw}"}}"#);
            let Ok(loaded) = RoundData::from_saved_json(&json) else {
                panic!("winner {raw:?} save must load");
            };
            assert_eq!(loaded.winner, expected, "winner {raw:?}");
            // No winner means no phantom vote: the round is still empty
            // and the decision summary shows the blank line.
            assert_eq!(loaded.is_empty(), expected.is_none(), "winner {raw:?}");
            let label = expected.map_or("—", Side::label);
            assert!(
                loaded.decision_text().contains(&format!("Winner: {label}")),
                "winner {raw:?}"
            );
            // Re-saving keeps the legacy string form, normalised.
            let saved = serde_json::to_string(&loaded).unwrap_or_default();
            let normalised = expected.map_or("", Side::label);
            assert!(
                saved.contains(&format!(r#""winner":"{normalised}""#)),
                "winner {raw:?} saved as {saved}"
            );
        }
        // Absent and null winners are also no winner.
        for json in ["{}", r#"{"winner":null}"#] {
            let Ok(loaded) = RoundData::from_saved_json(json) else {
                panic!("{json} must load");
            };
            assert_eq!(loaded.winner, None, "{json}");
        }
    }

    // GUIDE steps: 3 = Pro Rebuttal, 4 = Con Rebuttal, 5 = crossfire
    // after it, 7 = Con Summary, 8 = Grand Crossfire, 10 = Con Final Focus.
    fn pro_round(answer_col: Option<usize>) -> RoundData {
        let mut row = RowData::new(1, "C1: Oceans");
        row.cells[0].text = "Plastic harms oceans".to_owned();
        if let Some(col) = answer_col {
            row.cells[col].text = "an answer".to_owned();
        }
        RoundData {
            pro_rows: vec![row],
            con_rows: Vec::new(),
            ..RoundData::default()
        }
    }

    fn con_round(answer_col: Option<usize>) -> RoundData {
        let mut row = RowData::new(1, "");
        row.cells[1].text = "Ban costs small shops".to_owned();
        if let Some(col) = answer_col {
            row.cells[col].text = "an answer".to_owned();
        }
        RoundData {
            pro_rows: Vec::new(),
            con_rows: vec![row],
            ..RoundData::default()
        }
    }

    #[test]
    fn no_dropped_hints_before_that_sides_rebuttal_is_over() {
        // Pro is answerable by Con, whose rebuttal is GUIDE step 4.
        for step in [0, 1, 4] {
            assert!(
                pro_round(None).dropped_arguments(step).is_empty(),
                "Pro flagged at guide step {step}, before Con Rebuttal ended"
            );
        }
        // Con is answerable by Pro, whose rebuttal is GUIDE step 3.
        for step in [0, 1, 3] {
            assert!(
                con_round(None).dropped_arguments(step).is_empty(),
                "Con flagged at guide step {step}, before Pro Rebuttal ended"
            );
        }
    }

    #[test]
    fn unanswered_after_rebuttal_is_flagged_for_both_sides() {
        let pro = pro_round(None).dropped_arguments(5);
        assert_eq!(pro.len(), 1);
        assert_eq!(pro[0].owner, Side::Pro);
        assert_eq!(pro[0].answerer, Side::Con);
        assert_eq!(pro[0].name, "C1: Oceans");

        // Untitled Con row falls back to the argument's own text.
        let con = con_round(None).dropped_arguments(4);
        assert_eq!(con.len(), 1);
        assert_eq!(con[0].owner, Side::Con);
        assert_eq!(con[0].answerer, Side::Pro);
        assert_eq!(con[0].name, "Ban costs small shops");

        // Whitespace is not an answer; blank origins are not arguments.
        let mut whitespace = pro_round(Some(3));
        whitespace.pro_rows[0].cells[3].text = "  \n ".to_owned();
        assert_eq!(whitespace.dropped_arguments(10).len(), 1);
        let blank = RoundData {
            pro_rows: vec![RowData::new(1, "titled but unwritten")],
            con_rows: Vec::new(),
            ..RoundData::default()
        };
        assert!(blank.dropped_arguments(10).is_empty());
    }

    #[test]
    fn answers_outside_rebuttal_column_count_as_answered() {
        // Pro row answered in Con Constructive (1), Rebuttal (3),
        // Summary (5), or Final Focus (7) is never dropped.
        for (answer_col, step) in [(1, 10), (3, 10), (5, 8), (7, 10)] {
            assert!(
                pro_round(Some(answer_col))
                    .dropped_arguments(step)
                    .is_empty(),
                "Pro answer in column {answer_col} ignored at step {step}"
            );
        }
        // Con row answered in Pro Rebuttal (2) or Pro Summary (4).
        for (answer_col, step) in [(2, 10), (4, 7)] {
            assert!(
                con_round(Some(answer_col))
                    .dropped_arguments(step)
                    .is_empty(),
                "Con answer in column {answer_col} ignored at step {step}"
            );
        }
        // The owner's own later speeches are extensions, not answers:
        // a Pro row only extended in Pro Summary (4) is still dropped.
        assert_eq!(pro_round(Some(4)).dropped_arguments(10).len(), 1);
    }

    #[test]
    fn future_speech_answers_do_not_suppress_dropped_hints() {
        // At step 5 Con Rebuttal has ended, but Con Summary (5) and
        // Con Final Focus (7) have not happened: text typed ahead
        // there is not an answer yet.
        for answer_col in [5, 7] {
            assert_eq!(
                pro_round(Some(answer_col)).dropped_arguments(5).len(),
                1,
                "future Pro answer in column {answer_col} suppressed a dropped hint at step 5"
            );
        }
        // Same speech, once it is current, does count.
        assert!(pro_round(Some(5)).dropped_arguments(7).is_empty());
        assert!(pro_round(Some(7)).dropped_arguments(10).is_empty());
        // Con side, symmetric: Pro Final Focus (6) is future at step 4
        // and current at step 9.
        assert_eq!(con_round(Some(6)).dropped_arguments(4).len(), 1);
        assert!(con_round(Some(6)).dropped_arguments(9).is_empty());
    }

    #[test]
    fn explicit_dropped_mark_overrides_answer_text_and_rebuttal_gate() {
        // Answered in Con Rebuttal (3), yet judge marked it dropped:
        // the explicit mark wins at any step, even step 0.
        let mut round = pro_round(Some(3));
        round.pro_rows[0].cells[0].mark = Mark::Dropped;
        for step in [0, 5, 10] {
            let dropped = round.dropped_arguments(step);
            assert_eq!(dropped.len(), 1, "explicit 💤 ignored at step {step}");
            assert!(dropped[0].explicit);
            assert_eq!(dropped[0].name, "C1: Oceans");
        }
        // Without the mark the same answered row is never flagged.
        assert!(pro_round(Some(3)).dropped_arguments(10).is_empty());
    }

    #[test]
    fn inferred_drop_carries_extended_and_key_flags() {
        let mut round = pro_round(None);
        round.pro_rows[0].cells[4].text = "extend it".to_owned();
        round.pro_rows[0].cells[4].mark = Mark::Extended;
        round.pro_rows[0].cells[0].mark = Mark::Key;
        let dropped = round.dropped_arguments(10);
        assert_eq!(dropped.len(), 1);
        assert!(!dropped[0].explicit);
        assert!(dropped[0].extended);
        assert!(dropped[0].key);
    }

    #[test]
    fn con_first_swaps_speech_pairs_and_guide_but_keeps_canonical_cols() {
        let order = SpeakingOrder::ConFirst;
        assert_eq!(order.column_order(), [1, 0, 3, 2, 5, 4, 7, 6]);
        assert_eq!(SPEECH_COLS[order.column_order()[0]].side, Side::Con);
        let titles: Vec<&str> = order
            .column_order()
            .iter()
            .map(|&col| SPEECH_COLS[col].full)
            .collect();
        assert_eq!(titles[0], "Con Constructive");
        assert_eq!(titles[1], "Pro Constructive");
        assert_eq!(titles[7], "Pro Final Focus");
        let guide = order.guide();
        let walked: Vec<&str> = guide.iter().map(|g| g.title).collect();
        assert_eq!(walked[0], "Con Constructive");
        assert_eq!(walked[1], "Pro Constructive");
        assert_eq!(walked[3], "Con Rebuttal");
        assert_eq!(walked[4], "Pro Rebuttal");
        assert_eq!(walked[10], "Pro Final Focus");
        // Canonical flow columns travel with their speech.
        assert_eq!(guide[0].flow_col, Some(1));
        assert_eq!(guide[1].flow_col, Some(0));
        assert_eq!(guide[10].flow_col, Some(6));
        // Order-sensitive wording follows the new first/last speeches.
        assert!(guide[0].bubble.contains("First speech"));
        assert!(guide[10].bubble.contains("Last speech"));
        assert_eq!(SpeakingOrder::default(), SpeakingOrder::ProFirst);
        assert_eq!(GUIDE[0].title, "Pro Constructive");
    }

    #[test]
    fn guide_flowed_steps_share_speech_metadata_and_pair_swap_in_con_first() {
        // Issue #35: timer, headers and guide read one source of truth.
        for step in GUIDE {
            if let Some(col) = step.flow_col {
                assert_eq!(step.title, SPEECH_COLS[col].full);
                assert_eq!(step.side, Some(SPEECH_COLS[col].side));
                assert_eq!(step.minutes, SPEECH_COLS[col].minutes);
            }
        }
        for order in [SpeakingOrder::ProFirst, SpeakingOrder::ConFirst] {
            let walk = order.guide();
            let flowed: Vec<usize> = walk.iter().filter_map(|g| g.flow_col).collect();
            assert_eq!(flowed, order.column_order(), "{order:?}");
            assert_eq!(walk.iter().filter(|g| g.flow_col.is_none()).count(), 3);
        }
    }

    #[test]
    fn grid_column_places_each_canonical_column_once_per_order() {
        // Issue #26: cells render once in canonical order and are
        // sequenced only by this CSS grid column, so it must be a
        // permutation of 2..=9 (column 1 is the argument title) that
        // agrees with column_order in both speaking orders.
        for order in [SpeakingOrder::ProFirst, SpeakingOrder::ConFirst] {
            let mut placed: Vec<usize> = (0..SPEECH_COLS.len())
                .map(|col| order.grid_column(col))
                .collect();
            placed.sort_unstable();
            assert_eq!(
                placed,
                (2..=SPEECH_COLS.len() + 1).collect::<Vec<_>>(),
                "{order:?}"
            );
            for (pos, col) in order.column_order().iter().enumerate() {
                assert_eq!(order.grid_column(*col), pos + 2, "{order:?} col {col}");
            }
            assert_eq!(order.grid_column(SPEECH_COLS.len()), SPEECH_COLS.len() + 2);
        }
        assert_eq!(SpeakingOrder::ProFirst.grid_column(0), 2);
        assert_eq!(SpeakingOrder::ConFirst.grid_column(0), 3);
        assert_eq!(SpeakingOrder::ConFirst.grid_column(1), 2);
    }

    #[test]
    fn sheet_for_points_rebuttals_at_opponents_sheet_in_both_orders() {
        // Issue #24: every speech-(re)entry path derives the visible
        // sheet from this one mapping, so rebuttals must land on the
        // opponent's sheet regardless of speaking order.
        for order in [SpeakingOrder::ProFirst, SpeakingOrder::ConFirst] {
            for step in order.guide() {
                let expected = match step.title {
                    "Pro Rebuttal" => Some(Side::Con),
                    "Con Rebuttal" => Some(Side::Pro),
                    "Crossfire" | "Grand Crossfire" => None,
                    _ => step.side,
                };
                assert_eq!(sheet_for(&step), expected, "{order:?} {}", step.title);
            }
        }
    }

    #[test]
    fn speaking_order_persists_and_old_saves_default_to_pro_first() {
        let round = RoundData {
            speaking_order: SpeakingOrder::ConFirst,
            ..RoundData::default()
        };
        let json = serde_json::to_string(&round).unwrap_or_default();
        let Ok(loaded) = RoundData::from_saved_json(&json) else {
            panic!("round with speaking order must reload");
        };
        assert_eq!(loaded.speaking_order, SpeakingOrder::ConFirst);
        // A pre-#21 save has no field at all: it was Pro-first by force.
        let Ok(old) = RoundData::from_saved_json(r#"{"topic":"T"}"#) else {
            panic!("old save must load");
        };
        assert_eq!(old.speaking_order, SpeakingOrder::ProFirst);
        assert_eq!(old.topic, "T");
    }

    fn con_first_round(pro_answer: Option<usize>, con_answer: Option<usize>) -> RoundData {
        let mut pro = RowData::new(1, "C1: Oceans");
        pro.cells[0].text = "Plastic harms oceans".to_owned();
        if let Some(col) = pro_answer {
            pro.cells[col].text = "an answer".to_owned();
        }
        let mut con = RowData::new(2, "C1: Economy");
        con.cells[1].text = "Ban costs small shops".to_owned();
        if let Some(col) = con_answer {
            con.cells[col].text = "an answer".to_owned();
        }
        RoundData {
            speaking_order: SpeakingOrder::ConFirst,
            pro_rows: vec![pro],
            con_rows: vec![con],
            ..RoundData::default()
        }
    }

    #[test]
    fn con_first_dropped_arguments_follow_speaking_order() {
        // Con Rebuttal is guide step 3, Pro Rebuttal step 4 (Con-first).
        for step in [0, 3] {
            assert!(
                con_first_round(None, None)
                    .dropped_arguments(step)
                    .iter()
                    .all(|d| d.owner != Side::Pro),
                "Pro flagged at step {step}, before Con Rebuttal ended"
            );
        }
        for step in [0, 4] {
            assert!(
                con_first_round(None, None)
                    .dropped_arguments(step)
                    .iter()
                    .all(|d| d.owner != Side::Con),
                "Con flagged at step {step}, before Pro Rebuttal ended"
            );
        }
        let dropped = con_first_round(None, None).dropped_arguments(5);
        assert_eq!(dropped.len(), 2);
        // Pro's Constructive (canonical 0) speaks after Con's, so it can
        // answer Con — the old canonical `col > origin` check missed it.
        assert!(
            con_first_round(None, Some(0))
                .dropped_arguments(10)
                .iter()
                .all(|d| d.owner != Side::Con)
        );
        // Con's Constructive speaks before Pro's, so text there is
        // Con's own case, not an answer to Pro: Pro is still dropped.
        assert_eq!(
            con_first_round(Some(1), None).dropped_arguments(10).len(),
            2
        );
    }

    #[test]
    fn marked_arguments_first_mark_follows_speaking_order() {
        // Both Constructives marked on one row: the speech spoken first
        // in this round's order wins, not canonical column 0.
        for (speaking_order, expected) in [
            (SpeakingOrder::ProFirst, "Pro Constructive"),
            (SpeakingOrder::ConFirst, "Con Constructive"),
        ] {
            let mut row = RowData::new(1, "C1: Oceans");
            row.cells[0].mark = Mark::Key;
            row.cells[1].mark = Mark::Key;
            let round = RoundData {
                speaking_order,
                pro_rows: vec![row],
                con_rows: Vec::new(),
                ..RoundData::default()
            };
            let marked = round.marked_arguments(Mark::Key);
            assert_eq!(marked.len(), 1, "{speaking_order:?}");
            assert_eq!(marked[0].speech, expected, "{speaking_order:?}");
        }
    }

    #[test]
    fn marked_arguments_and_decision_text_include_key_extended_dropped() {
        let round = RoundData::sample();
        // Sample: pro2 origin ⭐, pro1 cells 3+4 ➜, con2 origin 💤.
        let key = round.marked_arguments(Mark::Key);
        assert_eq!(key.len(), 1);
        assert_eq!(key[0].owner, Side::Pro);
        assert_eq!(key[0].name, "C2: Health 🧸");
        assert_eq!(key[0].speech, "Pro Constructive");
        // One row per marked row, first marked speech wins.
        let extended = round.marked_arguments(Mark::Extended);
        assert_eq!(extended.len(), 1);
        assert_eq!(extended[0].name, "C1: Oceans 🌊");
        assert_eq!(extended[0].speech, "Con Rebuttal");
        let dropped = round.marked_arguments(Mark::Dropped);
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].owner, Side::Con);
        assert!(round.marked_arguments(Mark::None).is_empty());

        let text = round.decision_text();
        assert!(text.contains("Winner: —"), "{text}");
        assert!(
            text.contains("⭐ Key arguments:") && text.contains("C2: Health"),
            "{text}"
        );
        assert!(
            text.contains("➜ Extended arguments:") && text.contains("C1: Oceans"),
            "{text}"
        );
        assert!(
            text.contains("💤 Marked dropped:") && text.contains("C2: Alternatives"),
            "{text}"
        );
        // Unmarked round: no mark sections at all.
        let plain = RoundData::default().decision_text();
        assert!(!plain.contains("Key arguments"), "{plain}");
        assert!(!plain.contains("Extended arguments"), "{plain}");
        assert!(!plain.contains("Marked dropped"), "{plain}");
    }
}
