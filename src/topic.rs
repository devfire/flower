//! Current NSDA Public Forum resolution (issue #13).
//!
//! A browser fetch of speechanddebate.org is CORS-blocked, so the deploy
//! workflow extracts the resolution at build time into same-origin
//! `topics.json` (see `scripts/fetch_nsda_topic.py`). That file is also
//! bundled into the binary via `include_str!`, so the field is pre-filled
//! fully offline or when the JSON is missing. A judge's saved or typed
//! topic is never overwritten: a differing NSDA value is only *offered*.

use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Same-origin file emitted next to `dist/index.html` at deploy time and
/// copied by Trunk for local builds (`index.html` copy-file link).
/// Root-absolute, not relative: the SPA 404 fallback serves the app at
/// unknown deep paths (e.g. `/some/deep/path`), where a relative fetch
/// would resolve under that path and 404. The app is deployed at the
/// origin root (custom domain), so `/` is its base; anywhere that does
/// not hold, the fetch fails and callers keep the bundled topic.
const TOPICS_PATH: &str = "/topics.json";
const BUNDLED_JSON: &str = include_str!("../topics.json");

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct NsdaTopic {
    pub resolution: String,
    /// Topic period label, e.g. "2026 September/October".
    pub period: String,
    /// Date the build-time extraction ran (YYYY-MM-DD), if known.
    pub retrieved: String,
    pub source: String,
}

impl Default for NsdaTopic {
    fn default() -> Self {
        Self {
            resolution: String::new(),
            period: String::new(),
            retrieved: String::new(),
            source: "https://www.speechanddebate.org/topics/".to_owned(),
        }
    }
}

impl NsdaTopic {
    /// Parse and validate a `topics.json` payload. Anything that is not
    /// a non-empty NSDA-style resolution is rejected, so a bad fetch can
    /// never blank or corrupt the topic field.
    #[must_use]
    pub fn from_json(json: &str) -> Option<Self> {
        let parsed: Self = serde_json::from_str(json).ok()?;
        let topic = Self {
            resolution: parsed.resolution.trim().to_owned(),
            period: parsed.period.trim().to_owned(),
            retrieved: parsed.retrieved.trim().to_owned(),
            source: parsed.source.trim().to_owned(),
        };
        if topic.resolution.starts_with("Resolved:") && !topic.period.is_empty() {
            Some(topic)
        } else {
            None
        }
    }

    /// Provenance line shown under the topic field.
    #[must_use]
    pub fn provenance(&self) -> String {
        if self.retrieved.is_empty() {
            format!(
                "NSDA \u{b7} {} \u{b7} speechanddebate.org/topics",
                self.period
            )
        } else {
            format!(
                "NSDA \u{b7} {} \u{b7} retrieved {} \u{b7} speechanddebate.org/topics",
                self.period, self.retrieved
            )
        }
    }
}

/// The last bundled resolution, compiled in. Parse failure is a build
/// bug (the file is committed and validated by the extraction script's
/// tests), so fall back to an empty topic rather than panicking.
#[must_use]
pub fn bundled() -> NsdaTopic {
    NsdaTopic::from_json(BUNDLED_JSON).unwrap_or_default()
}

/// What to do with a candidate NSDA topic given what the field holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicAction {
    /// Field still holds the untouched auto-filled bundled value (or
    /// was empty at spawn and is still empty): safe to fill.
    Prefill,
    /// Field already shows the candidate.
    UpToDate,
    /// Judge has their own text, or touched the field since spawn:
    /// offer the candidate, never force it.
    Offer,
}

/// Decide how `candidate` may enter a field currently holding `current`,
/// where `auto_filled` is the bundled resolution the app itself put
/// there on a fresh visit and `spawned` is the field value captured at
/// spawn (after that auto-fill). Only text the app put there and the
/// judge has not touched since spawn is replaced: a field deliberately
/// cleared after spawn no longer equals `spawned`, so it is only offered
/// the candidate, never refilled.
#[must_use]
pub fn decide(
    current: &str,
    auto_filled: &str,
    spawned: &str,
    candidate: &NsdaTopic,
) -> TopicAction {
    let current = current.trim();
    if current == candidate.resolution.trim() {
        TopicAction::UpToDate
    } else if current == spawned.trim() && (current.is_empty() || current == auto_filled.trim()) {
        TopicAction::Prefill
    } else {
        TopicAction::Offer
    }
}

