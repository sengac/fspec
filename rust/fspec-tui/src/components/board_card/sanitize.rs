//! BOARD-024 — terminal-safe display text.
//!
//! Canonical values keep their original bytes for actions (link ids and
//! attachment paths); only the rendered copy passes through here. ANSI CSI
//! and OSC/DCS-style string sequences are removed entirely (so an OSC title
//! payload never reaches the screen), tabs become spaces and every other
//! control character is dropped.

/// Sanitize multi-line text, keeping `\n` line breaks.
pub fn sanitize(text: &str) -> String {
    let normalized = crate::text_normalize::normalize_line_endings(text);
    let mut out = String::with_capacity(normalized.len());
    let mut chars = normalized.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\u{1b}' => skip_escape(&mut chars),
            '\u{9b}' => skip_csi(&mut chars),
            '\n' => out.push('\n'),
            '\t' => out.push_str("    "),
            other if other.is_control() => {}
            other => out.push(other),
        }
    }
    out
}

/// Sanitize text that must render on one line (titles, labels, errors).
pub fn sanitize_line(text: &str) -> String {
    sanitize(text).replace('\n', " ")
}

fn skip_csi(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for next in chars.by_ref() {
        if ('\u{40}'..='\u{7e}').contains(&next) {
            break;
        }
    }
}

fn skip_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(next) = chars.next() {
        if next == '\u{7}' || next == '\u{9c}' {
            break;
        }
        if next == '\u{1b}' && chars.peek() == Some(&'\\') {
            chars.next();
            break;
        }
    }
}

fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.next() {
        Some('[') => skip_csi(chars),
        Some(']' | 'P' | 'X' | '^' | '_') => skip_string(chars),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_csi_osc_and_controls_but_keeps_unicode_and_newlines() {
        let raw = "CLEAN\u{1b}]0;bad-title\u{7}\u{1b}[31m日本語\u{1b}[0m\u{8}\nnext\tcol";
        assert_eq!(sanitize(raw), "CLEAN日本語\nnext    col");
        assert_eq!(sanitize_line("a\nb\u{1b}[1m"), "a b");
    }
}
