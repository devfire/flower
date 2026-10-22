//! `OpenRouter` connection (OAuth PKCE) + AI round summary.
//!
//! Client-only by design: the judge signs in at `OpenRouter`, which
//! redirects back with a one-time `code`; this module exchanges it for
//! *the judge's own* API key (their credits pay for summaries). The
//! PKCE verifier and the key live in `sessionStorage` only — they die
//! with the tab and are never written to the `localStorage` round save.
//! The key is sent nowhere except `openrouter.ai`.

use std::fmt;
use std::fmt::Write as _;

use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use crate::model::{RoundData, SPEECH_COLS};

const AUTH_URL: &str = "https://openrouter.ai/auth";
const KEYS_URL: &str = "https://openrouter.ai/api/v1/auth/keys";
const CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// Free-tier routing model: a connected account is still required (it
/// owns the key), but summaries cost the judge nothing.
const SUMMARY_MODEL: &str = "openrouter/free";
/// AI summary prompts, externalized to `prompts/` for easy editing;
/// bundled at compile time (same pattern as `topics.json`). Edit those
/// files — not Rust — to change the wording. `{round}` in the user
/// prompt is replaced with the flow text from `summary_prompt`.
const SUMMARY_SYSTEM_PROMPT: &str = include_str!("../prompts/summary_system.md");
const SUMMARY_USER_PROMPT: &str = include_str!("../prompts/summary_user.md");

const KEY_STORAGE: &str = "flower-openrouter-key";
const VERIFIER_STORAGE: &str = "flower-openrouter-verifier";
const STATE_STORAGE: &str = "flower-openrouter-state";

/// A user-owned `OpenRouter` API key. The wrapped value is reachable only
/// via `as_str` (for the `Authorization` header); `Debug` is redacted so
/// the key can never leak into console logs or panic messages.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Accept only a plausible key: non-empty, whitespace-free. Anything
    /// else (a stored error page, a hand-edited value) is no key at all.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let key = raw.trim();
        if !key.is_empty() && !key.chars().any(char::is_whitespace) {
            Some(Self(key.to_owned()))
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(\u{2026}redacted)")
    }
}

fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

fn stored(name: &str) -> Option<String> {
    session_storage()?.get_item(name).ok().flatten()
}

/// The connected key from a previous authorize in this tab, if any.
#[must_use]
pub fn load_key() -> Option<ApiKey> {
    stored(KEY_STORAGE).and_then(|raw| ApiKey::parse(&raw))
}

/// Forget the key and any half-finished authorize attempt.
pub fn disconnect() {
    if let Some(storage) = session_storage() {
        let _ = storage.remove_item(KEY_STORAGE);
        let _ = storage.remove_item(VERIFIER_STORAGE);
        let _ = storage.remove_item(STATE_STORAGE);
    }
}

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Base64url without padding (RFC 7636) — the PKCE wire encoding.
#[must_use]
pub fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64URL[((n >> 18) & 63) as usize] as char);
        out.push(B64URL[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64URL[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[(n & 63) as usize] as char);
        }
    }
    out
}

/// Percent-encode everything except RFC 3986 unreserved characters, so
/// the callback URL survives as one query value.
#[must_use]
pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[must_use]
pub fn auth_url(callback_url: &str, challenge: &str, state: &str) -> String {
    format!(
        "{AUTH_URL}?callback_url={}&code_challenge={}&code_challenge_method=S256&state={}&key_label={}",
        percent_encode(callback_url),
        percent_encode(challenge),
        percent_encode(state),
        percent_encode("Flower"),
    )
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let mut buf = [0u8; N];
    let crypto = window
        .crypto()
        .map_err(|_| "browser refused crypto".to_owned())?;
    crypto
        .get_random_values_with_u8_array(&mut buf)
        .map_err(|_| "browser refused to generate random values".to_owned())?;
    Ok(buf)
}

async fn sha256_base64url(text: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let promise = window
        .crypto()
        .map_err(|_| "browser refused crypto".to_owned())?
        .subtle()
        .digest_with_str_and_u8_array("SHA-256", text.as_bytes())
        .map_err(|_| "browser refused SHA-256".to_owned())?;
    let digest = JsFuture::from(promise)
        .await
        .map_err(|_| "SHA-256 failed".to_owned())?;
    Ok(base64url(&js_sys::Uint8Array::new(&digest).to_vec()))
}

/// Start the PKCE flow: stash verifier + CSRF state, then send the judge
/// to `OpenRouter`. On success the browser navigates away and this never
/// returns; the round autosave is flushed first so no typed cell is lost
/// to the redirect.
///
/// # Errors
/// A plain-language reason when the browser blocks crypto or storage.
pub async fn begin_connect() -> Result<(), String> {
    crate::persistence::flush_save();
    let verifier = base64url(&random_bytes::<32>()?);
    let state = base64url(&random_bytes::<16>()?);
    let challenge = sha256_base64url(&verifier).await?;
    let storage = session_storage().ok_or("this browser blocked session storage")?;
    storage
        .set_item(VERIFIER_STORAGE, &verifier)
        .and(storage.set_item(STATE_STORAGE, &state))
        .map_err(|_| "this browser blocked session storage".to_owned())?;
    let window = web_sys::window().ok_or("no browser window")?;
    let location = window.location();
    let callback_url = format!(
        "{}{}",
        location
            .origin()
            .map_err(|_| "page URL unavailable".to_owned())?,
        location
            .pathname()
            .map_err(|_| "page URL unavailable".to_owned())?,
    );
    location
        .set_href(&auth_url(&callback_url, &challenge, &state))
        .map_err(|_| "browser refused the OpenRouter redirect".to_owned())
}

