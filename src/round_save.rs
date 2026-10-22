//! Save repair: parsing/normalising stored rounds and storage keys.

use crate::model::{Mark, RoundData, RowData};

impl RoundData {
    /// Parse a saved round, repairing anything a truncated, older, or
    /// hand-edited save may contain. Only genuinely unparseable JSON (or a
    /// field of the wrong type) is an `Err`; callers must preserve the raw
    /// text in that case rather than overwrite it. A save whose row ids
    /// exhaust the `u32` id space (see `try_normalised`) is likewise an
    /// `Err`, so it takes the same preserved-save recovery path.
    ///
    /// # Errors
    /// Returns the `serde_json` error when `json` is not a readable round.
    pub fn from_saved_json(json: &str) -> Result<Self, serde_json::Error> {
        use serde::de::Error as _;
        serde_json::from_str::<Self>(json)?
            .try_normalised()
            .ok_or_else(|| {
                serde_json::Error::custom("row ids exhausted: no fresh row id available")
            })
    }

    /// Restore the invariants the UI relies on: unique row ids
    /// (later duplicates get fresh ids), and `next_id` above every row id
    /// so a future row can never reuse one.
    ///
    /// Returns `None` when the id space is exhausted — some row already
    /// holds `u32::MAX`, so no fresh duplicate repair and no `next_id`
    /// above it exist. Silently saturating there would keep duplicate
    /// ids (one delete would remove both rows) or hand `next_id` back
    /// as an id already in use.
    #[must_use]
    pub fn try_normalised(mut self) -> Option<Self> {
        use std::collections::HashSet;
        // Fresh ids start above every existing id, so a repaired duplicate
        // can never collide with a row not visited yet. `checked_add`
        // failing means a row holds MAX: bail before any repair.
        let mut fresh = self
            .pro_rows
            .iter()
            .chain(&self.con_rows)
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)?;
        let mut seen = HashSet::new();
        for row in self.pro_rows.iter_mut().chain(self.con_rows.iter_mut()) {
            if !seen.insert(row.id) {
                row.id = fresh;
                seen.insert(fresh);
                fresh = fresh.checked_add(1)?;
            }
        }
        let max_id = self
            .pro_rows
            .iter()
            .chain(&self.con_rows)
            .map(|r| r.id)
            .max()
            .unwrap_or(0);
        // A repaired duplicate may itself have taken MAX, leaving no
        // usable next_id even though every id is now unique.
        self.next_id = self.next_id.max(max_id.checked_add(1)?);
        Some(self)
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
            && blank(&self.notes)
            && self.winner.is_none()
            && blank(&self.rfd)
            && self.pro_rows.iter().all(row_blank)
            && self.con_rows.iter().all(row_blank)
    }
}
pub const STORAGE_KEY: &str = "flower-round-v1";
/// Where an unreadable save is copied before the app starts blank, so the
/// raw round is never silently overwritten (issue #2).
pub const STORAGE_BACKUP_KEY: &str = "flower-round-v1-unreadable-backup";
/// Monotonic autosave revision for `STORAGE_KEY` (issue #18). Kept in a
/// separate key so the round JSON format itself is unchanged and older
/// saves (which have no revision) load as revision 0.
pub const STORAGE_REVISION_KEY: &str = "flower-round-v1-revision";

/// Decode a stored revision: a missing key is a pre-revision (legacy)
/// save at revision 0; an unparsable value is `None`, which callers must
/// treat as unknown rather than as 0, so a corrupt counter is never
/// silently reset under a newer tab.
#[must_use]
pub fn stored_revision(raw: Option<&str>) -> Option<u64> {
    match raw {
        None => Some(0),
        Some(text) => text.parse().ok(),
    }
}

/// The revision a tab may write next: only over the exact revision it
/// loaded (or last wrote). Any other stored revision means another tab
/// saved first, so this returns `None` and the caller must refuse to
/// overwrite that newer save.
#[must_use]
pub fn next_revision(known: u64, stored: u64) -> Option<u64> {
    if stored == known {
        known.checked_add(1)
    } else {
        None
    }
}
