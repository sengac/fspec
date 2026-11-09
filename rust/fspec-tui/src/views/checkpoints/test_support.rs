//! Shared fixtures for the checkpoints test modules (COV-075 split).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use codelet_rpc_types::{ChangedFile, CheckpointInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

pub(super) fn ci(work_unit_id: &str, name: &str, is_automatic: bool) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        is_automatic,
    }
}

pub(super) fn cf(path: &str, ct: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: ct.to_string(),
        staged: false,
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

/// Render the view and return `(joined symbols, per-cell (symbol, fg))`.
pub(super) fn render_grid(
    view: &mut CheckpointsView,
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

/// Per-row `(text, leading-cell fg)` for diff-color assertions.
pub(super) fn diff_rows(view: &mut CheckpointsView, w: u16, h: u16) -> Vec<(String, Color)> {
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
            if leading.is_none() && sym.trim() != "" {
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

// ───────────────────────── RPC-369: click to select ─────────────────────
// Feature: spec/features/checkpoints-view-click-to-select.feature

/// Dispatch a synthetic left mouse Down event at the given screen cell.
pub(super) fn left_click(view: &mut CheckpointsView, column: u16, row: u16) -> CheckpointsEvent {
    view.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }))
}

/// A view with two checkpoints (first selected) and files loaded for it,
/// rendered once so the pane content rects are cached.
pub(super) fn view_with_two_checkpoints() -> CheckpointsView {
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        ci("AUTH-001", "baseline", false),
        ci("AUTH-002", "second", false),
    ]);
    view.set_files(
        "AUTH-001",
        "baseline",
        vec![cf("a.txt", "M"), cf("b.txt", "A")],
    );
    let _ = render_grid(&mut view, 100, 24);
    view
}
