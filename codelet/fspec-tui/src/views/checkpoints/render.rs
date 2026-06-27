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
use crate::views::diff_common::{
    diff_line, file_row, pane_header, render_pane_scrollbar, render_vertical_divider,
};
use crate::views::full_screen_shell::render_full_screen_scaffold_raw_title;

use super::checkpoint_row::checkpoint_label;
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
                cp_rect = Some(render_checkpoints_pane(
                    top[0],
                    buf,
                    &checkpoints,
                    selected_cp,
                    cp_scroll,
                    focused,
                ));
                files_rect = Some(render_files_pane(
                    top[2],
                    buf,
                    &files,
                    selected_file,
                    file_scroll,
                    focused,
                ));
                diff_rect = Some(render_diff_pane(
                    rows[1],
                    buf,
                    &diff_lines,
                    diff_scroll,
                    focused,
                ));
            },
            None,
        );
        self.checkpoints = checkpoints;
        self.files = files;
        self.diff_lines = diff_lines;
        self.last_checkpoints_rect = cp_rect;
        self.last_files_rect = files_rect;
        self.last_diff_rect = diff_rect;
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
) -> Rect {
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
    if overflow {
        render_pane_scrollbar(content, buf, list_width, scroll, visible, checkpoints.len());
    }
    content
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
    let label = checkpoint_label(cp);
    let avail = width.saturating_sub(2);
    let text = crate::views::diff_common::truncate_path(&label, avail);
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
) -> Rect {
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
    if overflow {
        render_pane_scrollbar(content, buf, list_width, scroll, visible, files.len());
    }
    content
}

fn render_diff_pane(
    area: Rect,
    buf: &mut Buffer,
    diff_lines: &[String],
    scroll: usize,
    focused: Pane,
) -> Rect {
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
    if overflow {
        render_pane_scrollbar(content, buf, list_width, scroll, visible, diff_lines.len());
    }
    content
}
