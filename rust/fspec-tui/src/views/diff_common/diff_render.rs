//! RPC-356 / RPC-363 — colored diff-line rendering shared across the
//! Changed Files and Checkpoints diff panes.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Classifies a raw unified-diff line and produces a styled ratatui
//! `Line`: `+` add lines green, `-` remove lines red, `@@` hunk headers
//! (and `--- ` / `+++ ` file headers) dim/cyan, context lines default.
//! Mirrors the TS `diff-parser` + `FileDiffViewer.renderDiffLine`.

use crate::sanitize_for_terminal;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// The visual classification of a single diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Hunk,
    Context,
}

/// Classify a raw diff line by its leading character(s).
///
/// File headers (`--- `/`+++ `) are treated as hunk metadata so they do
/// NOT colour as add/remove content (matches the TS parser).
pub fn classify(line: &str) -> DiffLineKind {
    if line.starts_with("@@") || line.starts_with("--- ") || line.starts_with("+++ ") {
        DiffLineKind::Hunk
    } else if line.starts_with('+') {
        DiffLineKind::Added
    } else if line.starts_with('-') {
        DiffLineKind::Removed
    } else {
        DiffLineKind::Context
    }
}

/// Build the styled `Line` for one diff row.
///
/// Sanitizes the raw diff text via [`crate::sanitize_for_terminal`] before
/// rendering to prevent ANSI escape sequences, control characters, tabs,
/// and carriage returns from trashing the terminal display.
pub fn diff_line(text: &str) -> Line<'static> {
    let sanitized = sanitize_for_terminal(text);
    let style = match classify(&sanitized) {
        DiffLineKind::Added => Style::default().fg(Color::Green),
        DiffLineKind::Removed => Style::default().fg(Color::Red),
        DiffLineKind::Hunk => Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        DiffLineKind::Context => Style::default().fg(Color::White),
    };
    Line::from(Span::styled(sanitized, style))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::sanitize_for_terminal;

    #[test]
    fn classify_recognizes_each_kind() {
        assert_eq!(classify("+added"), DiffLineKind::Added);
        assert_eq!(classify("-removed"), DiffLineKind::Removed);
        assert_eq!(classify("@@ -1,2 +1,3 @@"), DiffLineKind::Hunk);
        assert_eq!(classify("--- a/file"), DiffLineKind::Hunk);
        assert_eq!(classify("+++ b/file"), DiffLineKind::Hunk);
        assert_eq!(classify(" context"), DiffLineKind::Context);
    }

    // Feature: spec/features/sanitize-diff-output-in-changed-files-and-checkpoint-views.feature

    // ── Scenario: Diff lines with ANSI color codes display cleanly in the Changed Files view ──

    /// @step Given I have a changed file whose diff contains ANSI escape sequences like "\x1b[31m" for colored content
    fn given_diff_with_ansi_sequences() -> String {
        "+\x1b[31mcolored content\x1b[0m".to_string()
    }

    /// @step When I open the Changed Files view and select that file
    /// @step Then the diff pane displays the text content without ANSI escape sequences
    fn then_diff_line_has_no_ansi(input: &str) {
        let line = diff_line(input);
        let span_text = line.spans[0].content.as_ref();
        assert!(
            !span_text.contains('\x1b'),
            "Diff line should not contain ANSI escape sequences, got {span_text:?}");
    }

    /// @step And the terminal display is not corrupted by escape sequences
    fn then_terminal_not_corrupted_ansi(input: &str) {
        let sanitized = sanitize_for_terminal(input);
        let line = diff_line(&sanitized);
        let span_text = line.spans[0].content.as_ref();
        // Verify the text is clean readable content
        assert!(span_text.contains("colored content"));
    }

    #[test]
    fn diff_lines_with_ansi_color_codes_display_cleanly() {
        // @step Given I have a changed file whose diff contains ANSI escape sequences like "\x1b[31m" for colored content
        let input = given_diff_with_ansi_sequences();

        // @step When I open the Changed Files view and select that file
        // @step Then the diff pane displays the text content without ANSI escape sequences
        then_diff_line_has_no_ansi(&input);

        // @step And the terminal display is not corrupted by escape sequences
        then_terminal_not_corrupted_ansi(&input);
    }

    // ── Scenario: Diff lines with tab characters display with consistent spacing in the Checkpoint view ──

    /// @step Given I have a checkpoint with a file diff that contains tab characters
    fn given_diff_with_tabs() -> String {
        "+hello\tworld\tvalue".to_string()
    }

    /// @step When I open the Checkpoint view and select the file
    /// @step Then the diff pane displays two spaces instead of each tab character
    fn then_tabs_replaced_with_spaces(input: &str) {
        let line = diff_line(input);
        let span_text = line.spans[0].content.as_ref();
        assert!(
            !span_text.contains('\t'),
            "Diff line should not contain tab characters, got {span_text:?}");
        assert!(span_text.contains("  "), "Tabs should be replaced with two spaces");
    }

    /// @step And the terminal display maintains consistent visual width
    fn then_consistent_visual_width(input: &str) {
        let sanitized = sanitize_for_terminal(input);
        let line = diff_line(&sanitized);
        let span_text = line.spans[0].content.as_ref();
        // Verify no tabs remain
        assert!(!span_text.contains('\t'));
    }

    #[test]
    fn diff_lines_with_tab_characters_display_with_consistent_spacing() {
        // @step Given I have a checkpoint with a file diff that contains tab characters
        let input = given_diff_with_tabs();

        // @step When I open the Checkpoint view and select the file
        // @step Then the diff pane displays two spaces instead of each tab character
        then_tabs_replaced_with_spaces(&input);

        // @step And the terminal display maintains consistent visual width
        then_consistent_visual_width(&input);
    }

    // ── Scenario: Diff lines with carriage returns display without line overwriting ──

    /// @step Given I have a changed file whose diff contains carriage return characters
    fn given_diff_with_carriage_returns() -> String {
        "+line1\r\nline2\rline3".to_string()
    }

    /// @step When I open the Changed Files view and select that file
    /// @step Then the diff pane displays the content without line overwriting
    fn then_no_line_overwriting(input: &str) {
        let line = diff_line(input);
        let span_text = line.spans[0].content.as_ref();
        assert!(
            !span_text.contains('\r'),
            "Diff line should not contain carriage returns, got {span_text:?}");
    }

    /// @step And each line appears on its own row in the terminal
    fn then_each_line_on_own_row(input: &str) {
        let sanitized = sanitize_for_terminal(input);
        let line = diff_line(&sanitized);
        let span_text = line.spans[0].content.as_ref();
        // Carriage returns removed, newlines preserved
        assert!(!span_text.contains('\r'));
        assert!(span_text.contains('\n') || !input.contains('\r'));
    }

    #[test]
    fn diff_lines_with_carriage_returns_display_without_line_overwriting() {
        // @step Given I have a changed file whose diff contains carriage return characters
        let input = given_diff_with_carriage_returns();

        // @step When I open the Changed Files view and select that file
        // @step Then the diff pane displays the content without line overwriting
        then_no_line_overwriting(&input);

        // @step And each line appears on its own row in the terminal
        then_each_line_on_own_row(&input);
    }

    // ── Scenario: Diff lines with control characters display without corrupted rendering ──

    /// @step Given I have a checkpoint with a file diff that contains control characters like NUL or backspace
    fn given_diff_with_control_chars() -> String {
        "+text\x00with\x08control".to_string()
    }

    /// @step When I open the Checkpoint view and select the file
    /// @step Then the diff pane displays the content with control characters removed
    fn then_control_chars_removed(input: &str) {
        let line = diff_line(input);
        let span_text = line.spans[0].content.as_ref();
        for c in span_text.chars() {
            let code = c as u32;
            assert!(
                !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
                "Control character U+{code:02X} should be removed, found in {span_text:?}");
        }
    }

    /// @step And the terminal display is not corrupted
    fn then_terminal_not_corrupted_control(input: &str) {
        let sanitized = sanitize_for_terminal(input);
        let line = diff_line(&sanitized);
        let span_text = line.spans[0].content.as_ref();
        // Verify readable content remains
        assert!(span_text.contains("text"));
        assert!(span_text.contains("with"));
        assert!(span_text.contains("control"));
    }

    #[test]
    fn diff_lines_with_control_characters_display_without_corrupted_rendering() {
        // @step Given I have a checkpoint with a file diff that contains control characters like NUL or backspace
        let input = given_diff_with_control_chars();

        // @step When I open the Checkpoint view and select the file
        // @step Then the diff pane displays the content with control characters removed
        then_control_chars_removed(&input);

        // @step And the terminal display is not corrupted
        then_terminal_not_corrupted_control(&input);
    }
}
