//! Width-aware text wrap mirroring `src/tui/utils/textWrap.ts`.
//!
//! Feature: spec/features/agentview-scrollback-wrap.feature
//!
//! Splits a body string into one rendered row per visual width-`width`
//! slice, preserving paragraph boundaries (literal `\n` becomes a hard
//! break). Words wider than `width` are sliced by `char` count — long
//! tokens (e.g. URLs, 300×'x' bodies) still produce one line per
//! visual row instead of being clipped at the right edge by
//! ratatui's `Paragraph` no-wrap default.
//!
//! `unicode_width::UnicodeWidthStr::width()` is used for display-width
//! measurements so wide characters (CJK, emojis) are correctly accounted
//! for. Paragraphs that already fit within `width` are passed through
//! VERBATIM (internal whitespace preserved) — parity with
//! `textWrap.ts` (lines 104-109). This keeps rendered table grid rows
//! and their column padding intact instead of word-collapsing them;
//! only over-width paragraphs are word-wrapped.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// TUI-130: how a wrapped row ends in copied text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowEnd {
    /// The row ends a line of the text (a `\n`, or the end of the text).
    Break,
    /// The wrap split the line here. `space` is true when the wrap removed
    /// the space between two words at this point, false when it cut inside
    /// a token (a long URL) or kept the space on the row.
    Wrap { space: bool },
}

impl RowEnd {
    /// What joins this row to the next one in copied text: a line break,
    /// the removed space, or nothing.
    pub fn joiner(self) -> &'static str {
        match self {
            RowEnd::Break => "\n",
            RowEnd::Wrap { space: true } => " ",
            RowEnd::Wrap { space: false } => "",
        }
    }
}

/// Wrap `text` so every returned `String` has at most `width` chars.
///
/// - `\n` in `text` produces a hard paragraph break (one empty `String`
///   per empty paragraph).
/// - `width == 0` is treated as `width == 1` to keep the algorithm
///   terminating on degenerate viewports.
/// - Trailing whitespace at the end of an emitted row is trimmed.
pub fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    wrap_rows(text, width)
        .into_iter()
        .map(|(row, _)| row)
        .collect()
}

/// TUI-130: [`wrap_to_width`] with how each row ends, so a copy can join
/// the rows the wrap split without inserting line breaks.
pub fn wrap_rows(text: &str, width: usize) -> Vec<(String, RowEnd)> {
    wrap_rows_hanging(text, width, width)
}

/// TUI-130: [`wrap_rows`] with a narrower first row (`first_width`), for a
/// row that starts after a prefix such as `● ` so it is never cut off.
pub fn wrap_rows_hanging(text: &str, first_width: usize, width: usize) -> Vec<(String, RowEnd)> {
    let width = width.max(1);
    let first_width = first_width.clamp(1, width);
    let mut out: Vec<(String, RowEnd)> = Vec::new();
    for paragraph in text.split('\n') {
        let limit = if out.is_empty() { first_width } else { width };
        if paragraph.is_empty() {
            out.push((String::new(), RowEnd::Break));
            continue;
        }
        // Parity with `textWrap.ts` (lines 104-109): a paragraph whose
        // display width fits within `width` is emitted VERBATIM, preserving
        // internal whitespace. This keeps rendered table grid rows (column
        // padding) intact instead of word-collapsing them. Only over-width
        // paragraphs are wrapped.
        if paragraph.width() <= limit {
            out.push((paragraph.to_string(), RowEnd::Break));
            continue;
        }
        let mut rows = Vec::new();
        let first = if out.is_empty() { first_width } else { width };
        wrap_paragraph(paragraph, first, width, &mut rows);
        out.extend(row_ends(paragraph, rows));
    }
    out
}

/// How each row of one wrapped paragraph ends: the rows hold the
/// paragraph's words joined by single spaces (long words sliced), so walking
/// them over that text tells where the wrap removed a space.
fn row_ends(paragraph: &str, rows: Vec<String>) -> Vec<(String, RowEnd)> {
    let words = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut rest = words.as_str();
    let last = rows.len().saturating_sub(1);
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            rest = rest.strip_prefix(row.as_str()).unwrap_or(rest);
            if i == last {
                return (row, RowEnd::Break);
            }
            let space = match rest.strip_prefix(' ') {
                Some(after) => {
                    rest = after;
                    true
                }
                None => false,
            };
            (row, RowEnd::Wrap { space })
        })
        .collect()
}