/// Exchange the `?code=` `OpenRouter` redirected back with for the judge's
/// API key. `Ok(None)` on an ordinary load (no code in the URL). The code
/// is stripped from the address bar before the exchange, whatever the
/// outcome, so it is never bookmarked or shared.
///
/// # Errors
/// A plain-language reason (mismatched state, expired code, network).
pub async fn handle_callback() -> Result<Option<ApiKey>, String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let location = window.location();
    let search = location
        .search()
        .map_err(|_| "page URL unavailable".to_owned())?;
    let params = web_sys::UrlSearchParams::new_with_str(&search)
        .map_err(|_| "page URL unavailable".to_owned())?;
    let Some(code) = params.get("code") else {
        return Ok(None);
    };
    // Single-use from here: clear the PKCE secrets and the URL first.
    let expected_state = stored(STATE_STORAGE);
    let verifier = stored(VERIFIER_STORAGE);
    disconnect();
    let clean = format!(
        "{}{}",
        location
            .pathname()
            .map_err(|_| "page URL unavailable".to_owned())?,
        location
            .hash()
            .map_err(|_| "page URL unavailable".to_owned())?,
    );
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&clean));
    }
    if params.get("state") != expected_state || expected_state.is_none() {
        return Err(
            "OpenRouter sign-in didn't match the attempt started here — try connecting again."
                .to_owned(),
        );
    }
    let verifier = verifier.ok_or("sign-in attempt expired — try connecting again.")?;
    let body = serde_json::json!({
        "code": code,
        "code_verifier": verifier,
        "code_challenge_method": "S256",
    });
    let text = post_json(KEYS_URL, None, &body).await?;
    let reply: KeyReply = serde_json::from_str(&text)
        .map_err(|_| "OpenRouter sent an unreadable reply — try connecting again.".to_owned())?;
    let key = reply
        .key
        .and_then(|raw| ApiKey::parse(&raw))
        .ok_or("OpenRouter didn't return a key — try connecting again.")?;
    if let Some(storage) = session_storage() {
        storage
            .set_item(KEY_STORAGE, key.as_str())
            .map_err(|_| "this browser blocked session storage".to_owned())?;
    }
    Ok(Some(key))
}

#[derive(Deserialize)]
struct KeyReply {
    key: Option<String>,
}

#[derive(Deserialize)]
struct ChatReply {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

async fn post_json(
    url: &str,
    key: Option<&ApiKey>,
    body: &serde_json::Value,
) -> Result<String, String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let headers =
        web_sys::Headers::new().map_err(|_| "browser refused request headers".to_owned())?;
    headers
        .append("Content-Type", "application/json")
        .map_err(|_| "browser refused request headers".to_owned())?;
    if let Some(key) = key {
        headers
            .append("Authorization", &format!("Bearer {}", key.as_str()))
            .and(headers.append("X-Openrouter-Title", "Flower"))
            .map_err(|_| "browser refused request headers".to_owned())?;
        if let Ok(origin) = window.location().origin() {
            let _ = headers.append("HTTP-Referer", &origin);
        }
    }
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(&body.to_string()));
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(url, &init))
        .await
        .map_err(|_| "couldn't reach OpenRouter — check your connection".to_owned())?
        .dyn_into()
        .map_err(|_| "OpenRouter sent an unreadable reply".to_owned())?;
    let text = JsFuture::from(
        response
            .text()
            .map_err(|_| "OpenRouter sent an unreadable reply".to_owned())?,
    )
    .await
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_default();
    if response.ok() {
        Ok(text)
    } else {
        Err(api_error(&text).unwrap_or_else(|| {
            format!(
                "OpenRouter said no (HTTP {}) — try again",
                response.status()
            )
        }))
    }
}

fn api_error(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct ErrorReply {
        error: Option<ErrorBody>,
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        message: Option<String>,
    }
    let message = serde_json::from_str::<ErrorReply>(body)
        .ok()?
        .error?
        .message?;
    let message = message.trim();
    if message.is_empty() {
        None
    } else {
        Some(format!("OpenRouter: {message}"))
    }
}

