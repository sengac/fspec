//! RPC-364 — three-pane rendering for the Checkpoints view.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! Top row: a Checkpoints list pane (left) | a Files list pane (right).
//! Bottom: a full-width Diff pane. Each pane paints a focus-aware header
//! (highlighted when focused) and reuses the shared `file_row` / `diff_line`
//! helpers + the pane-scrollbar gutter (shown only on overflow) from
//! `crate::views::diff_common` (RPC-363). Uses the full-screen shell
//! scaffold (RPC-337) for the title + footer chrome.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::checkpoint_restore_dialog::render_restore_modal;
use crate::sanitize_for_terminal;
use crate::views::diff_common::{
    diff_line, file_row, pane_header, render_pane_scrollbar, render_vertical_divider,
};
use crate::views::full_screen_shell::render_full_screen_scaffold_raw_title;

use super::{CheckpointsView, Pane};

const FOOTER_HINT: &str =
    "ESC: Back | Tab/→: Next Pane | ←: Prev Pane | ↑↓: Navigate/Scroll | r: Restore File | t: Restore All | d: Delete | a: Delete All";
const EMPTY_MESSAGE: &str = "No checkpoints available";

impl CheckpointsView {
    /// Paint the view into `area`, caching per-pane Rects for wheel
    /// hit-testing + page-step math.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let count = self.checkpoints.len();
        let title = format!("Checkpoints ({count})");
        let checkpoints = std::mem::take(&mut self.checkpoints);
        let files = std::mem::take(&mut self.files);
        let diff_lines = std::mem::take(&mut self.diff_lines);
        let selected_cp = self.selected_checkpoint;
        let selected_file = self.selected_file;
        let cp_scroll = self.checkpoint_scroll;
        let file_scroll = self.file_scroll;
        let diff_scroll = self.diff_scroll;
        let focused = self.focused_pane;
        let mut cp_rect = None;
        let mut files_rect = None;
        let mut diff_rect = None;
        let mut cp_sb_rect = None;
        let mut files_sb_rect = None;
        let mut diff_sb_rect = None;
        render_full_screen_scaffold_raw_title(
            area,
            buf,
            &title,
            FOOTER_HINT,
            |body, buf| {
                if checkpoints.is_empty() {
                    render_empty(body, buf);
                    return;
                }
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(body);
                let top = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(40),
                        Constraint::Length(1),
                        Constraint::Percentage(60),
                    ])
                    .split(rows[0]);
                render_vertical_divider(top[1], buf);
                let (cr, csb) = render_checkpoints_pane(
                    top[0],
                    buf,
                    &checkpoints,
                    selected_cp,
                    cp_scroll,
                    focused,
                );
                cp_rect = Some(cr);
                cp_sb_rect = csb;
                let (fr, fsb) = render_files_pane(
                    top[2],
                    buf,
                    &files,
                    selected_file,
                    file_scroll,
                    focused,
                );
                files_rect = Some(fr);
                files_sb_rect = fsb;
                let (dr, dsb) = render_diff_pane(
                    rows[1],
                    buf,
                    &diff_lines,
                    diff_scroll,
                    focused,
                );
                diff_rect = Some(dr);
                diff_sb_rect = dsb;
            },
            None,
        );
        self.checkpoints = checkpoints;
        self.files = files;
        self.diff_lines = diff_lines;
        self.last_checkpoints_rect = cp_rect;
        self.last_files_rect = files_rect;
        self.last_diff_rect = diff_rect;
        self.last_cp_sb_rect = cp_sb_rect;
        self.last_files_sb_rect = files_sb_rect;
        self.last_diff_sb_rect = diff_sb_rect;
        // RPC-365: paint the restore confirmation/status modal over the
        // panes when active so it captures focus visually too.
        if let Some(dialog) = self.dialog() {
            render_restore_modal(area, buf, dialog.title(), &dialog.body_lines());
        }
        // RPC-366: paint the delete confirmation/status modal (reusing the
        // shared modal renderer) when active.
        if let Some(dialog) = self.delete_dialog() {
            render_restore_modal(area, buf, dialog.title(), &dialog.body_lines());
        }
    }
}

fn render_empty(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(Span::styled(
        EMPTY_MESSAGE,
        Style::default().fg(Color::DarkGray),
    )))
    .render(area, buf);
}

