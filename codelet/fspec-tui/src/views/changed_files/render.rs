//! RPC-356 — dual-pane rendering for the Changed Files view.
//!
//! Feature: spec/features/rust-changed-files-view.feature
//!
//! Splits the body into a left file-list pane and a right diff pane,
//! painting a focus-aware header on each, the file rows + diff rows, and
//! an empty-state message when there are no changed files. Uses the
//! shared full-screen shell scaffold (RPC-337) for the title + footer
//! chrome so it matches the other mode-views.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::views::full_screen_shell::render_full_screen_scaffold_raw_title;

use crate::views::diff_common::{
    diff_line, file_row, pane_header, render_pane_scrollbar, render_vertical_divider,
};
use super::{ChangedFilesView, Pane};

const FOOTER_HINT: &str = "ESC: Back | Tab: Switch Panes | ↑↓: Navigate/Scroll | PgUp/PgDn: Scroll";
const EMPTY_MESSAGE: &str = "No changed files";

impl ChangedFilesView {
    /// Paint the view into `area`. Records the per-pane Rects so the
    /// event layer can hit-test the mouse wheel + compute page steps.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let count = self.files.len();
        let title = format!("Changed Files ({count})");
        // `self` is borrowed mutably to cache rects; the scaffold takes
        // a closure so we capture the pieces we need by raw pointer-free
        // moves first.
        let files = std::mem::take(&mut self.files);
        let diff_lines = std::mem::take(&mut self.diff_lines);
        let selected_index = self.selected_index;
        let focused = self.focused_pane;
        let file_scroll = self.file_scroll;
        let diff_scroll = self.diff_scroll;
        let mut files_rect = None;
        let mut diff_rect = None;
        render_full_screen_scaffold_raw_title(
            area,
            buf,
            &title,
            FOOTER_HINT,
            |body, buf| {
                if files.is_empty() {
                    render_empty(body, buf);
                    return;
                }
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(40),
                        Constraint::Length(1),
                        Constraint::Percentage(60),
                    ])
                    .split(body);
                render_vertical_divider(panes[1], buf);
                files_rect =
                    Some(render_files_pane(panes[0], buf, &files, selected_index, file_scroll, focused));
                diff_rect =
                    Some(render_diff_pane(panes[2], buf, &diff_lines, diff_scroll, focused));
            },
            None,
        );
        self.files = files;
        self.diff_lines = diff_lines;
        self.last_files_rect = files_rect;
        self.last_diff_rect = diff_rect;
    }
}

fn render_empty(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(Span::styled(
        EMPTY_MESSAGE,
        Style::default().fg(Color::DarkGray),
    )))
    .render(area, buf);
}

/// Paint the file-list pane (shared focus-aware header + underline rule,
/// the file rows, and an overflow scrollbar). Returns the content Rect.
fn render_files_pane(
    area: Rect,
    buf: &mut Buffer,
    files: &[codelet_rpc_types::ChangedFile],
    selected_index: usize,
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
        .map(|(idx, f)| file_row(f, idx == selected_index, list_width as usize))
        .collect();
    let list_area = Rect { width: list_width, ..content };
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
    let list_area = Rect { width: list_width, ..content };
    Paragraph::new(lines).render(list_area, buf);
    if overflow {
        render_pane_scrollbar(content, buf, list_width, scroll, visible, diff_lines.len());
    }
    content
}
