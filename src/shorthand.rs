//! Shorthand-chip insertion (issue #8).
//!
//! Chips used to append to whichever flow cell was focused last — even
//! while the judge was writing in Notes, the RFD, or a title —
//! and silently did nothing when no cell had ever been focused. Now every
//! text box registers itself as the insert target on focus, insertion
//! happens at the caret (replacing any selection), keyboard focus is
//! restored to that box with the caret after the symbol, and a chip tap
//! with no target shows a hint instead of doing nothing.

use crate::model::Side;
use crate::state::{RoundState, save};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Every text box a shorthand chip can write into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextTarget {
    FlowCell { side: Side, row_id: u32, col: usize },
    RowTitle { side: Side, row_id: u32 },
    Topic,
    ProTeam,
    ConTeam,
    Notes,
    Rfd,
}

impl TextTarget {
    /// Stable DOM id of the box. Only the interactive sheet renders ids:
    /// the print-only sheets reuse the same rows and must not duplicate
    /// them, or `get_element_by_id` could restore focus to a hidden box.
    #[must_use]
    pub fn id(self) -> String {
        match self {
            Self::FlowCell { side, row_id, col } => {
                format!("flow-{}-{row_id}-{col}", side.label().to_lowercase())
            }
            Self::RowTitle { side, row_id } => {
                format!("title-{}-{row_id}", side.label().to_lowercase())
            }
            Self::Topic => "field-topic".to_string(),
            Self::ProTeam => "field-pro-team".to_string(),
            Self::ConTeam => "field-con-team".to_string(),
            Self::Notes => "field-notes".to_string(),
            Self::Rfd => "notes-rfd".to_string(),
        }
    }

    fn signal(self, state: &RoundState) -> Option<RwSignal<String>> {
        match self {
            Self::FlowCell { side, row_id, col } => state
                .rows(side)
                .get_untracked()
                .iter()
                .find(|row| row.id == row_id)
                .and_then(|row| row.cells.get(col))
                .map(|cell| cell.text),
            Self::RowTitle { side, row_id } => state
                .rows(side)
                .get_untracked()
                .iter()
                .find(|row| row.id == row_id)
                .map(|row| row.title),
            Self::Topic => Some(state.topic),
            Self::ProTeam => Some(state.pro_team),
            Self::ConTeam => Some(state.con_team),
            Self::Notes => Some(state.notes),
            Self::Rfd => Some(state.rfd),
        }
    }
}

/// Shared chip-insertion state: the box last focused, plus the hint shown
/// when a chip is tapped before any box was ever focused.
#[derive(Clone, Copy)]
pub struct InsertCtx {
    pub target: RwSignal<Option<TextTarget>>,
    pub hint: RwSignal<Option<String>>,
}

impl InsertCtx {
    #[must_use]
    pub fn new() -> Self {
        Self {
            target: RwSignal::new(None),
            hint: RwSignal::new(None),
        }
    }

    /// A box gained focus: it becomes the chip target and any stale
    /// "tap a box first" hint no longer applies.
    pub fn focus(self, target: TextTarget) {
        self.target.set(Some(target));
        self.hint.set(None);
    }
}

fn floor_char_boundary(text: &str, idx: usize) -> usize {
    let mut idx = idx.min(text.len());
    while !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Insert `sym` into `text` at the byte-range selection (or at the end
/// when `selection` is `None`), replacing any selected range. A single
/// separating space is added when the character before the caret is not
/// whitespace, mirroring the old append behaviour. Returns the new text
/// and the byte offset the caret should sit at (just after the symbol).
#[must_use]
pub fn inserted_text(text: &str, selection: Option<(usize, usize)>, sym: &str) -> (String, usize) {
    let (raw_start, raw_end) = selection.unwrap_or((text.len(), text.len()));
    let start = floor_char_boundary(text, raw_start.min(raw_end));
    let end = floor_char_boundary(text, raw_start.max(raw_end));
    let needs_space = text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace());
    let insertion = if needs_space {
        format!(" {sym}")
    } else {
        sym.to_string()
    };
    let mut out = String::with_capacity(text.len() - (end - start) + insertion.len());
    out.push_str(&text[..start]);
    out.push_str(&insertion);
    out.push_str(&text[end..]);
    (out, start + insertion.len())
}

/// DOM selection offsets count UTF-16 code units; Rust strings index by
/// byte. Convert both ways so carets survive emoji (most shorthand, and
/// half the flow, is emoji).
fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut used = 0usize;
    for (idx, ch) in text.char_indices() {
        if used >= units {
            return idx;
        }
        used += ch.len_utf16();
        if used > units {
            // Offset fell inside a surrogate pair: caret before that char.
            return idx;
        }
    }
    text.len()
}

fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text[..floor_char_boundary(text, byte)]
        .chars()
        .map(char::len_utf16)
        .sum()
}

