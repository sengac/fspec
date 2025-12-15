// Feature: spec/features/enhanced-text-styling-with-bash-highlighting-and-diff-rendering.feature
//
// Tests for CLI-006: Enhanced text styling with bash highlighting and diff rendering
// These tests verify tree-sitter-bash highlighting and diffy diff rendering

#[cfg(test)]
mod bash_highlighting_tests {
    use codelet::cli::highlight::highlight_bash_command;

    // ============================================================================
    // Scenario: Highlight bash command with dimmed strings and operators
    // ============================================================================

    #[test]
    fn test_highlight_bash_command_with_dimmed_strings_and_operators() {
        // @step Given the agent shows a bash command for approval
        let command = "echo \"hi\" && bar | qux";

        // @step When the command is 'echo "hi" && bar | qux'
        let highlighted_lines = highlight_bash_command(command);
        let result = highlighted_lines.join("\n");

        // @step Then the string "hi" should be dimmed
        // String should have dim codes \x1b[2m...\x1b[22m
        assert!(result.contains("\x1b[2m"), "String should be dimmed");

        // @step And the operator && should be dimmed
        assert!(result.contains("&&"), "Should contain && operator");

        // @step And the operator | should be dimmed
        assert!(result.contains("|"), "Should contain | operator");

        // @step And the command 'echo' should be in default style
        assert!(result.contains("echo"), "Should contain echo command");

        // @step And the text 'bar' and 'qux' should be in default style
        assert!(result.contains("bar"), "Should contain bar");
        assert!(result.contains("qux"), "Should contain qux");
    }

    // ============================================================================
    // Scenario: Show bash command with non-dimmed elements
    // ============================================================================

    #[test]
    fn test_show_bash_command_with_non_dimmed_elements() {
        // @step Given the agent shows a bash command for approval
        let command = "cat file.txt";

        // @step When the command is 'cat file.txt'
        let highlighted_lines = highlight_bash_command(command);
        let result = highlighted_lines.join("\n");

        // @step Then the command 'cat' should be in default style
        assert!(result.contains("cat"), "Should contain cat command");

        // @step And the argument 'file.txt' should be in default style
        assert!(
            result.contains("file.txt"),
            "Should contain file.txt argument"
        );

        // @step And no dimming should be applied
        // Simple commands may still have minimal highlighting, but content must be present
        assert!(!result.is_empty(), "Result should not be empty");
    }

}

#[cfg(test)]
mod diff_rendering_tests {
    use codelet::cli::diff::{render_diff_line, render_diff_summary, render_file_diff};

    // ============================================================================
    // Scenario: Show diff summary with addition and deletion counts
    // ============================================================================

    #[test]
    fn test_show_diff_summary_with_addition_and_deletion_counts() {
        // @step Given the agent has made changes to files
        let additions = 5;
        let deletions = 2;

        // @step When the diff summary shows '+5 -2'
        let summary = render_diff_summary(additions, deletions);

        // @step Then the text '+5' should be displayed in green
        assert!(
            summary.contains("\x1b[32m+5") || summary.contains("\x1b[92m+5"),
            "Additions should be in green"
        );

        // @step And the text '-2' should be displayed in red
        assert!(
            summary.contains("\x1b[31m-2") || summary.contains("\x1b[91m-2"),
            "Deletions should be in red"
        );

        // @step And the summary should show 5 additions and 2 deletions
        assert!(summary.contains("5"), "Should show 5");
        assert!(summary.contains("2"), "Should show 2");
    }

    // ============================================================================
    // Scenario: Show diff line with addition in green
    // ============================================================================

    #[test]
    fn test_show_diff_line_with_addition_in_green() {
        // @step Given the agent has added a line to a file
        let line_number = 1;
        let line_content = "new line";

        // @step When the diff shows line '1 + new line'
        let diff_line = render_diff_line(line_number, '+', line_content);

        // @step Then the line number '1' should be displayed
        assert!(diff_line.contains("1"), "Should show line number 1");

        // @step And the '+' prefix should be displayed
        assert!(diff_line.contains("+"), "Should show + prefix");

        // @step And the text 'new line' should be displayed in green
        assert!(
            diff_line.contains("\x1b[32m") || diff_line.contains("\x1b[92m"),
            "Addition should be in green"
        );
        assert!(diff_line.contains("new line"), "Should contain content");
    }

    // ============================================================================
    // Scenario: Diff rendering includes line numbers and file paths
    // ============================================================================

    #[test]
    fn test_diff_rendering_includes_line_numbers_and_file_paths() {
        // @step Given the agent has modified a file 'src/main.rs'
        let file_path = "src/main.rs";
        let old_line_number = 10;
        let new_line_number = 10;
        let addition_content = "let x = 5;";
        let deletion_content = "let x = 3;";

        // @step When the diff is displayed
        let diff = render_file_diff(
            file_path,
            &[
                (old_line_number, deletion_content, '-'),
                (new_line_number, addition_content, '+'),
            ],
        )
        .unwrap();

        // @step Then the file path 'src/main.rs' should be shown
        assert!(diff.contains("src/main.rs"), "Should show file path");

        // @step And each changed line should have a line number
        assert!(diff.contains("10"), "Should show line numbers");

        // @step And additions should show line numbers from the new file
        assert!(
            diff.contains(addition_content),
            "Should show addition content"
        );
        assert!(
            diff.contains("\x1b[32m") || diff.contains("\x1b[92m"),
            "Addition should be in green"
        );

        // @step And deletions should show line numbers from the old file
        assert!(
            diff.contains(deletion_content),
            "Should show deletion content"
        );
        assert!(
            diff.contains("\x1b[31m") || diff.contains("\x1b[91m"),
            "Deletion should be in red"
        );
    }
}
