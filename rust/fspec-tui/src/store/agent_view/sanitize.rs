//! Terminal output sanitization for TUI rendering.
//!
//! Strips ANSI escape sequences, replaces tabs, removes carriage returns,
//! and filters control characters from tool output before it enters the
//! scrollback buffer. Mirrors TypeScript `sanitizeForTerminal()` from
//! `src/tui/utils/stringWidth.ts`.
//!
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
