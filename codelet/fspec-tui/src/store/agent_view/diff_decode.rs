//! RPC-391 — decode marker-encoded diff lines into colored ratatui spans.
//!
//! Feature: spec/features/agentview-edit-diff-rendering.feature
//!
//! Mirrors the TS `VirtualList renderItem` decode (`AgentView.tsx:5345-5391`)
//! and the `TurnContentModal` decode (`TurnContentModal.tsx:71-96`):
//!   - line containing `[R]` → strip the 3-char marker, whole line gets the
//!     dark-red background + white fg.
//!   - line containing `[A]` → strip `[A]`, dark-green background + white fg.
//!   - context line matching `^[L ]?\s*\d+\s{3}` → gray line-number gutter +
//!     default-white content (split at the 3 spaces after the number).
//!   - otherwise → a single default-styled span.
//!
//! Shared by `chunk_wrap` (scrollback) and `turn_modal` (the full-diff
//! modal) so both decode identically and neither shows literal markers.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Removed-line background `#8B0000` (TS `DIFF_COLORS.removed`).
pub const DIFF_BG_REMOVED: Color = Color::Rgb(139, 0, 0);
/// Added-line background `#006400` (TS `DIFF_COLORS.added`).
pub const DIFF_BG_ADDED: Color = Color::Rgb(0, 100, 0);

/// Decode a single marker-encoded diff line into styled spans.
///
/// Returns the spans for the line (one or two). Markers are stripped so
/// they never render literally.
///
/// **RPC-391**: width-agnostic decode (no padding). Equivalent to
/// [`decode_diff_line_padded`] with `width == 0`. Kept as the
/// no-width entry point so call sites that don't have a render width
/// (and the modal's `is_decoded_diff_line` checks) stay clear.
pub fn decode_diff_line(line: &str) -> Vec<Span<'static>> {
    decode_diff_line_padded(line, 0)
}

/// **RPC-392**: decode a marker-encoded diff line, right-padding the
/// `[R]`/`[A]` bar with spaces to `width` display columns so the
/// background fills the row (parity with the TS `<Box flexGrow={1}>`).
///
/// Context-gutter and plain lines are returned UNCHANGED (no padding, no
/// background) — only the removed/added bars are padded. `width == 0`
/// (or content already ≥ `width`) adds no padding and never panics
/// (saturating arithmetic). The display-width metric is the same
/// `chars().count()` proxy `wrap_to_width` uses (DRY).
pub fn decode_diff_line_padded(line: &str, width: usize) -> Vec<Span<'static>> {
    if let Some(idx) = line.find("[R]") {
        return vec![colored_span(
            pad_to_width(strip_marker(line, idx), width),
            DIFF_BG_REMOVED,
        )];
    }
    if let Some(idx) = line.find("[A]") {
        return vec![colored_span(
            pad_to_width(strip_marker(line, idx), width),
            DIFF_BG_ADDED,
        )];
    }
    if let Some(split) = context_gutter_len(line) {
        let (gutter, content) = line.split_at(split);
        return vec![
            Span::styled(gutter.to_string(), Style::default().fg(Color::Gray)),
            Span::styled(content.to_string(), Style::default().fg(Color::White)),
        ];
    }
    vec![Span::raw(line.to_string())]
}

/// Right-pad `text` with spaces to `width` DISPLAY columns (the
/// `chars().count()` proxy shared with `wrap_to_width`). If the content
/// is already as wide as / wider than `width`, it is returned unchanged
/// (no truncation). Saturating arithmetic — `width == 0` adds nothing.
fn pad_to_width(text: String, width: usize) -> String {
    let display = text.chars().count();
    let pad = width.saturating_sub(display);
    if pad == 0 {
        return text;
    }
    let mut out = text;
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

/// Whether `line` is a marker/diff-context line that `decode_diff_line`
/// will style (vs. a plain header/indicator line rendered as-is).
pub fn is_decoded_diff_line(line: &str) -> bool {
    line.contains("[R]") || line.contains("[A]") || context_gutter_len(line).is_some()
}

/// **RPC-391/392**: decode a single wrapped modal row into styled spans.
/// Diff rows (markers / context gutter) get colored exactly like the
/// scrollback; `[R]`/`[A]` bars are right-padded to `width` columns so the
/// background fills the row. Non-diff rows render as a single raw span
/// (parity with the previous plain-text modal). Shared so the modal never
/// shows literal `[R]`/`[A]`.
pub fn decode_modal_row(row: &str, width: usize) -> Vec<Span<'static>> {
    if is_decoded_diff_line(row) {
        decode_diff_line_padded(row, width)
    } else {
        vec![Span::raw(row.to_string())]
    }
}

fn colored_span(text: String, bg: Color) -> Span<'static> {
    Span::styled(text, Style::default().bg(bg).fg(Color::White))
}

/// Remove the 3-char marker (`[R]`/`[A]`) at byte `idx`, keeping the rest.
fn strip_marker(line: &str, idx: usize) -> String {
    let mut out = String::with_capacity(line.len().saturating_sub(3));
    out.push_str(&line[..idx]);
    out.push_str(&line[idx + 3..]);
    out
}

/// If `line` matches `^[L ]?\s*\d+\s{3}` (optional tree connector / leading
/// space, digits, then exactly three spaces), return the byte length of the
/// gutter prefix (number + the 3 trailing spaces). Otherwise `None`.
fn context_gutter_len(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    // Optional leading 'L' or ' ' (the tree connector slot).
    if i < bytes.len() && (bytes[i] == b'L' || bytes[i] == b' ') {
        i += 1;
    }
    // Any further leading spaces (line-number left padding).
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // At least one digit.
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    // Exactly three spaces follow the number.
    if line.get(i..i + 3) == Some("   ") {
        Some(i + 3)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/agentview-edit-diff-rendering.feature
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn removed_line_strips_marker_and_colors_red() {
        let spans = decode_diff_line("  2 [R]- line2");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.bg, Some(DIFF_BG_REMOVED));
        assert_eq!(spans[0].style.fg, Some(Color::White));
        assert!(!spans[0].content.contains("[R]"));
        assert!(spans[0].content.contains("line2"));
    }

    #[test]
    fn added_line_strips_marker_and_colors_green() {
        let spans = decode_diff_line("  3 [A]+ CHANGED");
        assert_eq!(spans[0].style.bg, Some(DIFF_BG_ADDED));
        assert!(!spans[0].content.contains("[A]"));
    }

    #[test]
    fn context_line_splits_gray_gutter_and_white_content() {
        let spans = decode_diff_line("L 250   foo");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].style.fg, Some(Color::Gray));
        assert!(spans[0].content.contains("250"));
        assert_eq!(spans[1].style.fg, Some(Color::White));
        assert_eq!(spans[1].content.as_ref(), "foo");
    }

    #[test]
    fn plain_line_is_a_single_raw_span() {
        let spans = decode_diff_line("... +5 lines (select turn to /expand)");
        assert_eq!(spans.len(), 1);
        assert!(spans[0].style.bg.is_none());
    }
}
