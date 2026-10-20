//! Round data + the speech-by-speech guide.
//!
//! Flowing rules encoded here come from the NSDA "Intro to Flowing" judge
//! training: one page per side, one column per speech, responses written
//! directly to the right of the argument they answer, a blank box means the
//! argument was dropped, and an arrow means it was extended.

use serde::{Deserialize, Serialize};

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

/// The 8 speeches that get flowed, left → right. Crossfires are not flowed.
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

pub const GUIDE: [GuideStep; 11] = [
    GuideStep {
        title: "Pro Constructive",
        side: Some(Side::Pro),
        minutes: 4,
        flow_col: Some(0),
        bubble: "First speech! Just catch the big ideas — nobody flows every word. ♡",
        listen_for: "“Our first contention is…” — each contention is a new row on the Pro sheet.",
        do_it: "Start one row per contention. Write 3–6 words: the claim + why it matters (the “impact”).",
        watch_out: "Don't write full sentences. Tone + signposting tell you what matters most.",
    },
    GuideStep {
        title: "Con Constructive",
        side: Some(Side::Con),
        minutes: 4,
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
        title: "Pro Rebuttal",
        side: Some(Side::Pro),
        minutes: 4,
        flow_col: Some(2),
        bubble: "Now they answer each other! Write answers in the box directly RIGHT of the argument. ➜",
        listen_for: "“They say…, but…” — Pro is now answering the Con case.",
        do_it: "On the Con sheet, write Pro's answer in the Pro Reb column, on the same row as the Con argument it answers.",
        watch_out: "A Con row with a blank Pro Reb box = that argument went unanswered. Leave it blank on purpose!",
    },
    GuideStep {
        title: "Con Rebuttal",
        side: Some(Side::Con),
        minutes: 4,
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
        title: "Pro Summary",
        side: Some(Side::Pro),
        minutes: 3,
        flow_col: Some(4),
        bubble: "Summaries = “extend” time. If they carry an argument forward, draw your arrow ➜!",
        listen_for: "“Extend our…” and picking their best 1–2 arguments to focus on.",
        do_it: "Write the extension in the Pro Sum column on the same row, and tap ➜ extended on it.",
        watch_out: "Arguments NOT mentioned here are usually gone for good — Pro can't revive them later.",
    },
    GuideStep {
        title: "Con Summary",
        side: Some(Side::Con),
        minutes: 3,
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
        title: "Pro Final Focus",
        side: Some(Side::Pro),
        minutes: 2,
        flow_col: Some(6),
        bubble: "Final Focus = “judge, vote for us because…”. Listen for weighing! ⚖️",
        listen_for: "Weighing words: bigger, sooner, more likely, “even if they win…”.",
        do_it: "Write their #1 reason in the Pro FF column on that argument's row. Star ⭐ it.",
        watch_out: "No brand-new arguments should appear here — if it's not on the flow already, be skeptical.",
    },
    GuideStep {
        title: "Con Final Focus",
        side: Some(Side::Con),
        minutes: 2,
        flow_col: Some(7),
        bubble: "Last speech! After this, the ballot below walks you through deciding.",
        listen_for: "Con's #1 reason + their weighing against Pro's #1 reason.",
        do_it: "Fill the Con FF column, star ⭐ their key argument, then scroll to “Your Decision”.",
        watch_out: "You don't need a perfect flow — decide from the rows with answers, extensions and weighing.",
    },
];

