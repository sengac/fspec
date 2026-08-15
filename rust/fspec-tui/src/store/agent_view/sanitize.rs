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
            c if is_control_char(c) => {} // control chars → removed
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /**
     * Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature
     */

    // ── Scenario: ANSI color codes from ls --color are stripped before rendering ──

    /// @step Given a bash command outputs text with ANSI color escape sequences like "\x1b[01;34m" for colored directories
    fn given_ansi_color_sequences() -> String {
        // Simulates `ls --color=always` output
        format!("\x1b[01;34mDocuments\x1b[0m\n\x1b[01;34mDownloads\x1b[0m\nfile.txt")
    }

    /// @step When the tool output is processed for TUI display
    fn when_processed_for_tui(input: &str) -> String {
        sanitize_for_terminal(input)
    }

    /// @step Then the ANSI escape sequences are removed from the displayed text
    fn then_ansi_sequences_removed(output: &str) {
        assert!(
            !output.contains('\x1b'),
            "Output should not contain escape characters, got {:?}",
            output
        );
    }

    /// @step And only the plain text content (filenames) is visible in the TUI
    fn then_only_plain_text_visible(output: &str) {
        let expected = "Documents\nDownloads\nfile.txt";
        assert_eq!(output, expected, "Only plain text should remain, got {output:?}");
    }

    #[test]
    fn ansi_color_codes_from_ls_are_stripped_before_rendering() {
        // @step Given a bash command outputs text with ANSI color escape sequences like "\x1b[01;34m" for colored directories
        let input = given_ansi_color_sequences();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then the ANSI escape sequences are removed from the displayed text
        then_ansi_sequences_removed(&output);

        // @step And only the plain text content (filenames) is visible in the TUI
        then_only_plain_text_visible(&output);
    }

    // ── Scenario: Complex ANSI sequences from neofetch are fully stripped ──

    /// @step Given a bash command outputs complex ANSI sequences including cursor movement, colors, and bold formatting
    fn given_complex_ansi_sequences() -> String {
        // Simulates `neofetch` output with cursor movement, colors, bold
        format!(
            "\x1b[1mOS:\x1b[0m Ubuntu 24.04\n\
             \x1b[34mKernel:\x1b[0m 6.8.0\n\
             \x1b[2J\x1b[H\
             \x1b[32mCPU:\x1b[0m Apple M2\n\
             \x1b]0;Terminal Title\x07\
             Memory: 8GB"
        )
    }

    /// @step Then all ANSI escape sequences are removed from the displayed text
    fn then_all_ansi_removed(output: &str) {
        assert!(
            !output.contains('\x1b'),
            "No escape characters should remain, got {:?}",
            output
        );
        assert!(
            !output.contains('\x07'),
            "BEL character from OSC should be removed, got {:?}",
            output
        );
    }

    /// @step And only readable plain text is visible in the TUI
    fn then_only_readable_text(output: &str) {
        let expected = "OS: Ubuntu 24.04\nKernel: 6.8.0\nCPU: Apple M2\nMemory: 8GB";
        assert_eq!(output, expected, "Only readable text should remain, got {output:?}");
    }

    #[test]
    fn complex_ansi_sequences_from_neofetch_are_fully_stripped() {
        // @step Given a bash command outputs complex ANSI sequences including cursor movement, colors, and bold formatting
        let input = given_complex_ansi_sequences();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then all ANSI escape sequences are removed from the displayed text
        then_all_ansi_removed(&output);

        // @step And only readable plain text is visible in the TUI
        then_only_readable_text(&output);
    }

    // ── Scenario: Tab characters in bash output are replaced with spaces ──

    /// @step Given a bash command outputs text containing tab characters
    fn given_text_with_tabs() -> String {
        "hello\tworld\nname\tvalue".to_string()
    }

    /// @step Then each tab character is replaced with two spaces
    fn then_tabs_replaced_with_two_spaces(output: &str) {
        assert!(
            !output.contains('\t'),
            "No tabs should remain, got {:?}",
            output
        );
    }

    /// @step And the text maintains consistent visual width
    fn then_consistent_visual_width(output: &str) {
        let expected = "hello  world\nname  value";
        assert_eq!(output, expected, "Tabs should be two spaces, got {output:?}");
    }

    #[test]
    fn tab_characters_in_bash_output_are_replaced_with_spaces() {
        // @step Given a bash command outputs text containing tab characters
        let input = given_text_with_tabs();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then each tab character is replaced with two spaces
        then_tabs_replaced_with_two_spaces(&output);

        // @step And the text maintains consistent visual width
        then_consistent_visual_width(&output);
    }

    // ── Scenario: Carriage returns are removed to prevent line overwriting ──

    /// @step Given a bash command outputs text containing carriage return characters
    fn given_text_with_carriage_returns() -> String {
        "line1\r\nline2\rline3\nline4".to_string()
    }

    /// @step Then carriage return characters are removed from the displayed text
    fn then_carriage_returns_removed(output: &str) {
        assert!(
            !output.contains('\r'),
            "No carriage returns should remain, got {:?}",
            output
        );
    }

    /// @step And lines are not overwritten in the TUI
    fn then_lines_not_overwritten(output: &str) {
        let expected = "line1\nline2line3\nline4";
        assert_eq!(output, expected, "Lines should not be overwritten, got {output:?}");
    }

    #[test]
    fn carriage_returns_are_removed_to_prevent_line_overwriting() {
        // @step Given a bash command outputs text containing carriage return characters
        let input = given_text_with_carriage_returns();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then carriage return characters are removed from the displayed text
        then_carriage_returns_removed(&output);

        // @step And lines are not overwritten in the TUI
        then_lines_not_overwritten(&output);
    }

    // ── Scenario: Control characters are removed except newlines ──

    /// @step Given a bash command outputs text containing control characters (0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F)
    fn given_text_with_control_chars() -> String {
        // Contains: NUL, backspace, vertical tab, form feed, ESC-range, DEL
        // Also contains newline (should be preserved)
        String::from_utf8(vec![
            0x00, // NUL
            0x08, // Backspace
            'a' as u8,
            0x0A, // Newline (should be preserved)
            0x0B, // Vertical tab
            'b' as u8,
            0x0C, // Form feed
            'c' as u8,
            0x0E, // Shift out
            'd' as u8,
            0x1F, // Unit separator
            'e' as u8,
            0x7F, // DEL
            'f' as u8,
        ])
        .expect("test string is valid UTF-8")
    }

    /// @step Then all control characters are removed from the displayed text
    fn then_control_chars_removed(output: &str) {
        for c in output.chars() {
            let code = c as u32;
            assert!(
                !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
                "Control character U+{:02X} should be removed, found in {:?}",
                code, output
            );
        }
    }

    /// @step And newline characters are preserved so multi-line output renders correctly
    fn then_newlines_preserved(output: &str) {
        assert!(
            output.contains('\n'),
            "Newlines should be preserved, got {:?}",
            output
        );
    }

    #[test]
    fn control_characters_are_removed_except_newlines() {
        // @step Given a bash command outputs text containing control characters (0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F)
        let input = given_text_with_control_chars();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then all control characters are removed from the displayed text
        then_control_chars_removed(&output);

        // @step And newline characters are preserved so multi-line output renders correctly
        then_newlines_preserved(&output);
    }

    // ── Scenario: Plain text output passes through sanitization unchanged ──

    /// @step Given a bash command outputs plain text without any escape sequences or control characters
    fn given_plain_text() -> String {
        "Hello World\nThis is plain text\nNo special characters here".to_string()
    }

    /// @step Then the text is displayed exactly as output by the command
    fn then_text_displayed_exactly(output: &str, original: &str) {
        assert_eq!(output, original, "Plain text should pass through unchanged");
    }

    /// @step And no characters are removed or modified
    fn then_no_chars_removed(output: &str, original: &str) {
        assert_eq!(
            output.chars().count(),
            original.chars().count(),
            "Character count should be identical"
        );
    }

    #[test]
    fn plain_text_output_passes_through_sanitization_unchanged() {
        // @step Given a bash command outputs plain text without any escape sequences or control characters
        let input = given_plain_text();

        // @step When the tool output is processed for TUI display
        let output = when_processed_for_tui(&input);

        // @step Then the text is displayed exactly as output by the command
        then_text_displayed_exactly(&output, &input);

        // @step And no characters are removed or modified
        then_no_chars_removed(&output, &input);
    }

    // ── Scenario: ToolProgress streaming chunks are also sanitized ──

    /// @step Given a bash command streams output line-by-line via ToolProgress
    fn given_streaming_chunks() -> Vec<String> {
        vec![
            format!("\x1b[32mStep 1\x1b[0m"),
            format!("\x1b[32mStep 2\x1b[0m"),
            format!("\x1b[32mStep 3\x1b[0m"),
        ]
    }

    /// @step When each streaming chunk is processed for TUI display
    fn when_each_chunk_processed(chunks: &[String]) -> Vec<String> {
        chunks.iter().map(|c| sanitize_for_terminal(c)).collect()
    }

    /// @step Then ANSI escape sequences are stripped from each chunk before rendering
    fn then_each_chunk_stripped(chunks: &[String]) {
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(
                !chunk.contains('\x1b'),
                "Chunk {} should have no escape chars, got {:?}",
                i,
                chunk
            );
        }
    }

    /// @step And the user sees clean output in real-time as it streams
    fn then_clean_streaming_output(chunks: &[String]) {
        let expected = vec!["Step 1", "Step 2", "Step 3"];
        assert_eq!(
            chunks,
            &expected,
            "Each chunk should contain clean text"
        );
    }

    #[test]
    fn toolprogress_streaming_chunks_are_also_sanitized() {
        // @step Given a bash command streams output line-by-line via ToolProgress
        let chunks = given_streaming_chunks();

        // @step When each streaming chunk is processed for TUI display
        let sanitized = when_each_chunk_processed(&chunks);

        // @step Then ANSI escape sequences are stripped from each chunk before rendering
        then_each_chunk_stripped(&sanitized);

        // @step And the user sees clean output in real-time as it streams
        then_clean_streaming_output(&sanitized);
    }

    // ── Scenario: LLM receives raw unsanitized output while TUI gets sanitized ──

    /// @step Given a bash command outputs text with ANSI color codes
    fn given_ansi_output_for_llm_and_tui() -> String {
        // Simulates raw bash output with ANSI codes
        format!("\x1b[01;34mDocuments\x1b[0m\n\x1b[01;34mDownloads\x1b[0m")
    }

    /// @step When the tool result is returned to both the LLM and the TUI
    fn when_tool_result_returned_to_both(input: &str) -> (String, String) {
        // LLM receives the raw output unchanged
        let llm_output = input.to_string();
        // TUI receives the sanitized output
        let tui_output = sanitize_for_terminal(input);
        (llm_output, tui_output)
    }

    /// @step Then the LLM receives the raw output with ANSI codes intact
    fn then_llm_receives_raw_output(llm_output: &str) {
        assert!(
            llm_output.contains('\x1b'),
            "LLM output should contain ANSI escape codes, got {:?}",
            llm_output
        );
    }

    /// @step And the TUI scrollback contains only the sanitized plain text
    fn then_tui_receives_sanitized(tui_output: &str) {
        assert!(
            !tui_output.contains('\x1b'),
            "TUI output should not contain ANSI escape codes, got {:?}",
            tui_output
        );
        let expected = "Documents\nDownloads";
        assert_eq!(tui_output, expected, "Only plain text should be in TUI scrollback");
    }

    #[test]
    fn llm_receives_raw_unsanitized_output_while_tui_gets_sanitized() {
        // @step Given a bash command outputs text with ANSI color codes
        let raw_output = given_ansi_output_for_llm_and_tui();

        // @step When the tool result is returned to both the LLM and the TUI
        let (llm_output, tui_output) = when_tool_result_returned_to_both(&raw_output);

        // @step Then the LLM receives the raw output with ANSI codes intact
        then_llm_receives_raw_output(&llm_output);

        // @step And the TUI scrollback contains only the sanitized plain text
        then_tui_receives_sanitized(&tui_output);
    }

    // ── Additional edge case tests ──

    #[test]
    fn empty_string_passes_through_unchanged() {
        let output = sanitize_for_terminal("");
        assert_eq!(output, "");
    }

    #[test]
    fn only_newlines_are_preserved_as_control_chars() {
        // Input has only newlines
        let input = "\n\n\n";
        let output = sanitize_for_terminal(input);
        assert_eq!(output, "\n\n\n");
    }

    #[test]
    fn mixed_ansi_tabs_and_control_chars_all_sanitized() {
        // Combined test: ANSI + tabs + carriage returns + control chars
        let input = format!("\x1b[31mred\x1b[0m\twith\ttabs\r\nand\x07control");
        let output = sanitize_for_terminal(&input);
        assert_eq!(output, "red  with  tabs\nandcontrol");
    }
}
