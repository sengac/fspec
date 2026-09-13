//! Canonical terminal-output sanitizer (TUI-111).
//!
//! TUI-111: promoted from `store/agent_view/sanitize.rs` to the
//! crate-level `terminal::sanitize` module so every ingress boundary
//! (stores, chunk recorder, dialogs, view setters) shares ONE
//! implementation — no view re-implements ANSI/control stripping.
//!
//! Strips ANSI escape sequences, replaces tabs, removes carriage returns,
//! and filters control characters from external text before it enters
//! the TUI. Mirrors TypeScript `sanitizeForTerminal()` from
//! `src/tui/utils/stringWidth.ts`.
//!
//! Feature: spec/features/sanitize-all-tui-output-at-ingress.feature
//! Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature

use regex::Regex;
use std::sync::LazyLock;

/// Regex matching ANSI escape sequences (CSI, OSC, SGR).
/// Mirrors TypeScript `ANSI_ESCAPE_REGEX`.
static ANSI_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r"\x1b(?:\[[0-9;]*[A-Za-z]|\][^\x07]*\x07|[^\x1b])")
        .expect("ANSI escape regex must compile — compile-time constant pattern")
});

/// Sanitize text for safe terminal rendering.
///
/// Removes or replaces characters that can cause terminal rendering issues:
/// 1. ANSI escape sequences (colors, cursor movement, etc.)
/// 2. Control characters (except newlines which are preserved)
/// 3. Tabs (replaced with two spaces for consistent width)
/// 4. Carriage returns (removed to prevent line overwriting)
///
/// # Arguments
/// * `text` - Raw text from tool output
///
/// # Returns
/// Sanitized text safe for terminal rendering
pub fn sanitize_for_terminal(text: &str) -> String {
    // Step 1: Strip ANSI escape sequences (colors, cursor movement, etc.)
    let stripped = ANSI_ESCAPE_RE.replace_all(text, "");

    // Step 2: Replace tabs with two spaces, remove carriage returns,
    // and filter control characters (preserving newlines)
    let mut result = String::with_capacity(stripped.len());
    for c in stripped.chars() {
        match c {
            '\t' => result.push_str("  "), // tab → two spaces
            '\r' => {}                     // carriage return → removed
            '\n' => result.push('\n'),     // newline → preserved
            c if is_control_char(c) => {}  // control chars → removed
            _ => result.push(c),           // everything else → kept
        }
    }
    result
}

/// Returns true if `c` is a control character that should be removed.
/// Matches the TypeScript `CONTROL_CHAR_REGEX`: 0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F.
///
/// Note: 0x09 (tab) and 0x0A (newline) are handled separately.
fn is_control_char(c: char) -> bool {
    let code = c as u32;
    matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)
}

#[cfg(test)]
mod proptests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::sanitize_for_terminal;
    use proptest::arbitrary::any;
    use proptest::proptest;

    proptest! {
        /// TUI-111: on arbitrary input (including lone ESC bytes, partial
        /// escape sequences, tabs, CRs and NULs) the output contains no
        /// ESC byte and no forbidden control character.
        #[test]
        fn sanitizer_output_has_no_esc_or_forbidden_control_chars(
            input in proptest::collection::vec(any::<u8>(), 0..200)
        ) {
            let text = String::from_utf8_lossy(&input).to_string();
            let out = sanitize_for_terminal(&text);
            assert!(
                !out.contains('\x1b'),
                "output must contain no ESC byte, got {out:?}"
            );
            for c in out.chars() {
                let code = c as u32;
                assert!(
                    !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
                    "output must contain no forbidden control char U+{code:02X}, got {out:?}"
                );
            }
        }

        /// TUI-111: idempotence — passing the result through the sanitizer a
        /// second time changes nothing.
        #[test]
        fn sanitizer_is_idempotent(
            input in proptest::collection::vec(any::<u8>(), 0..200)
        ) {
            let text = String::from_utf8_lossy(&input).to_string();
            let once = sanitize_for_terminal(&text);
            let twice = sanitize_for_terminal(&once);
            assert_eq!(
                &once, &twice,
                "sanitize(sanitize(x)) != sanitize(x) for {text:?}"
            );
        }

        /// TUI-111: plain text (no escape sequences, tabs, CR or
        /// forbidden control chars — DEL \u{7F} excluded by the strategy)
        /// passes through byte-for-byte unchanged.
        #[test]
        fn plain_text_passes_through_unchanged(
            chars in proptest::collection::vec(
                proptest::prop_oneof![
                    proptest::char::range('\u{20}', '\u{7E}'),
                    proptest::char::range('\u{80}', '\u{2000}')
                ],
                0..100
            )
        ) {
            let input: String = chars.into_iter().collect();
            let out = sanitize_for_terminal(&input);
            assert_eq!(out, input, "plain text must survive unchanged");
        }
    }
}
