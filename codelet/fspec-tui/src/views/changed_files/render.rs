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
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::views::full_screen_shell::render_full_screen_scaffold_raw_title;

use super::diff_render::diff_line;
use super::row::file_row;
use super::{ChangedFilesView, Pane};

const FOOTER_HINT: &str = "ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll";
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
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(body);
                files_rect =
                    Some(render_files_pane(panes[0], buf, &files, selected_index, file_scroll, focused));
                diff_rect =
                    Some(render_diff_pane(panes[1], buf, &diff_lines, diff_scroll, focused));
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

/// Paint a 1-row pane header that highlights when focused. Returns the
/// content Rect below the header.
fn pane_header(area: Rect, buf: &mut Buffer, label: &str, focused: bool) -> Rect {
    if area.height == 0 {
        return area;
    }
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    Paragraph::new(Line::from(Span::styled(label.to_string(), style))).render(split[0], buf);
    split[1]
}

fn render_files_pane(
    area: Rect,
    buf: &mut Buffer,
    files: &[codelet_rpc_types::ChangedFile],
    selected_index: usize,
    scroll: usize,
    focused: Pane,
) -> Rect {
    let content = pane_header(area, buf, "Files", focused == Pane::Files);
    let width = content.width as usize;
    let visible = content.height as usize;
    let lines: Vec<Line<'_>> = files
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(idx, f)| file_row(f, idx == selected_index, width))
        .collect();
    Paragraph::new(lines).render(content, buf);
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
    let lines: Vec<Line<'_>> = diff_lines
        .iter()
        .skip(scroll)
        .take(visible)
        .map(|l| diff_line(l))
        .collect();
    Paragraph::new(lines).render(content, buf);
    content
}