/// Wrap a single newline-free paragraph by word-splitting, breaking
/// any token wider than the row into successive row-wide slices. The
/// paragraph's first row is `first_width` wide, every later row `width`.
///
/// Preserves the separating space between the last fitting word and a
/// following long token so callers can reconstruct the original body
/// by concatenating every emitted row (byte-equal round-trip).
fn wrap_paragraph(paragraph: &str, first_width: usize, width: usize, out: &mut Vec<String>) {
    let limit = |out: &Vec<String>| if out.is_empty() { first_width } else { width };
    let mut current = String::new();
    let mut current_len: usize = 0;

    for word in paragraph.split_whitespace() {
        let word_len = word.width();
        if word_len > width || (current.is_empty() && word_len > limit(out)) {
            // Preserve the inter-word space at the end of the
            // current line when room allows — this keeps the
            // concatenation of all emitted rows byte-equal to the
            // original paragraph for inputs of the form
            // "<prefix> <long-token>".
            if !current.is_empty() {
                if current_len < limit(out) {
                    current.push(' ');
                }
                out.push(std::mem::take(&mut current));
                current_len = 0;
            }
            // Slice the long word into row-wide display-width rows.
            let mut buf = String::new();
            let mut buf_width: usize = 0;
            for ch in word.chars() {
                let ch_width = ch.width().unwrap_or(1);
                buf.push(ch);
                buf_width += ch_width;
                if buf_width >= limit(out) {
                    out.push(std::mem::take(&mut buf));
                    buf_width = 0;
                }
            }
            if !buf.is_empty() {
                current = buf;
                current_len = current.width();
            }
            continue;
        }

        // Will the next word (plus a separating space, if needed) fit?
        let needs_space = !current.is_empty();
        let projected = current_len + if needs_space { 1 } else { 0 } + word_len;
        if projected <= limit(out) {
            if needs_space {
                current.push(' ');
                current_len += 1;
            }
            current.push_str(word);
            current_len += word_len;
        } else {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
            current_len = word_len;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
}

/// TUI-126: `text` wrapped to `width` columns ([`wrap_to_width`]) as styled
/// lines, so a renderer shows every character at any width instead of
/// cutting the text off at the right edge. Shared by every view and dialog
/// that shows long text (TUI-131).
pub fn wrapped_lines(text: &str, width: usize, style: Style) -> Vec<Line<'static>> {
    wrap_to_width(text, width)
        .into_iter()
        .map(|row| Line::from(Span::styled(row, style)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_unsplit_token_breaks_into_width_slices() {
        let body = "x".repeat(300);
        let rows = wrap_to_width(&body, 80);
        assert_eq!(rows.len(), 4, "300 chars in width 80 → 4 rows");
        let concatenated: String = rows.join("");
        assert_eq!(concatenated, body);
        for (idx, row) in rows.iter().enumerate() {
            let expected_len = if idx < 3 { 80 } else { 300 - 3 * 80 };
            assert_eq!(row.chars().count(), expected_len);
        }
    }

    #[test]
    fn wraps_on_whitespace_when_possible() {
        let rows = wrap_to_width("hello world foo bar", 10);
        assert_eq!(
            rows,
            vec![
                "hello".to_string(),
                "world foo".to_string(),
                "bar".to_string()
            ]
        );
    }

    #[test]
    fn newline_produces_hard_break() {
        let rows = wrap_to_width("[Thinking]\nhello", 80);
        assert_eq!(rows, vec!["[Thinking]".to_string(), "hello".to_string()]);
    }

    #[test]
    fn empty_input_returns_one_empty_row() {
        let rows = wrap_to_width("", 80);
        assert_eq!(rows, vec![String::new()]);
    }

    #[test]
    fn long_word_followed_by_long_word_keeps_byte_equality() {
        // Both tokens exceed `width` so both pass through the
        // long-word break path. The inter-token space rides on the
        // tail of the first token's last partial row when room allows,
        // keeping the concatenation byte-equal to the input.
        let rows = wrap_to_width(&format!("{} done", "x".repeat(5)), 3);
        assert_eq!(
            rows,
            vec![
                "xxx".to_string(),
                "xx ".to_string(),
                "don".to_string(),
                "e".to_string(),
            ]
        );
        let concatenated: String = rows.join("");
        assert_eq!(concatenated, "xxxxx done");
    }

    // Unit-level regression for the fits-within-width verbatim
    // pass-through (parity with textWrap.ts lines 104-109). The
    // scenario-linked end-to-end coverage lives in
    // markdown_tables_tests.rs; this guards the wrap primitive directly.
    #[test]
    fn fits_within_width_preserves_internal_padding() {
        let row = "│ Name  │ Role     │ Location  │";
        let wrapped = wrap_to_width(row, 200);
        assert_eq!(wrapped, vec![row.to_string()]);
        assert!(
            wrapped[0].contains("Name  "),
            "padding must not collapse to a single space"
        );
    }

    #[test]
    fn zero_width_treated_as_one() {
        let rows = wrap_to_width("ab", 0);
        // Width 1: each char on its own row.
        assert_eq!(rows, vec!["a".to_string(), "b".to_string()]);
    }
}
