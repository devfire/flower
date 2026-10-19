//! Round data + the speech-by-speech guide.
//!
//! Flowing rules encoded here come from the NSDA "Intro to Flowing" judge
//! training: one page per side, one column per speech, responses written
//! directly to the right of the argument they answer, a blank box means the
//! argument was dropped, and an arrow means it was extended.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    pub fn emoji(self) -> &'static str {
        match self {
            Side::Pro => "🌸",
            Side::Con => "🍃",
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
    SpeechCol { full: "Pro Constructive", side: Side::Pro, minutes: 4 },
    SpeechCol { full: "Con Constructive", side: Side::Con, minutes: 4 },
    SpeechCol { full: "Pro Rebuttal", side: Side::Pro, minutes: 4 },
    SpeechCol { full: "Con Rebuttal", side: Side::Con, minutes: 4 },
    SpeechCol { full: "Pro Summary", side: Side::Pro, minutes: 3 },
    SpeechCol { full: "Con Summary", side: Side::Con, minutes: 3 },
    SpeechCol { full: "Pro Final Focus", side: Side::Pro, minutes: 2 },
    SpeechCol { full: "Con Final Focus", side: Side::Con, minutes: 2 },
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
        title: "Pro Constructive", side: Some(Side::Pro), minutes: 4, flow_col: Some(0),
        bubble: "First speech! Just catch the big ideas — nobody flows every word. ♡",
        listen_for: "“Our first contention is…” — each contention is a new row on the 🌸 Pro sheet.",
        do_it: "Start one row per contention. Write 3–6 words: the claim + why it matters (the “impact”).",
        watch_out: "Don't write full sentences. Tone + signposting tell you what matters most.",
    },
    GuideStep {
        title: "Con Constructive", side: Some(Side::Con), minutes: 4, flow_col: Some(1),
        bubble: "Flip to the 🍃 Con sheet — their case gets its own page, just like paper!",
        listen_for: "Con's own contentions. They may also start answering Pro — but their case comes first.",
        do_it: "One row per Con contention on the Con sheet, same as before. Claim + impact, short and sweet.",
        watch_out: "Keep the sheets separate in your head: pink rows = Pro's case, green rows = Con's case.",
    },
    GuideStep {
        title: "Crossfire", side: None, minutes: 3, flow_col: None,
        bubble: "Pencils down, relax! ☕ Crossfire is questions — you don't have to flow it.",
        listen_for: "A clear concession (“yes, that's true”) or a debater unable to answer.",
        do_it: "Only jot a note in the Crossfire Notes box below if something big is admitted.",
        watch_out: "Don't decide anything yet — crossfire alone doesn't win rounds, the flow does.",
    },
    GuideStep {
        title: "Pro Rebuttal", side: Some(Side::Pro), minutes: 4, flow_col: Some(2),
        bubble: "Now they answer each other! Write answers in the box directly RIGHT of the argument. ➜",
        listen_for: "“They say…, but…” — Pro is now answering the 🍃 Con case.",
        do_it: "On the Con sheet, write Pro's answer in the Pro Reb column, on the same row as the Con argument it answers.",
        watch_out: "A Con row with a blank Pro Reb box = that argument went unanswered. Leave it blank on purpose!",
    },
    GuideStep {
        title: "Con Rebuttal", side: Some(Side::Con), minutes: 4, flow_col: Some(3),
        bubble: "Same thing, other direction — Con answers Pro on the 🌸 Pro sheet. You’ve got this!",
        listen_for: "Con answering each Pro contention, and defending their own case.",
        do_it: "On the Pro sheet, fill the Con Reb column row-by-row. Blank box = dropped argument — tap 💤 on the original.",
        watch_out: "New evidence names/dates? Just write the author's last name, e.g. “Smith '24”.",
    },
    GuideStep {
        title: "Crossfire", side: None, minutes: 3, flow_col: None,
        bubble: "Second crossfire — breathe, stretch, sip water. 🌷",
        listen_for: "Either side clarifying what their strongest argument is.",
        do_it: "Star ⭐ anything on your flow that keeps coming up — it's probably deciding the round.",
        watch_out: "Speakers may sound confident about arguments that aren't on the flow. Trust your paper.",
    },
    GuideStep {
        title: "Pro Summary", side: Some(Side::Pro), minutes: 3, flow_col: Some(4),
        bubble: "Summaries = “extend” time. If they carry an argument forward, draw your arrow ➜!",
        listen_for: "“Extend our…” and picking their best 1–2 arguments to focus on.",
        do_it: "Write the extension in the Pro Sum column on the same row, and tap ➜ extended on it.",
        watch_out: "Arguments NOT mentioned here are usually gone for good — Pro can't revive them later.",
    },
    GuideStep {
        title: "Con Summary", side: Some(Side::Con), minutes: 3, flow_col: Some(5),
        bubble: "Con does the same — which of their arguments survive? 🍃",
        listen_for: "“Extend…” plus answers to Pro's summary choices.",
        do_it: "Fill the Con Sum column on both sheets where Con speaks to that row. Mark ➜ extended.",
        watch_out: "Compare: which side still has live, answered-for arguments? That picture is the ballot forming.",
    },
    GuideStep {
        title: "Grand Crossfire", side: None, minutes: 3, flow_col: None,
        bubble: "Everyone's in! Last questions — almost there, you're doing great. ✨",
        listen_for: "“Why should the judge vote for you?” — they'll name their voting issues.",
        do_it: "Write those 1–2 named reasons in Crossfire Notes. Compare them with your ⭐ rows.",
        watch_out: "Grand crossfire claims still need to exist on the flow to count.",
    },
    GuideStep {
        title: "Pro Final Focus", side: Some(Side::Pro), minutes: 2, flow_col: Some(6),
        bubble: "Final Focus = “judge, vote for us because…”. Listen for weighing! ⚖️",
        listen_for: "Weighing words: bigger, sooner, more likely, “even if they win…”.",
        do_it: "Write their #1 reason in the Pro FF column on that argument's row. Star ⭐ it.",
        watch_out: "No brand-new arguments should appear here — if it's not on the flow already, be skeptical.",
    },
    GuideStep {
        title: "Con Final Focus", side: Some(Side::Con), minutes: 2, flow_col: Some(7),
        bubble: "Last speech! After this, the ballot below walks you through deciding. 🌸",
        listen_for: "Con's #1 reason + their weighing against Pro's #1 reason.",
        do_it: "Fill the Con FF column, star ⭐ their key argument, then scroll to “Your Decision”.",
        watch_out: "You don't need a perfect flow — decide from the rows with answers, extensions and weighing.",
    },
];

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CellData {
    pub text: String,
    pub mark: Mark,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RowData {
    pub id: u32,
    pub title: String,
    /// Invariant: exactly `SPEECH_COLS.len()` cells, one per speech.
    pub cells: Vec<CellData>,
}

impl RowData {
    pub fn new(id: u32, title: &str) -> Self {
        Self {
            id,
            title: title.to_owned(),
            cells: (0..SPEECH_COLS.len()).map(|_| CellData::default()).collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
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

impl RoundData {
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
