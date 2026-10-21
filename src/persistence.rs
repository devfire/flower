//! localStorage persistence: load, debounced autosave, flush hooks.

use std::cell::{Cell, RefCell};

use crate::model::{
    RoundData, STORAGE_BACKUP_KEY, STORAGE_KEY, STORAGE_REVISION_KEY, next_revision,
    stored_revision,
};
use crate::progress::{PROGRESS_STORAGE_KEY, RoundProgress};
use crate::state::RoundState;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};
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
    // Base for the cross-tab revision check in `write_snapshot`: the
    // revision stored alongside the round this tab is about to load.
    // An unreadable counter falls back to 0 here; the write-time re-read
    // still tells corrupt apart from legacy and refuses to overwrite.
    let loaded_revision = storage
        .as_ref()
        .and_then(|s| s.get_item(STORAGE_REVISION_KEY).ok())
        .and_then(|raw| stored_revision(raw.as_deref()))
        .unwrap_or(0);
    KNOWN_REVISION.with(|known| known.set(Some(loaded_revision)));
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
    /// Revision this tab loaded (or last wrote) under `STORAGE_KEY`
    /// (issue #18). `None` only after a cross-tab conflict was shown:
    /// the recovery notice blocks autosaves until the judge explicitly
    /// chooses to replace, and that one write is then allowed over
    /// whatever revision is stored.
    static KNOWN_REVISION: Cell<Option<u64>> = const { Cell::new(Some(0)) };
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
    let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        state.save_failed.set(true);
        return;
    };
    let Ok(json) = serde_json::to_string(&state.snapshot()) else {
        state.save_failed.set(true);
        return;
    };
    // Cross-tab guard (issue #18): never write over a revision this tab
    // did not load or write itself — another tab saved first.
    let stored = match storage.get_item(STORAGE_REVISION_KEY) {
        Ok(raw) => stored_revision(raw.as_deref()),
        Err(_) => None,
    };
    let next = match (KNOWN_REVISION.with(Cell::get), stored) {
        (Some(known), Some(current)) => next_revision(known, current),
        // Only reachable after a conflict was shown and the judge then
        // chose to replace (notice dismissed or round replaced): that
        // explicit write may go over whatever is stored.
        (None, Some(current)) => current.checked_add(1),
        (None, None) => Some(1),
        (Some(_), None) => None,
    };
    let Some(next) = next else {
        // Adopt the other tab's revision as this tab's base (or `None`
        // when unreadable, the explicit-replace marker above) so the
        // judge's dismissal write can proceed — but block every autosave
        // until then via the recovery notice.
        KNOWN_REVISION.with(|known| known.set(stored));
        state.save_failed.set(true);
        state.recovery_notice.set(Some(
            "⚠️ Another tab or window with this app open saved a newer version of this \
             round, so this tab stopped auto-saving to avoid overwriting it. Check the \
             other tab first — if this tab's version should win after all, tap the \
             button below to replace the newer save."
                .to_owned(),
        ));
        return;
    };
    // Revision first: a failed round write then leaves a bumped counter
    // (other tabs pause instead of overwriting), and this tab's known
    // revision already matches it, so its retry is not self-blocked.
    if storage
        .set_item(STORAGE_REVISION_KEY, &next.to_string())
        .is_err()
    {
        state.save_failed.set(true);
        return;
    }
    KNOWN_REVISION.with(|known| known.set(Some(next)));
    let saved = storage.set_item(STORAGE_KEY, &json).is_ok();
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
