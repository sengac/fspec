//! Logging helpers for the Rhai custom-provider dispatch chain.
//!
//! Keeps `tracing` preview/truncation logic in one place so the
//! provider/stream modules stay under the 300-line cap.

/// Render a JSON value as a string truncated to `max_chars` characters
/// for log previews. Never panics; falls back to `<<serialize failed>>`
/// when serialisation fails.
pub(crate) fn truncate_json_preview(
    value: &serde_json::Value,
    max_chars: usize,
) -> String {
    let s = serde_json::to_string(value)
        .unwrap_or_else(|_| "<<serialize failed>>".to_string());
    truncate_str(&s, max_chars)
}

/// Truncate a string slice to `max_chars` characters, annotating the
/// truncation length. Safe on non-ASCII input (uses char iteration).
pub(crate) fn truncate_str(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{head}…(truncated {max_chars} of {total})")
    }
}