fn selection_utf16(el: &web_sys::Element) -> Option<(usize, usize)> {
    if let Some(area) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        let start = area.selection_start().ok()??;
        let end = area.selection_end().ok()??;
        Some((usize::try_from(start).ok()?, usize::try_from(end).ok()?))
    } else if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        let start = input.selection_start().ok()??;
        let end = input.selection_end().ok()??;
        Some((usize::try_from(start).ok()?, usize::try_from(end).ok()?))
    } else {
        None
    }
}

/// The caret/selection of the target box, as byte offsets into `text`.
/// A blurred box keeps its selection in the DOM, so reading it when the
/// chip is tapped gives the caret the judge actually left behind.
fn caret_for(id: &str, text: &str) -> Option<(usize, usize)> {
    let el = web_sys::window()?.document()?.get_element_by_id(id)?;
    let (start, end) = selection_utf16(&el)?;
    Some((utf16_to_byte(text, start), utf16_to_byte(text, end)))
}

/// Put keyboard focus back in the target box with the caret just after
/// the inserted symbol, so the judge can keep typing without tapping.
fn restore_focus(id: &str, text: &str, caret_byte: usize) {
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
    else {
        return;
    };
    let caret = u32::try_from(byte_to_utf16(text, caret_byte)).unwrap_or(u32::MAX);
    // Set the value directly as well as via the signal: the DOM node must
    // already show the new text when the caret is placed in it.
    if let Some(area) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        area.set_value(text);
        let _ = area.set_selection_range(caret, caret);
    } else if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_value(text);
        let _ = input.set_selection_range(caret, caret);
    }
    if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
        let _ = html.focus();
    }
}

pub fn insert_shorthand(state: &RoundState, ctx: InsertCtx, sym: &str) {
    let Some(target) = ctx.target.get_untracked() else {
        ctx.hint.set(Some(
            "\u{1f446} Tap a box you're writing in first \u{2014} a flow box, a contention title, Notes, or your reason \u{2014} then tap a symbol and it'll land where your cursor is."
                .to_string(),
        ));
        return;
    };
    let Some(signal) = target.signal(state) else {
        // The box that had focus (e.g. its row) no longer exists.
        ctx.target.set(None);
        ctx.hint.set(Some(
            "\u{1f446} That box is gone \u{2014} tap the box you're writing in, then tap a symbol again."
                .to_string(),
        ));
        return;
    };
    let text = signal.get_untracked();
    let id = target.id();
    let (new_text, caret) = inserted_text(&text, caret_for(&id, &text), sym);
    signal.set(new_text.clone());
    ctx.hint.set(None);
    save(state);
    restore_focus(&id, &new_text, caret);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_at_end_with_a_separating_space_like_before() {
        assert_eq!(inserted_text("harm", None, "∴"), ("harm ∴".to_string(), 8));
        assert_eq!(inserted_text("", None, "∴"), ("∴".to_string(), 3));
        assert_eq!(inserted_text("harm ", None, "∴"), ("harm ∴".to_string(), 8));
        assert_eq!(
            inserted_text("harm\n", None, "∴"),
            ("harm\n∴".to_string(), 8)
        );
    }

    #[test]
    fn inserts_at_the_caret_not_just_the_end() {
        // Caret after "ab" in "abcd": symbol lands mid-text, caret after it.
        assert_eq!(
            inserted_text("abcd", Some((2, 2)), "→"),
            ("ab →cd".to_string(), 6)
        );
        // Caret at the very start: no leading space.
        assert_eq!(
            inserted_text("abcd", Some((0, 0)), "→"),
            ("→abcd".to_string(), 3)
        );
    }

    #[test]
    fn replaces_the_selection() {
        assert_eq!(
            inserted_text("aXXd", Some((1, 3)), "→"),
            ("a →d".to_string(), 5)
        );
        // Reversed or out-of-range selections are clamped, never panic.
        assert_eq!(
            inserted_text("ab", Some((99, 1)), "→"),
            ("a →".to_string(), 5)
        );
    }

    #[test]
    fn caret_inside_a_multibyte_char_snaps_to_its_start() {
        // Byte 2 is inside "🐰" (bytes 0..4) in the source text.
        assert_eq!(
            inserted_text("🐰x", Some((2, 2)), "→"),
            ("→🐰x".to_string(), 3)
        );
    }

    #[test]
    fn utf16_offsets_round_trip_through_emoji() {
        let text = "a🐰b";
        // UTF-16: a=0, 🐰=1..3, b=3, end=4.
        assert_eq!(utf16_to_byte(text, 0), 0);
        assert_eq!(utf16_to_byte(text, 1), 1);
        assert_eq!(utf16_to_byte(text, 3), 5);
        assert_eq!(utf16_to_byte(text, 4), 6);
        assert_eq!(utf16_to_byte(text, 99), 6);
        // Inside the surrogate pair: caret lands before the emoji.
        assert_eq!(utf16_to_byte(text, 2), 1);
        for byte in [0, 1, 5, 6] {
            assert_eq!(utf16_to_byte(text, byte_to_utf16(text, byte)), byte);
        }
    }
}
