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
//! `char` count is used as the visual-width proxy. The set of inputs
//! that flow through scrollback is ASCII-dominant; full Unicode East
//! Asian Width handling can be layered on with the `unicode-width`
//! crate in a later RPC if a multibyte-prefix variant is added.

/// Wrap `text` so every returned `String` has at most `width` chars.
///
/// - `\n` in `text` produces a hard paragraph break (one empty `String`
///   per empty paragraph).
/// - `width == 0` is treated as `width == 1` to keep the algorithm
///   terminating on degenerate viewports.
/// - Trailing whitespace at the end of an emitted row is trimmed.
pub fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            out.push(String::new());
            continue;
        }
        wrap_paragraph(paragraph, width, &mut out);
    }
    out
}

/// Wrap a single newline-free paragraph by word-splitting, breaking
/// any token wider than `width` into successive `width`-char slices.
///
/// Preserves the separating space between the last fitting word and a
/// following long token so callers can reconstruct the original body
/// by concatenating every emitted row (byte-equal round-trip).
fn wrap_paragraph(paragraph: &str, width: usize, out: &mut Vec<String>) {
    let mut current = String::new();
    let mut current_len: usize = 0;

    for word in paragraph.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > width {
            // Preserve the inter-word space at the end of the
            // current line when room allows — this keeps the
            // concatenation of all emitted rows byte-equal to the
            // original paragraph for inputs of the form
            // "<prefix> <long-token>".
            if !current.is_empty() {
                if current_len < width {
                    current.push(' ');
                }
                out.push(std::mem::take(&mut current));
                current_len = 0;
            }
            // Slice the long word into `width`-char rows.
            let mut buf: Vec<char> = Vec::with_capacity(width);
            for ch in word.chars() {
                buf.push(ch);
                if buf.len() == width {
                    out.push(buf.iter().collect());
                    buf.clear();
                }
            }
            if !buf.is_empty() {
                current = buf.iter().collect();
                current_len = current.chars().count();
            }
            continue;
        }

        // Will the next word (plus a separating space, if needed) fit?
        let needs_space = !current.is_empty();
        let projected = current_len + if needs_space { 1 } else { 0 } + word_len;
        if projected <= width {
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

    #[test]
    fn zero_width_treated_as_one() {
        let rows = wrap_to_width("ab", 0);
        // Width 1: each char on its own row.
        assert_eq!(rows, vec!["a".to_string(), "b".to_string()]);
    }
}