/// The full flow as plain text for the model: every non-empty cell under
/// its speech heading (in speaking order), marks spelled out, plus the
/// notes, ballot, and the judge's own decision text.
#[must_use]
pub fn summary_prompt(round: &RoundData) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "Topic: {}", round.topic.trim());
    let _ = writeln!(
        prompt,
        "Pro team: {} · Con team: {}",
        round.pro_team.trim(),
        round.con_team.trim()
    );
    for (rows, side) in [(&round.pro_rows, "Pro"), (&round.con_rows, "Con")] {
        let _ = writeln!(prompt, "\n{side} sheet (their arguments, top to bottom):");
        for row in rows {
            let mut cells = String::new();
            for &col in &round.speaking_order.column_order() {
                if let Some(cell) = row.cells.get(col) {
                    let text = cell.text.trim();
                    if !text.is_empty() {
                        let mark = cell.mark.label();
                        let _ = write!(
                            cells,
                            "\n  - {}: {text}{}",
                            SPEECH_COLS[col].full,
                            if mark.is_empty() {
                                String::new()
                            } else {
                                format!(" [{mark}]")
                            }
                        );
                    }
                }
            }
            if !cells.is_empty() {
                let _ = writeln!(prompt, "Argument: {}", row.title.trim());
                prompt.push_str(&cells);
                prompt.push('\n');
            }
        }
    }
    if !round.notes.trim().is_empty() {
        let _ = writeln!(prompt, "\nNotes: {}", round.notes.trim());
    }
    let _ = writeln!(prompt, "\n{}", round.decision_text());
    prompt
}

/// The user prompt from `prompts/summary_user.md` with `{round}`
/// replaced by the flow text. Extracted so tests can prove the
/// placeholder survives edits to the prompt file.
fn summary_user_prompt(round: &RoundData) -> String {
    SUMMARY_USER_PROMPT
        .trim()
        .replace("{round}", &summary_prompt(round))
}

/// Ask the connected account's `OpenRouter` for a plain-language summary
/// of the round: what was argued, what clashed, what was dropped or
/// extended, and how the weighing went.
///
/// # Errors
/// A plain-language reason (bad key, no credits/models, network).
pub async fn summarize(key: &ApiKey, round: &RoundData) -> Result<String, String> {
    let body = serde_json::json!({
        "model": SUMMARY_MODEL,
        "messages": [
            {"role": "system", "content": SUMMARY_SYSTEM_PROMPT.trim()},
            {"role": "user", "content": summary_user_prompt(round)},
        ],
    });
    let text = post_json(CHAT_URL, Some(key), &body).await?;
    let reply: ChatReply = serde_json::from_str(&text)
        .map_err(|_| "OpenRouter sent an unreadable reply — try again.".to_owned())?;
    reply
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .map(|content| content.trim().to_owned())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "OpenRouter returned an empty summary — try again.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RoundData, Side};

    #[test]
    fn base64url_is_unpadded_and_url_safe() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        // 0xfb 0xff would be `+/8=` in standard base64.
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn percent_encode_keeps_unreserved_only() {
        assert_eq!(percent_encode("aZ09-_.~"), "aZ09-_.~");
        assert_eq!(
            percent_encode("http://x/cb?a=b&c=d e"),
            "http%3A%2F%2Fx%2Fcb%3Fa%3Db%26c%3Dd%20e"
        );
    }

    #[test]
    fn api_key_rejects_blank_and_whitespace_and_debug_redacts() {
        assert!(ApiKey::parse("").is_none());
        assert!(ApiKey::parse("  ").is_none());
        assert!(ApiKey::parse("sk or v1").is_none());
        let key = ApiKey::parse("  sk-or-v1-secret  ").unwrap_or_else(|| ApiKey("x".to_owned()));
        assert_eq!(key.as_str(), "sk-or-v1-secret");
        assert!(!format!("{key:?}").contains("secret"));
    }

    #[test]
    fn auth_url_carries_s256_challenge_and_state() {
        let url = auth_url("https://flower.example/", "chal", "st8");
        assert!(url.starts_with("https://openrouter.ai/auth?"));
        assert!(url.contains("callback_url=https%3A%2F%2Fflower.example%2F"));
        assert!(url.contains("code_challenge=chal"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=st8"));
    }

    #[test]
    fn summary_prompt_lists_flow_in_speaking_order_with_marks() {
        let mut round = RoundData::sample();
        round.winner = Some(Side::Pro);
        round.notes = "Con conceded only 9% is recycled".to_owned();
        let prompt = summary_prompt(&round);
        assert!(prompt.contains("Resolved: The United States should ban single-use plastics."));
        assert!(prompt.contains("Pro sheet"));
        assert!(prompt.contains("C1: Oceans"));
        assert!(prompt.contains("Pro Constructive: Plastic → oceans"));
        assert!(prompt.contains("[➜ extended]"));
        assert!(prompt.contains("Winner: Pro"));
        assert!(prompt.contains("Notes: Con conceded only 9% is recycled"));
        assert!(!prompt.contains("Crossfire notes"));
        // Sparse flow: empty default round yields no Argument lines.
        assert!(!summary_prompt(&RoundData::default()).contains("Argument:"));
    }

    #[test]
    fn external_prompts_render_with_round_flow() {
        assert!(!SUMMARY_SYSTEM_PROMPT.trim().is_empty());
        let rendered = summary_user_prompt(&RoundData::sample());
        assert!(rendered.contains("What each side argued"));
        assert!(rendered.contains("Pro sheet"));
        assert!(!rendered.contains("{round}"));
    }
}
