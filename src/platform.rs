//! Browser platform helpers: printing, clipboard, clock, audio.
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
