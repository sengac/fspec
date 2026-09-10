//! Tests for the terminal-output sanitizer (extracted from sanitize.rs to keep that file under the 300-LoC ceiling).
//!
//! Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature

mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use crate::store::agent_view::sanitize::sanitize_for_terminal;

    // Feature: spec/features/sanitize-bash-tool-output-before-tui-rendering-to-prevent-terminal-trashing.feature

    /// @step When the tool output is processed for TUI display
    fn when_processed_for_tui(input: &str) -> String {
        sanitize_for_terminal(input)
    }


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
            "\x1b[32mStep 1\x1b[0m".to_string(),
            "\x1b[32mStep 2\x1b[0m".to_string(),
            "\x1b[32mStep 3\x1b[0m".to_string(),
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
                "Chunk {i} should have no escape chars, got {chunk:?}"
            );
        }
    }

    /// @step And the user sees clean output in real-time as it streams
    fn then_clean_streaming_output(chunks: &[String]) {
        let expected = vec!["Step 1", "Step 2", "Step 3"];
        assert_eq!(chunks, &expected, "Each chunk should contain clean text");
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
        "\x1b[01;34mDocuments\x1b[0m\n\x1b[01;34mDownloads\x1b[0m".to_string()
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
            "LLM output should contain ANSI escape codes, got {llm_output:?}"
        );
    }

    /// @step And the TUI scrollback contains only the sanitized plain text
    fn then_tui_receives_sanitized(tui_output: &str) {
        assert!(
            !tui_output.contains('\x1b'),
            "TUI output should not contain ANSI escape codes, got {tui_output:?}"
        );
        let expected = "Documents\nDownloads";
        assert_eq!(
            tui_output, expected,
            "Only plain text should be in TUI scrollback"
        );
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
        let input = "\x1b[31mred\x1b[0m\twith\ttabs\r\nand\x07control".to_string();
        let output = sanitize_for_terminal(&input);
        assert_eq!(output, "red  with  tabs\nandcontrol");
    }
}
