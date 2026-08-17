//! RPC-356 / RPC-363 — file-row formatting shared across the Changed
//! Files and Checkpoints list panes.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Builds one ratatui `Line` per changed file: a selection cursor
//! (`>`/space), a colored single-letter status (A=green, M=yellow,
//! D=red, R=cyan; default M=yellow), then the (truncated) path. Mirrors
//! the TS `ChangedFilesViewer.renderFileItem`.

use crate::sanitize_for_terminal;
use codelet_rpc_types::ChangedFile;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Map a single-letter change type to its UI color.
///
/// A=green, M=yellow, D=red, R=cyan; anything else defaults to yellow
/// (matching the TS `default` arm).
pub fn status_color(change_type: &str) -> Color {
    match change_type {
        "A" => Color::Green,
        "M" => Color::Yellow,
        "D" => Color::Red,
        "R" => Color::Cyan,
        _ => Color::Yellow,
    }
}

/// Truncate `path` to fit `max_width` columns, appending an ellipsis
/// when it overflows. `max_width == 0` yields an empty string.
pub fn truncate_path(path: &str, max_width: usize) -> String {
    let len = path.width();
    if max_width == 0 {
        return String::new();
    }
    if len <= max_width {
        return path.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let keep = max_width - 1;
    let truncated: String = path.chars().take(keep).collect();
    format!("{truncated}…")
}

/// Build the styled `Line` for one file row. `selected` drives the
/// cursor glyph + the row foreground (cyan when selected, white
/// otherwise). `width` is the pane width used to truncate the path.
///
/// Sanitizes the file path and change type via [`crate::sanitize_for_terminal`]
/// before rendering to prevent control characters from corrupting the display.
pub fn file_row(file: &ChangedFile, selected: bool, width: usize) -> Line<'static> {
    let cursor = if selected { ">" } else { " " };
    let row_fg = if selected { Color::Cyan } else { Color::White };
    // Sanitize before truncating so control chars are removed from width calc.
    let sanitized_path = sanitize_for_terminal(&file.path);
    let sanitized_change = sanitize_for_terminal(&file.change_type);
    // Account for "> " + "X " prefixes (4 columns) when truncating.
    let path_width = width.saturating_sub(4);
    let path = truncate_path(&sanitized_path, path_width);
    let status_style = Style::default().fg(status_color(&sanitized_change));
    let mut row_style = Style::default().fg(row_fg);
    if selected {
        row_style = row_style.add_modifier(Modifier::BOLD);
    }
    Line::from(vec![
        Span::styled(format!("{cursor} "), row_style),
        Span::styled(sanitized_change, status_style),
        Span::styled(" ", row_style),
        Span::styled(path, row_style),
    ])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn status_color_maps_each_letter() {
        assert_eq!(status_color("A"), Color::Green);
        assert_eq!(status_color("M"), Color::Yellow);
        assert_eq!(status_color("D"), Color::Red);
        assert_eq!(status_color("R"), Color::Cyan);
        assert_eq!(status_color("?"), Color::Yellow);
    }

    #[test]
    fn truncate_path_appends_ellipsis_when_too_long() {
        assert_eq!(truncate_path("short.txt", 20), "short.txt");
        assert_eq!(truncate_path("verylongpath.txt", 5), "very…");
        assert_eq!(truncate_path("anything", 0), "");
    }

    // Feature: spec/features/sanitize-file-paths-and-labels-in-changed-files-and-checkpoint-views.feature

    // ── Scenario: File paths with unusual characters display cleanly in the Changed Files view ──

    /// @step Given I have a changed file with a path containing control characters or ANSI sequences
    fn given_file_with_control_chars_in_path() -> ChangedFile {
        ChangedFile {
            path: "path\x1b[31mwith\x1b[0mansi.txt".to_string(),
            change_type: "M".to_string(),
            staged: false,
        }
    }

    /// @step When I open the Changed Files view
    /// @step Then the file list displays the path without control characters
    fn then_file_row_has_no_control_chars(file: &ChangedFile) {
        let line = file_row(file, false, 40);
        for span in &line.spans {
            let text = span.content.as_ref();
            for c in text.chars() {
                let code = c as u32;
                assert!(
                    !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
                    "File row should not contain control character U+{code:02X}, found in {text:?}");
            }
        }
    }

    /// @step And the terminal display is not corrupted
    fn then_terminal_not_corrupted_file_path(file: &ChangedFile) {
        let line = file_row(file, false, 40);
        // Verify readable content remains
        let span_texts: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(span_texts.contains("path"));
        assert!(span_texts.contains("with"));
        assert!(span_texts.contains("ansi"));
    }

    #[test]
    fn file_paths_with_unusual_characters_display_cleanly() {
        // @step Given I have a changed file with a path containing control characters or ANSI sequences
        let file = given_file_with_control_chars_in_path();

        // @step When I open the Changed Files view
        // @step Then the file list displays the path without control characters
        then_file_row_has_no_control_chars(&file);

        // @step And the terminal display is not corrupted
        then_terminal_not_corrupted_file_path(&file);
    }

    // ── Scenario: Checkpoint labels with special characters display cleanly in the Checkpoint view ──

    /// @step Given I have a checkpoint with a label containing control characters or ANSI sequences
    fn given_checkpoint_with_control_chars_in_label() -> ChangedFile {
        ChangedFile {
            path: "file\x00with\x08control.txt".to_string(),
            change_type: "A".to_string(),
            staged: false,
        }
    }

    /// @step When I open the Checkpoint view
    /// @step Then the checkpoint list displays the label without control characters
    fn then_file_row_removes_control_chars(file: &ChangedFile) {
        let line = file_row(file, false, 40);
        for span in &line.spans {
            let text = span.content.as_ref();
            assert!(
                !text.contains('\x00'),
                "File row should not contain NUL, got {text:?}");
            assert!(
                !text.contains('\x08'),
                "File row should not contain backspace, got {text:?}");
        }
    }

    /// @step And the terminal display is not corrupted
    fn then_terminal_not_corrupted_checkpoint(file: &ChangedFile) {
        let line = file_row(file, false, 40);
        let span_texts: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(span_texts.contains("file"));
        assert!(span_texts.contains("with"));
        assert!(span_texts.contains("control"));
    }

    #[test]
    fn checkpoint_labels_with_special_characters_display_cleanly() {
        // @step Given I have a checkpoint with a label containing control characters or ANSI sequences
        let file = given_checkpoint_with_control_chars_in_label();

        // @step When I open the Checkpoint view
        // @step Then the checkpoint list displays the label without control characters
        then_file_row_removes_control_chars(&file);

        // @step And the terminal display is not corrupted
        then_terminal_not_corrupted_checkpoint(&file);
    }
}
