//! Granular per-cell signals: each textarea owns an `RwSignal`, so typing
//! never re-creates DOM nodes or loses the caret. A plain `RoundData`
//! snapshot is rebuilt (untracked) only when saving / exporting.

use crate::model::*;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct CellState {
    pub text: RwSignal<String>,
    pub mark: RwSignal<Mark>,
}

#[derive(Clone)]
pub struct RowState {
    pub id: u32,
    pub title: RwSignal<String>,
    pub cells: Vec<CellState>,
}

impl RowState {
    fn from_data(data: &RowData) -> Self {
        Self {
            id: data.id,
            title: RwSignal::new(data.title.clone()),
            cells: data
                .cells
                .iter()
                .map(|c| CellState { text: RwSignal::new(c.text.clone()), mark: RwSignal::new(c.mark) })
                .collect(),
        }
    }
    fn to_data(&self) -> RowData {
        RowData {
            id: self.id,
            title: self.title.get_untracked(),
            cells: self
                .cells
                .iter()
                .map(|c| CellData { text: c.text.get_untracked(), mark: c.mark.get_untracked() })
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct RoundState {
    pub topic: RwSignal<String>,
    pub pro_team: RwSignal<String>,
    pub con_team: RwSignal<String>,
    pub pro_rows: RwSignal<Vec<RowState>>,
    pub con_rows: RwSignal<Vec<RowState>>,
    pub crossfire_notes: RwSignal<String>,
    pub winner: RwSignal<String>,
    pub rfd: RwSignal<String>,
    pub next_id: RwSignal<u32>,
}

impl RoundState {
    pub fn from_data(data: RoundData) -> Self {
        Self {
            topic: RwSignal::new(data.topic),
            pro_team: RwSignal::new(data.pro_team),
            con_team: RwSignal::new(data.con_team),
            pro_rows: RwSignal::new(data.pro_rows.iter().map(RowState::from_data).collect()),
            con_rows: RwSignal::new(data.con_rows.iter().map(RowState::from_data).collect()),
            crossfire_notes: RwSignal::new(data.crossfire_notes),
            winner: RwSignal::new(data.winner),
            rfd: RwSignal::new(data.rfd),
            next_id: RwSignal::new(data.next_id),
        }
    }

    pub fn snapshot(&self) -> RoundData {
        RoundData {
            topic: self.topic.get_untracked(),
            pro_team: self.pro_team.get_untracked(),
            con_team: self.con_team.get_untracked(),
            pro_rows: self.pro_rows.get_untracked().iter().map(RowState::to_data).collect(),
            con_rows: self.con_rows.get_untracked().iter().map(RowState::to_data).collect(),
            crossfire_notes: self.crossfire_notes.get_untracked(),
            winner: self.winner.get_untracked(),
            rfd: self.rfd.get_untracked(),
            next_id: self.next_id.get_untracked(),
        }
    }

    pub fn rows(&self, side: Side) -> RwSignal<Vec<RowState>> {
        match side {
            Side::Pro => self.pro_rows,
            Side::Con => self.con_rows,
        }
    }

    pub fn replace(&self, data: RoundData) {
        let fresh = Self::from_data(data);
        self.topic.set(fresh.topic.get_untracked());
        self.pro_team.set(fresh.pro_team.get_untracked());
        self.con_team.set(fresh.con_team.get_untracked());
        self.pro_rows.set(fresh.pro_rows.get_untracked());
        self.con_rows.set(fresh.con_rows.get_untracked());
        self.crossfire_notes.set(fresh.crossfire_notes.get_untracked());
        self.winner.set(fresh.winner.get_untracked());
        self.rfd.set(fresh.rfd.get_untracked());
        self.next_id.set(fresh.next_id.get_untracked());
    }

    pub fn add_row(&self, side: Side) {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        self.rows(side).update(|rows| rows.push(RowState::from_data(&RowData::new(id, ""))));
        save(self);
    }

    pub fn remove_row(&self, side: Side, id: u32) {
        self.rows(side).update(|rows| rows.retain(|r| r.id != id));
        save(self);
    }
}

pub fn load_initial() -> RoundData {
    let stored = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten());
    match stored {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => RoundData::default(),
    }
}

pub fn save(state: &RoundState) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
        && let Ok(json) = serde_json::to_string(&state.snapshot())
    {
        let _ = storage.set_item(STORAGE_KEY, &json);
    }
}

pub fn confirm(message: &str) -> bool {
    web_sys::window().is_some_and(|w| w.confirm_with_message(message).unwrap_or(false))
}

pub fn print_page() {
    if let Some(w) = web_sys::window() {
        let _ = w.print();
    }
}

pub fn copy_text(text: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().clipboard().write_text(text);
    }
}

/// Monotonic clock (ms) for the speech timer deadline. `performance.now()`
/// keeps ticking in background tabs, unlike a per-tick decrement.
pub fn now_ms() -> Option<f64> {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now())
}

/// Create/resume an `AudioContext` from a user gesture (timer Start), so
/// the expiry beep is not blocked by autoplay policy minutes later.
pub fn unlock_audio() -> Option<web_sys::AudioContext> {
    let ctx = web_sys::AudioContext::new().ok()?;
    let _ = ctx.resume();
    Some(ctx)
}

/// Expiry signal for a judge looking at the debaters, not the screen:
/// three short beeps plus vibration where the device supports it.
pub fn timer_expired_feedback(audio: Option<&web_sys::AudioContext>) {
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().vibrate_with_duration(300);
    }
    let owned;
    let ctx = match audio {
        Some(ctx) => ctx,
        None => match web_sys::AudioContext::new() {
            Ok(ctx) => {
                owned = ctx;
                &owned
            }
            Err(_) => return,
        },
    };
    let _ = ctx.resume();
    let start_at = ctx.current_time();
    for i in 0..3 {
        let (Ok(osc), Ok(gain)) = (ctx.create_oscillator(), ctx.create_gain()) else {
            continue;
        };
        osc.set_type(web_sys::OscillatorType::Sine);
        osc.frequency().set_value(880.0);
        let t = start_at + f64::from(i) * 0.28;
        let _ = gain.gain().set_value_at_time(0.15, t);
        let _ = gain.gain().set_value_at_time(0.0001, t + 0.18);
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&ctx.destination());
        let _ = osc.start_with_when(t);
        let _ = osc.stop_with_when(t + 0.2);
    }
}
