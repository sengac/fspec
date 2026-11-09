//! Shared fixtures for the changed_files test modules (COV-075 split).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

pub(super) fn cf(path: &str, ct: &str, staged: bool) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: ct.to_string(),
        staged,
    }
}

pub(super) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

/// Render the view to a `(joined symbols, per-cell fg colors)` grid.
pub(super) fn render_grid(
    view: &mut ChangedFilesView,
    w: u16,
    h: u16,
) -> (String, Vec<(String, Color)>) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    let mut cells = Vec::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            joined.push_str(cell.symbol());
            cells.push((cell.symbol().to_string(), cell.fg));
        }
        joined.push('\n');
    }
    (joined, cells)
}

pub(super) fn color_of(cells: &[(String, Color)], symbol: &str) -> Option<Color> {
    cells.iter().find(|(s, _)| s == symbol).map(|(_, c)| *c)
}

/// Render the view and return a per-row `(text, leading-cell fg)` list
/// for the diff pane content. Lets tests assert the colour of the first
/// glyph of a specific diff line (e.g. the `+` of `+newline`) without
/// being fooled by an identical glyph elsewhere on the screen.
pub(super) fn diff_rows(view: &mut ChangedFilesView, w: u16, h: u16) -> Vec<(String, Color)> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let mut text = String::new();
        let mut leading: Option<Color> = None;
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            // Skip structural chrome (pane divider '│' / heading rule '─')
            // so the leading colour reflects the diff line's own first glyph
            // (RPC-367 added an inter-pane divider to the left of this pane).
            if leading.is_none() && sym.trim() != "" && sym != "│" && sym != "─" {
                leading = Some(cell.fg);
            }
            text.push_str(sym);
        }
        rows.push((text.trim_end().to_string(), leading.unwrap_or(Color::Reset)));
    }
    rows
}

pub(super) fn row_color_for(rows: &[(String, Color)], needle: &str) -> Option<Color> {
    rows.iter()
        .find(|(t, _)| t.contains(needle))
        .map(|(_, c)| *c)
}

/// Build a Changed Files view with a long diff, render it (caching rects)
/// and move focus to the diff pane via Tab.
pub(super) fn diff_focused_view() -> ChangedFilesView {
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 80, 20);
    let _ = view.handle_event(&key(KeyCode::Tab));
    view
}

// ───────────────────────── RPC-368: click to select ─────────────────────
// Feature: spec/features/changed-files-view-click-to-select.feature

/// Dispatch a synthetic left mouse Down event at the given screen cell.
pub(super) fn left_click(view: &mut ChangedFilesView, column: u16, row: u16) -> ChangedFilesEvent {
    view.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }))
}

/// Render the view and return the raw `TestBackend` buffer so tests can
/// inspect individual cells (used for scrollbar glyph assertions).
pub(super) fn render_buffer(
    view: &mut ChangedFilesView,
    w: u16,
    h: u16,
) -> ratatui::buffer::Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

/// Count scrollbar glyphs (`■` thumb / `│` track painted by
/// `render_list_scrollbar`) in the single column `col` over the rows of
/// `rect`.
pub(super) fn scrollbar_glyphs_in_column(
    buf: &ratatui::buffer::Buffer,
    col: u16,
    rect: Rect,
) -> usize {
    let mut count = 0;
    for y in rect.y..rect.y.saturating_add(rect.height) {
        let sym = buf[(col, y)].symbol();
        if sym == "■" || sym == "│" {
            count += 1;
        }
    }
    count
}

// ───────────────────────── RPC-367: pane borders ────────────────────────
// Feature: spec/features/rust-tui-pane-borders-changed-files.feature

/// True if a vertical divider glyph `│` appears anywhere in column `col`
/// of the rendered buffer.
pub(super) fn has_vertical_divider_in_column(buf: &ratatui::buffer::Buffer, col: u16) -> bool {
    (0..buf.area.height).any(|y| buf[(col, y)].symbol() == "│")
}
