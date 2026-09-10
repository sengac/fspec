//! Tests for the terminal-output sanitizer (extracted from sanitize.rs to keep that file under the 300-LoC ceiling).
//!
//! Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature

mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use crate::store::agent_view::sanitize::sanitize_for_terminal;

    // Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature

    // ── Scenario: ANSI color codes from ls --color are stripped before rendering ──

    /// @step Given a bash command outputs text with ANSI color escape sequences like "\x1b[01;34m" for colored directories
    fn given_ansi_color_sequences() -> String {
        // Simulates `ls --color=always` output
        "\x1b[01;34mDocuments\x1b[0m\n\x1b[01;34mDownloads\x1b[0m\nfile.txt".to_string()
    }

    /// @step When the tool output is processed for TUI display
    fn when_processed_for_tui(input: &str) -> String {
        sanitize_for_terminal(input)
    }

    /// @step Then the ANSI escape sequences are removed from the displayed text
    fn then_ansi_sequences_removed(output: &str) {
        assert!(
            !output.contains('\x1b'),
            "Output should not contain escape characters, got {output:?}"
        );
    }

    /// @step And only the plain text content (filenames) is visible in the TUI
    fn then_only_plain_text_visible(output: &str) {
        let expected = "Documents\nDownloads\nfile.txt";
        assert_eq!(
            output, expected,
            "Only plain text should remain, got {output:?}"
        );
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
        "\x1b[1mOS:\x1b[0m Ubuntu 24.04\n\
             \x1b[34mKernel:\x1b[0m 6.8.0\n\
             \x1b[2J\x1b[H\
             \x1b[32mCPU:\x1b[0m Apple M2\n\
             \x1b]0;Terminal Title\x07\
             Memory: 8GB"
            .to_string()
    }

    /// @step Then all ANSI escape sequences are removed from the displayed text
    fn then_all_ansi_removed(output: &str) {
        assert!(
            !output.contains('\x1b'),
            "No escape characters should remain, got {output:?}"
        );
        assert!(
            !output.contains('\x07'),
            "BEL character from OSC should be removed, got {output:?}"
        );
    }

    /// @step And only readable plain text is visible in the TUI
    fn then_only_readable_text(output: &str) {
        let expected = "OS: Ubuntu 24.04\nKernel: 6.8.0\nCPU: Apple M2\nMemory: 8GB";
        assert_eq!(
            output, expected,
            "Only readable text should remain, got {output:?}"
        );
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
            "No tabs should remain, got {output:?}"
        );
    }

    /// @step And the text maintains consistent visual width
    fn then_consistent_visual_width(output: &str) {
        let expected = "hello  world\nname  value";
        assert_eq!(
            output, expected,
            "Tabs should be two spaces, got {output:?}"
        );
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
            "No carriage returns should remain, got {output:?}"
        );
    }

    /// @step And lines are not overwritten in the TUI
    fn then_lines_not_overwritten(output: &str) {
        let expected = "line1\nline2line3\nline4";
        assert_eq!(
            output, expected,
            "Lines should not be overwritten, got {output:?}"
        );
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
            b'a', 0x0A, // Newline (should be preserved)
            0x0B, // Vertical tab
            b'b', 0x0C, // Form feed
            b'c', 0x0E, // Shift out
            b'd', 0x1F, // Unit separator
            b'e', 0x7F, // DEL
            b'f',
        ])
        .expect("test string is valid UTF-8")
    }

    /// @step Then all control characters are removed from the displayed text
    fn then_control_chars_removed(output: &str) {
        for c in output.chars() {
            let code = c as u32;
            assert!(
                !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
                "Control character U+{code:02X} should be removed, found in {output:?}"
            );
        }
    }

    /// @step And newline characters are preserved so multi-line output renders correctly
    fn then_newlines_preserved(output: &str) {
        assert!(
            output.contains('\n'),
            "Newlines should be preserved, got {output:?}"
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

}
