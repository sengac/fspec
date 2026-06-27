//! RPC-356 / RPC-363 — file-row formatting shared across the Changed
//! Files and Checkpoints list panes.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Builds one ratatui `Line` per changed file: a selection cursor
//! (`>`/space), a colored single-letter status (A=green, M=yellow,
//! D=red, R=cyan; default M=yellow), then the (truncated) path. Mirrors
//! the TS `ChangedFilesViewer.renderFileItem`.

use codelet_rpc_types::ChangedFile;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

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
    let len = path.chars().count();
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
pub fn file_row(file: &ChangedFile, selected: bool, width: usize) -> Line<'_> {
    let cursor = if selected { ">" } else { " " };
    let row_fg = if selected { Color::Cyan } else { Color::White };
    // Account for "> " + "X " prefixes (4 columns) when truncating.
    let path_width = width.saturating_sub(4);
    let path = truncate_path(&file.path, path_width);
    let status_style = Style::default().fg(status_color(&file.change_type));
    let mut row_style = Style::default().fg(row_fg);
    if selected {
        row_style = row_style.add_modifier(Modifier::BOLD);
    }
    Line::from(vec![
        Span::styled(format!("{cursor} "), row_style),
        Span::styled(file.change_type.clone(), status_style),
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
}