/// `#[serde(default)]`: a save missing a field loads with that field
/// defaulted instead of failing the whole round (see `RoundData::normalised`).
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CellData {
    pub text: String,
    pub mark: Mark,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RowData {
    pub id: u32,
    pub title: String,
    /// Invariant: exactly `SPEECH_COLS.len()` cells, one per speech.
    /// Deserialised data may violate it; `RoundData::normalised` restores it
    /// before any row reaches the UI.
    pub cells: Vec<CellData>,
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
            cells: (0..SPEECH_COLS.len())
                .map(|_| CellData::default())
                .collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoundData {
    pub topic: String,
    pub pro_team: String,
    pub con_team: String,
    pub pro_rows: Vec<RowData>,
    pub con_rows: Vec<RowData>,
    pub crossfire_notes: String,
    pub winner: String,
    pub rfd: String,
    pub next_id: u32,
}

impl Default for RoundData {
    fn default() -> Self {
        Self {
            topic: String::new(),
            pro_team: String::new(),
            con_team: String::new(),
            pro_rows: vec![RowData::new(1, ""), RowData::new(2, "")],
            con_rows: vec![RowData::new(3, ""), RowData::new(4, "")],
            crossfire_notes: String::new(),
            winner: String::new(),
            rfd: String::new(),
            next_id: 5,
        }
    }
}

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
    /// Speech (`SPEECH_COLS` full name) of the first cell with the mark.
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
    /// Parse a saved round, repairing anything a truncated, older, or
    /// hand-edited save may contain. Only genuinely unparseable JSON (or a
    /// field of the wrong type) is an `Err`; callers must preserve the raw
    /// text in that case rather than overwrite it.
    ///
    /// # Errors
    /// Returns the `serde_json` error when `json` is not a readable round.
    pub fn from_saved_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json).map(Self::normalised)
    }

    /// Restore the invariants the UI relies on: exactly `SPEECH_COLS.len()`
    /// cells per row (pad short rows, drop extra cells), unique row ids
    /// (later duplicates get fresh ids), and `next_id` above every row id
    /// so a future row can never reuse one.
    #[must_use]
    pub fn normalised(mut self) -> Self {
        use std::collections::HashSet;
        // Fresh ids start above every existing id, so a repaired duplicate
        // can never collide with a row not visited yet.
        let mut fresh = self
            .pro_rows
            .iter()
            .chain(&self.con_rows)
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut seen = HashSet::new();
        for row in self.pro_rows.iter_mut().chain(self.con_rows.iter_mut()) {
            if !seen.insert(row.id) {
                row.id = fresh;
                seen.insert(fresh);
                fresh = fresh.saturating_add(1);
            }
            row.cells.resize_with(SPEECH_COLS.len(), CellData::default);
        }
        let max_id = self
            .pro_rows
            .iter()
            .chain(&self.con_rows)
            .map(|r| r.id)
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max_id.saturating_add(1));
        self
    }

    /// True when nothing a judge typed or marked would be lost by replacing
    /// this round (whitespace alone does not count).
    pub fn is_empty(&self) -> bool {
        let blank = |s: &str| s.trim().is_empty();
        let row_blank = |r: &RowData| {
            blank(&r.title)
                && r.cells
                    .iter()
                    .all(|c| blank(&c.text) && c.mark == Mark::None)
        };
        blank(&self.topic)
            && blank(&self.pro_team)
            && blank(&self.con_team)
            && blank(&self.crossfire_notes)
            && blank(&self.winner)
            && blank(&self.rfd)
            && self.pro_rows.iter().all(row_blank)
            && self.con_rows.iter().all(row_blank)
    }

    /// Arguments that look dropped at guide step `guide_step` (an index
    /// into `GUIDE`). Pure so the ballot hints can be unit-tested and the
    /// view only has to feed it a tracked snapshot.
    ///
    /// Inferred drop: opponent's rebuttal is already over and *every*
    /// opponent-speech box on the row is blank — an answer in
    /// Constructive, Summary, or Final Focus counts just as much as one
    /// in Rebuttal. Explicit drop: the judge marked a cell 💤; that is
    /// surfaced immediately, rebuttal gate and answer text notwithstanding,
    /// because the judge — not the text heuristic — decided it.
    #[must_use]
    pub fn dropped_arguments(&self, guide_step: usize) -> Vec<DroppedArgument> {
        // Highest flow column whose speech ended before the current step.
        let completed = GUIDE
            .iter()
            .take(guide_step.min(GUIDE.len()))
            .filter_map(|g| g.flow_col)
            .max();
        let mut dropped = Vec::new();
        // (rows, owner, origin column, opponent's rebuttal column)
        for (rows, owner, origin_col, rebuttal_col) in [
            (&self.pro_rows, Side::Pro, 0usize, 3usize),
            (&self.con_rows, Side::Con, 1usize, 2usize),
        ] {
            let answerer = match owner {
                Side::Pro => Side::Con,
                Side::Con => Side::Pro,
            };
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
                // Inferred only: this side has not had its rebuttal yet.
                if completed.is_none_or(|done| rebuttal_col > done) {
                    continue;
                }
                let origin = row.cells.get(origin_col).map_or("", |c| c.text.trim());
                if origin.is_empty() {
                    continue;
                }
                // Any opponent box counts, even in the speech happening
                // now: text there means the answer was given.
                let has_answer = row.cells.iter().enumerate().any(|(col, cell)| {
                    col > origin_col
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
    /// summary. `Mark::None` yields nothing — it is absence of a mark.
    #[must_use]
    pub fn marked_arguments(&self, mark: Mark) -> Vec<MarkedArgument> {
        if mark == Mark::None {
            return Vec::new();
        }
        let mut marked = Vec::new();
        for (rows, owner, origin_col) in [
            (&self.pro_rows, Side::Pro, 0usize),
            (&self.con_rows, Side::Con, 1usize),
        ] {
            for row in rows {
                if let Some(speech) = row
                    .cells
                    .iter()
                    .enumerate()
                    .find(|(_, c)| c.mark == mark)
                    .and_then(|(col, _)| SPEECH_COLS.get(col))
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
        let winner = if self.winner.is_empty() {
            "—"
        } else {
            &self.winner
        };
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

    pub fn sample() -> Self {
        let mut r = Self {
            topic: "Resolved: The United States should ban single-use plastics.".to_owned(),
            pro_team: "Pro — Maya & Jun".to_owned(),
            con_team: "Con — Leo & Priya".to_owned(),
            ..Self::default()
        };
        let mut pro1 = RowData::new(10, "C1: Oceans 🌊");
        pro1.cells[0].text = "Plastic → oceans, harms sea life".to_owned();
        pro1.cells[3].text = "“Recycling fixes it” — only 9% recycled".to_owned();
        pro1.cells[3].mark = Mark::Extended;
        pro1.cells[4].text = "➜ Extend: harm is permanent".to_owned();
        pro1.cells[4].mark = Mark::Extended;
        let mut pro2 = RowData::new(11, "C2: Health 🧸");
        pro2.cells[0].text = "Microplastics → drinking water".to_owned();
        pro2.cells[0].mark = Mark::Key;
        let mut con1 = RowData::new(12, "C1: Economy 💰");
        con1.cells[1].text = "Ban → costs ↑ for small shops".to_owned();
        con1.cells[2].text = "Shops adapt — paper is cheap now".to_owned();
        let mut con2 = RowData::new(13, "C2: Alternatives 🌱");
        con2.cells[1].text = "Compostables replace plastic".to_owned();
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

pub const STORAGE_KEY: &str = "flower-round-v1";
/// Where an unreadable save is copied before the app starts blank, so the
/// raw round is never silently overwritten (issue #2).
pub const STORAGE_BACKUP_KEY: &str = "flower-round-v1-unreadable-backup";

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
        sample.winner = "Pro".to_owned();
        let fields = [
            "topic",
            "pro_team",
            "con_team",
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
        let mut wide = RowData::new(1, "wide");
        wide.cells = (0..10)
            .map(|i| CellData {
                text: format!("c{i}"),
                mark: Mark::None,
            })
            .collect();
        let mut narrow = RowData::new(2, "narrow");
        narrow.cells.truncate(3);
        narrow.cells[2].text = "last kept".to_owned();
        let round = RoundData {
            pro_rows: vec![wide, narrow],
            ..RoundData::default()
        }
        .normalised();
        assert_eq!(round.pro_rows[0].cells.len(), SPEECH_COLS.len());
        assert_eq!(round.pro_rows[0].cells[7].text, "c7");
        assert_eq!(round.pro_rows[1].cells.len(), SPEECH_COLS.len());
        assert_eq!(round.pro_rows[1].cells[2].text, "last kept");
        assert_eq!(round.pro_rows[1].cells[7].text, "");
    }

    #[test]
    fn duplicate_row_ids_are_repaired_and_next_id_clamped_above_them() {
        let round = RoundData {
            pro_rows: vec![RowData::new(2, "a"), RowData::new(2, "b")],
            con_rows: vec![RowData::new(2, "c"), RowData::new(9, "d")],
            next_id: 1,
            ..RoundData::default()
        }
        .normalised();
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
    fn unparseable_or_wrongly_typed_saves_are_errors_not_blank_rounds() {
        assert!(RoundData::from_saved_json("{not json").is_err());
        assert!(RoundData::from_saved_json("{\"topic\":42}").is_err());
        assert!(RoundData::from_saved_json("[1,2,3]").is_err());
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