fn render_checkpoints_pane(
    area: Rect,
    buf: &mut Buffer,
    checkpoints: &[codelet_rpc_types::CheckpointInfo],
    selected: usize,
    scroll: usize,
    focused: Pane,
) -> (Rect, Option<Rect>) {
    let content = pane_header(area, buf, "Checkpoints", focused == Pane::Checkpoints);
    let visible = content.height as usize;
    let overflow = checkpoints.len() > visible;
    let list_width = if overflow {
        content.width.saturating_sub(1)
    } else {
        content.width
    };
    let lines: Vec<Line<'_>> = checkpoints
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(idx, c)| checkpoint_line(c, idx == selected, list_width as usize))
        .collect();
    let list_area = Rect {
        width: list_width,
        ..content
    };
    Paragraph::new(lines).render(list_area, buf);
    let sb_rect = if overflow {
        let sb = Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        };
        render_pane_scrollbar(content, buf, list_width, scroll, visible, checkpoints.len());
        Some(sb)
    } else {
        None
    };
    (content, sb_rect)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /**
     * Feature: spec/features/sanitize-file-paths-and-labels-in-changed-files-and-checkpoint-views.feature
     */

    // ── Scenario: Checkpoint labels with special characters display cleanly in the Checkpoint view ──

    /// @step Given I have a checkpoint with a label containing control characters or ANSI sequences
    fn given_checkpoint_with_control_chars_in_label() -> codelet_rpc_types::CheckpointInfo {
        codelet_rpc_types::CheckpointInfo {
            work_unit_id: "TUI-105".to_string(),
            name: format!("label\x00with\x08control"),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            is_automatic: false,
        }
    }

    /// @step When I open the Checkpoint view
    /// @step Then the checkpoint list displays the label without control characters
    fn then_checkpoint_line_removes_control_chars(cp: &codelet_rpc_types::CheckpointInfo) {
        let line = checkpoint_line(cp, false, 40);
        for span in &line.spans {
            let text = span.content.as_ref();
            assert!(
                !text.contains('\x00'),
                "Checkpoint line should not contain NUL, got {:?}",
                text
            );
            assert!(
                !text.contains('\x08'),
                "Checkpoint line should not contain backspace, got {:?}",
                text
            );
        }
    }

    /// @step And the terminal display is not corrupted
    fn then_terminal_not_corrupted_checkpoint_label(cp: &codelet_rpc_types::CheckpointInfo) {
        let line = checkpoint_line(cp, false, 40);
        let span_texts: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(span_texts.contains("label"));
        assert!(span_texts.contains("with"));
        assert!(span_texts.contains("control"));
    }

    #[test]
    fn checkpoint_labels_with_special_characters_display_cleanly() {
        // @step Given I have a checkpoint with a label containing control characters or ANSI sequences
        let cp = given_checkpoint_with_control_chars_in_label();

        // @step When I open the Checkpoint view
        // @step Then the checkpoint list displays the label without control characters
        then_checkpoint_line_removes_control_chars(&cp);

        // @step And the terminal display is not corrupted
        then_terminal_not_corrupted_checkpoint_label(&cp);
    }
}

fn checkpoint_line(
    cp: &codelet_rpc_types::CheckpointInfo,
    selected: bool,
    width: usize,
) -> Line<'static> {
    let cursor = if selected { ">" } else { " " };
    let fg = if selected { Color::Cyan } else { Color::White };
    let mut style = Style::default().fg(fg);
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    let label = crate::views::checkpoints::checkpoint_label(cp);
    // Sanitize before truncating so control chars are removed from width calc.
    let sanitized = sanitize_for_terminal(&label);
    let avail = width.saturating_sub(2);
    let text = crate::views::diff_common::truncate_path(&sanitized, avail);
    Line::from(vec![
        Span::styled(format!("{cursor} "), style),
        Span::styled(text, style),
    ])
}

fn render_files_pane(
    area: Rect,
    buf: &mut Buffer,
    files: &[codelet_rpc_types::ChangedFile],
    selected: usize,
    scroll: usize,
    focused: Pane,
) -> (Rect, Option<Rect>) {
    let content = pane_header(area, buf, "Files", focused == Pane::Files);
    let visible = content.height as usize;
    let overflow = files.len() > visible;
    let list_width = if overflow {
        content.width.saturating_sub(1)
    } else {
        content.width
    };
    let lines: Vec<Line<'_>> = files
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(idx, f)| file_row(f, idx == selected, list_width as usize))
        .collect();
    let list_area = Rect {
        width: list_width,
        ..content
    };
    Paragraph::new(lines).render(list_area, buf);
    let sb_rect = if overflow {
        let sb = Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        };
        render_pane_scrollbar(content, buf, list_width, scroll, visible, files.len());
        Some(sb)
    } else {
        None
    };
    (content, sb_rect)
}

fn render_diff_pane(
    area: Rect,
    buf: &mut Buffer,
    diff_lines: &[String],
    scroll: usize,
    focused: Pane,
) -> (Rect, Option<Rect>) {
    let content = pane_header(area, buf, "Diff", focused == Pane::Diff);
    let visible = content.height as usize;
    let overflow = diff_lines.len() > visible;
    let list_width = if overflow {
        content.width.saturating_sub(1)
    } else {
        content.width
    };
    let lines: Vec<Line<'_>> = diff_lines
        .iter()
        .skip(scroll)
        .take(visible)
        .map(|l| diff_line(l))
        .collect();
    let list_area = Rect {
        width: list_width,
        ..content
    };
    Paragraph::new(lines).render(list_area, buf);
    let sb_rect = if overflow {
        let sb = Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        };
        render_pane_scrollbar(content, buf, list_width, scroll, visible, diff_lines.len());
        Some(sb)
    } else {
        None
    };
    (content, sb_rect)
}
