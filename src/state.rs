//! Granular per-cell signals: each textarea owns an `RwSignal`, so typing
//! never re-creates DOM nodes or loses the caret. A plain `RoundData`
//! snapshot is rebuilt (untracked) only when saving / exporting.

use std::cell::{Cell, RefCell};

use crate::model::*;
use crate::progress::{PROGRESS_STORAGE_KEY, RoundProgress};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};

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
                .map(|c| CellState {
                    text: RwSignal::new(c.text.clone()),
                    mark: RwSignal::new(c.mark),
                })
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
                .map(|c| CellData {
                    text: c.text.get_untracked(),
                    mark: c.mark.get_untracked(),
                })
                .collect(),
        }
    }
    /// Tracked twin of `to_data`: reading with `get()` subscribes the
    /// calling reactive scope, so derived views re-run on every edit.
    fn to_data_tracked(&self) -> RowData {
        RowData {
            id: self.id,
            title: self.title.get(),
            cells: self
                .cells
                .iter()
                .map(|c| CellData {
                    text: c.text.get(),
                    mark: c.mark.get(),
                })
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct RoundState {
    pub topic: RwSignal<String>,
    pub pro_team: RwSignal<String>,
    pub con_team: RwSignal<String>,
    pub speaking_order: RwSignal<SpeakingOrder>,
    pub pro_rows: RwSignal<Vec<RowState>>,
    pub con_rows: RwSignal<Vec<RowState>>,
    pub crossfire_notes: RwSignal<String>,
    pub winner: RwSignal<String>,
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
        Self {
            topic: RwSignal::new(data.topic),
            pro_team: RwSignal::new(data.pro_team),
            con_team: RwSignal::new(data.con_team),
            speaking_order: RwSignal::new(data.speaking_order),
            pro_rows: RwSignal::new(data.pro_rows.iter().map(RowState::from_data).collect()),
            con_rows: RwSignal::new(data.con_rows.iter().map(RowState::from_data).collect()),
            crossfire_notes: RwSignal::new(data.crossfire_notes),
            winner: RwSignal::new(data.winner),
            rfd: RwSignal::new(data.rfd),
            next_id: RwSignal::new(data.next_id),
            save_failed: RwSignal::new(false),
            recovery_notice: RwSignal::new(None),
        }
    }

    pub fn snapshot(&self) -> RoundData {
        RoundData {
            topic: self.topic.get_untracked(),
            pro_team: self.pro_team.get_untracked(),
            con_team: self.con_team.get_untracked(),
            speaking_order: self.speaking_order.get_untracked(),
            pro_rows: self
                .pro_rows
                .get_untracked()
                .iter()
                .map(RowState::to_data)
                .collect(),
            con_rows: self
                .con_rows
                .get_untracked()
                .iter()
                .map(RowState::to_data)
                .collect(),
            crossfire_notes: self.crossfire_notes.get_untracked(),
            winner: self.winner.get_untracked(),
            rfd: self.rfd.get_untracked(),
            next_id: self.next_id.get_untracked(),
        }
    }

    /// Tracked twin of `snapshot`, for derived views (dropped-argument
    /// hints) that must update live as the judge types. Never use it for
    /// saving/export: it subscribes the caller to every cell in the round.
    pub fn snapshot_tracked(&self) -> RoundData {
        RoundData {
            topic: self.topic.get(),
            pro_team: self.pro_team.get(),
            con_team: self.con_team.get(),
            speaking_order: self.speaking_order.get(),
            pro_rows: self
                .pro_rows
                .get()
                .iter()
                .map(RowState::to_data_tracked)
                .collect(),
            con_rows: self
                .con_rows
                .get()
                .iter()
                .map(RowState::to_data_tracked)
                .collect(),
            crossfire_notes: self.crossfire_notes.get(),
            winner: self.winner.get(),
            rfd: self.rfd.get(),
            next_id: self.next_id.get(),
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
        self.topic.set(fresh.topic.get_untracked());
        self.pro_team.set(fresh.pro_team.get_untracked());
        self.con_team.set(fresh.con_team.get_untracked());
        self.speaking_order
            .set(fresh.speaking_order.get_untracked());
        self.pro_rows.set(fresh.pro_rows.get_untracked());
        self.con_rows.set(fresh.con_rows.get_untracked());
        self.crossfire_notes
            .set(fresh.crossfire_notes.get_untracked());
        self.winner.set(fresh.winner.get_untracked());
        self.rfd.set(fresh.rfd.get_untracked());
        self.next_id.set(fresh.next_id.get_untracked());
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

/// A round plus, when the stored save could not be read, the notice the UI
/// must show. In that case `data` is blank but the raw save is preserved
/// under `STORAGE_BACKUP_KEY` and `save` stays blocked until the judge
/// dismisses the notice — a blank round never silently overwrites it.
pub struct LoadedRound {
    pub data: RoundData,
    /// Restored walk/timer/sheet state (issue #17). Fresh-round defaults
    /// when there is no readable progress save — including when the round
    /// itself was unreadable, so a blank replacement round never inherits
    /// a half-finished walk from the save it is replacing.
    pub progress: RoundProgress,
    pub recovery_notice: Option<String>,
}

pub fn load_initial() -> LoadedRound {
    let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
    let stored = storage
        .as_ref()
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten());
    // Progress is best-effort and independent of the round key: a judge
    // who only advanced speeches / burned prep (never typed) has no round
    // save yet, and a missing or unreadable progress save restarts the
    // walk but must never cost them their flow.
    let load_progress = || {
        storage
            .as_ref()
            .and_then(|s| s.get_item(PROGRESS_STORAGE_KEY).ok().flatten())
            .and_then(|json| RoundProgress::from_saved_json(&json).ok())
            .unwrap_or_default()
    };
    let Some(json) = stored else {
        return LoadedRound {
            data: RoundData::default(),
            progress: load_progress(),
            recovery_notice: None,
        };
    };
    if let Ok(data) = RoundData::from_saved_json(&json) {
        let progress = load_progress();
        LoadedRound {
            data,
            progress,
            recovery_notice: None,
        }
    } else {
        let backed_up = storage
            .as_ref()
            .is_some_and(|s| s.set_item(STORAGE_BACKUP_KEY, &json).is_ok());
        let recovery_notice = if backed_up {
            "⚠️ We couldn't read the round saved on this device, so we started blank — \
             but we did NOT overwrite it: a copy of the old save is still kept here. \
             When you're ready, tap the button below to start fresh and replace it."
                .to_owned()
        } else {
            "⚠️ We couldn't read the round saved on this device, and this browser \
             wouldn't let us keep a copy of it either. Saving is paused so nothing \
             is lost silently — tap the button below when you're ready to start fresh."
                .to_owned()
        };
        LoadedRound {
            data: RoundData::default(),
            progress: RoundProgress::default(),
            recovery_notice: Some(recovery_notice),
        }
    }
}

/// Delay between the last edit and the autosave write. Typing during a
/// speech fires `save` per keystroke; coalescing those calls means the
/// O(round) snapshot + JSON + `localStorage` write runs at most once per
/// pause instead of once per character (issue #12).
const SAVE_DEBOUNCE_MS: u32 = 400;

/// A scheduled-but-not-yet-written save. Dropping the `Timeout` cancels
/// it, so replacing the pending entry is what resets the debounce clock.
struct PendingSave {
    state: RoundState,
    // Never read: held only so dropping the entry cancels the timer.
    #[allow(dead_code)]
    timeout: Timeout,
}

/// Same debounce for progress (issue #17): the timer tick schedules this
/// four times a second, and the trailing write only happens on a pause,
/// a switch, or a page-hide flush — never per tick.
struct PendingProgressSave {
    state: RoundState,
    progress: RoundProgress,
    #[allow(dead_code)]
    timeout: Timeout,
}

type FlushListener = Closure<dyn FnMut(web_sys::Event)>;

thread_local! {
    static PENDING_SAVE: RefCell<Option<PendingSave>> = const { RefCell::new(None) };
    static PENDING_PROGRESS_SAVE: RefCell<Option<PendingProgressSave>> = const { RefCell::new(None) };
    static FLUSH_HOOKS_INSTALLED: Cell<bool> = const { Cell::new(false) };
    /// Kept alive for the page's lifetime: dropping a `Closure` removes
    /// the event listener it backs.
    static FLUSH_LISTENERS: RefCell<Vec<FlushListener>> = const { RefCell::new(Vec::new()) };
}

/// Schedule an autosave. Every call site already invokes this after each
/// mutation; it now only (re)arms a trailing timer — no snapshot, JSON,
/// or storage work happens per keystroke.
pub fn save(state: &RoundState) {
    // Never overwrite an unreadable save the judge hasn't been told about.
    if state.recovery_notice.get_untracked().is_some() {
        return;
    }
    install_flush_hooks();
    let scheduled = state.clone();
    let timeout = Timeout::new(SAVE_DEBOUNCE_MS, flush_save);
    PENDING_SAVE.with(|pending| {
        *pending.borrow_mut() = Some(PendingSave {
            state: scheduled,
            timeout,
        });
    });
}

/// Schedule a progress autosave under `PROGRESS_STORAGE_KEY`, with the
/// same recovery-block and debounce rules as `save`. `state` is only the
/// carrier for the recovery block and the save-failed flag.
pub fn save_progress(state: &RoundState, progress: RoundProgress) {
    if state.recovery_notice.get_untracked().is_some() {
        return;
    }
    install_flush_hooks();
    let scheduled = state.clone();
    let timeout = Timeout::new(SAVE_DEBOUNCE_MS, flush_save);
    PENDING_PROGRESS_SAVE.with(|pending| {
        *pending.borrow_mut() = Some(PendingProgressSave {
            state: scheduled,
            progress,
            timeout,
        });
    });
}

/// Write any pending autosave now. Called by the debounce timer and when
/// the page is about to go away (`pagehide`, window `blur`, tab hidden),
/// so a debounced edit still survives an abrupt tab close.
pub fn flush_save() {
    let pending = PENDING_SAVE.with(|pending| pending.borrow_mut().take());
    if let Some(pending) = pending {
        write_snapshot(&pending.state);
    }
    let pending = PENDING_PROGRESS_SAVE.with(|pending| pending.borrow_mut().take());
    if let Some(pending) = pending {
        write_progress(&pending.state, pending.progress);
    }
}

fn write_progress(state: &RoundState, progress: RoundProgress) {
    if state.recovery_notice.get_untracked().is_some() {
        return;
    }
    let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
    let saved = match (storage, serde_json::to_string(&progress)) {
        (Some(storage), Ok(json)) => storage.set_item(PROGRESS_STORAGE_KEY, &json).is_ok(),
        (Some(_), Err(_)) | (None, _) => false,
    };
    // Only ever raise the flag here: a successful progress write does not
    // prove the (separately debounced) round snapshot is saved.
    if !saved {
        state.save_failed.set(true);
    }
}

fn write_snapshot(state: &RoundState) {
    // Re-checked at write time: a notice shown after scheduling must
    // still block the write.
    if state.recovery_notice.get_untracked().is_some() {
        return;
    }
    let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
    let saved = match (storage, serde_json::to_string(&state.snapshot())) {
        (Some(storage), Ok(json)) => storage.set_item(STORAGE_KEY, &json).is_ok(),
        (Some(_), Err(_)) | (None, _) => false,
    };
    state.save_failed.set(!saved);
}

fn install_flush_hooks() {
    if FLUSH_HOOKS_INSTALLED.with(Cell::get) {
        return;
    }
    FLUSH_HOOKS_INSTALLED.with(|installed| installed.set(true));
    let Some(window) = web_sys::window() else {
        return;
    };
    add_flush_listener(
        window.unchecked_ref(),
        "pagehide",
        Closure::new(|_: web_sys::Event| flush_save()),
    );
    add_flush_listener(
        window.unchecked_ref(),
        "blur",
        Closure::new(|_: web_sys::Event| flush_save()),
    );
    if let Some(document) = window.document() {
        add_flush_listener(
            document.unchecked_ref(),
            "visibilitychange",
            Closure::new(|_: web_sys::Event| {
                if web_sys::window()
                    .and_then(|w| w.document())
                    .is_some_and(|doc| doc.hidden())
                {
                    flush_save();
                }
            }),
        );
    }
}

fn add_flush_listener(target: &web_sys::EventTarget, name: &str, closure: FlushListener) {
    let _ = target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
    FLUSH_LISTENERS.with(|listeners| listeners.borrow_mut().push(closure));
}

pub fn print_page() {
    if let Some(w) = web_sys::window() {
        let _ = w.print();
    }
}

/// Copy `text` to the clipboard; `true` only when the browser confirms
/// the write, so the UI can show ✓ or an error instead of guessing
/// (the promise rejection — blocked permission, unfocused document —
/// was previously swallowed).
pub async fn copy_text(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let promise = window.navigator().clipboard().write_text(text);
    wasm_bindgen_futures::JsFuture::from(promise).await.is_ok()
}

/// Monotonic clock (ms) for the speech timer deadline. `performance.now()`
/// keeps ticking in background tabs, unlike a per-tick decrement.
pub fn now_ms() -> Option<f64> {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
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