/// Fetch the same-origin `topics.json` shipped beside the app. Returns
/// `None` offline, on 404, or on any invalid payload; callers keep the
/// bundled topic in every one of those cases.
#[must_use]
pub async fn fetch_current() -> Option<NsdaTopic> {
    let window = web_sys::window()?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(TOPICS_PATH))
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    if !response.ok() {
        return None;
    }
    let text = JsFuture::from(response.text().ok()?)
        .await
        .ok()?
        .as_string()?;
    NsdaTopic::from_json(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> NsdaTopic {
        NsdaTopic {
            resolution: "Resolved: New topic.".to_owned(),
            period: "2026 November/December".to_owned(),
            retrieved: "2026-11-01".to_owned(),
            source: "https://www.speechanddebate.org/topics/".to_owned(),
        }
    }

    #[test]
    fn bundled_topic_is_the_committed_nsda_resolution() {
        let topic = bundled();
        assert!(topic.resolution.starts_with("Resolved:"), "{topic:?}");
        assert!(!topic.period.is_empty());
        assert!(topic.provenance().contains(&topic.period));
        assert!(topic.provenance().contains("speechanddebate.org/topics"));
    }

    #[test]
    fn untouched_autofill_is_prefilled_but_touched_field_is_only_offered() {
        let new = candidate();
        // Empty at spawn (no bundled fill) and still empty: prefill.
        assert_eq!(decide("", "Resolved: Old.", "", &new), TopicAction::Prefill);
        assert_eq!(
            decide("  ", "Resolved: Old.", "", &new),
            TopicAction::Prefill
        );
        // Stale bundled auto-fill, untouched since spawn, upgrades.
        assert_eq!(
            decide("Resolved: Old.", "Resolved: Old.", "Resolved: Old.", &new),
            TopicAction::Prefill
        );
        assert_eq!(
            decide(&new.resolution, "Resolved: Old.", "Resolved: Old.", &new),
            TopicAction::UpToDate
        );
        // Issue #25: auto-fill deliberately cleared while the fetch was
        // in flight stays empty — offered, never resurrected.
        assert_eq!(
            decide("", "Resolved: Old.", "Resolved: Old.", &new),
            TopicAction::Offer
        );
        // Saved topic deliberately cleared since spawn likewise.
        assert_eq!(
            decide("", "Resolved: Old.", "Resolved: My own wording.", &new),
            TopicAction::Offer
        );
        // Any edit since spawn, and untouched saved judge text, are
        // never overwritten, only offered.
        assert_eq!(
            decide("Resolved: Typed.", "Resolved: Old.", "Resolved: Old.", &new),
            TopicAction::Offer
        );
        assert_eq!(
            decide(
                "Resolved: My own wording.",
                "Resolved: Old.",
                "Resolved: My own wording.",
                &new
            ),
            TopicAction::Offer
        );
    }

    #[test]
    fn invalid_topics_json_is_rejected() {
        assert!(NsdaTopic::from_json("{not json").is_none());
        assert!(NsdaTopic::from_json(r#"{"resolution":"","period":"x"}"#).is_none());
        assert!(NsdaTopic::from_json(r#"{"resolution":"Hello.","period":"x"}"#).is_none());
        assert!(NsdaTopic::from_json(r#"{"resolution":"Resolved: Ok."}"#).is_none());
        assert!(
            NsdaTopic::from_json(
                r#"{"resolution":"  Resolved: Ok.  ","period":" 2026 September/October "}"#
            )
            .is_some_and(
                |t| t.resolution == "Resolved: Ok." && t.period == "2026 September/October"
            )
        );
    }
}
