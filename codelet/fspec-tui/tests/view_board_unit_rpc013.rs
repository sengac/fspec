//! RPC-013 — BoardView footer rendering tests.
//!
//! Feature: spec/features/rpc013-board-footer.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme};
use codelet_rpc_types::WorkUnitInfo;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
    last_state_change_at: None,
    }
}

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn render_board(width: u16, height: u16, units: Vec<WorkUnitInfo>) -> String {
    let (view, _rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(units);
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

fn render_board_rows(width: u16, height: u16, units: Vec<WorkUnitInfo>) -> Vec<String> {
    let (view, _rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(units);
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Scenario: BoardView renders the literal TS UnifiedBoardLayout footer string
#[test]
fn board_view_renders_literal_unified_board_layout_footer_string() {
    // @step Given an App with bootstrap complete and Navigator.active_view = ViewMode::Board
    // @step And BoardStore seeded with [AUTH-001 backlog]
    let units = vec![wu("AUTH-001", "backlog")];
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_board(120, 24, units);
    // @step Then the rendered buffer contains the substring "← → Columns"
    assert!(joined.contains("← → Columns"), "missing '← → Columns' in:\n{joined}");
    // @step And the rendered buffer contains the substring "↑↓ Work Units"
    assert!(joined.contains("↑↓ Work Units"), "missing '↑↓ Work Units' in:\n{joined}");
    // @step And the rendered buffer contains the substring "[ Priority Up"
    assert!(joined.contains("[ Priority Up"), "missing '[ Priority Up' in:\n{joined}");
    // @step And the rendered buffer contains the substring "] Priority Down"
    assert!(joined.contains("] Priority Down"), "missing '] Priority Down' in:\n{joined}");
    // @step And the rendered buffer contains the substring "↵ Work Agent"
    assert!(joined.contains("↵ Work Agent"), "missing '↵ Work Agent' in:\n{joined}");
    // @step And the rendered buffer contains the substring "ESC Back"
    assert!(joined.contains("ESC Back"), "missing 'ESC Back' in:\n{joined}");
}

/// Scenario: BoardView footer omits the legacy `? help q quit Tab switch pane` hint
#[test]
fn board_view_footer_omits_legacy_generic_hint() {
    // @step Given an App with bootstrap complete and Navigator.active_view = ViewMode::Board
    let units = vec![wu("AUTH-001", "backlog")];
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_board(120, 24, units);
    // @step Then the rendered buffer does NOT contain the substring "? help"
    assert!(!joined.contains("? help"), "legacy '? help' still present in:\n{joined}");
    // @step And the rendered buffer does NOT contain the substring "switch pane"
    assert!(!joined.contains("switch pane"), "legacy 'switch pane' still present in:\n{joined}");
    // @step And the rendered buffer does NOT contain the substring "Tab "
    assert!(!joined.contains("Tab "), "legacy 'Tab ' still present in:\n{joined}");
}

/// Scenario: BoardView paints headers above the footer in its own area
#[test]
fn board_view_paints_headers_above_footer_row_in_its_own_area() {
    // @step Given a BoardView rendered against a 120x24 TestBackend with [AUTH-001 backlog]
    let units = vec![wu("AUTH-001", "backlog")];
    // @step When a developer scans the rendered buffer row by row
    let rows = render_board_rows(120, 24, units);
    assert_eq!(rows.len(), 24);
    // @step Then row 22 (the last in-bounds row of the box) contains the footer string substring "← → Columns"
    // The BoardView's outer Block draws on rows 0 and 23 (top/bottom border).
    // The footer 1-row chunk sits just above the bottom border at row 22.
    assert!(
        rows[22].contains("← → Columns"),
        "expected footer on row 22, got:\nrow 21: {}\nrow 22: {}\nrow 23: {}",
        rows[21], rows[22], rows[23]
    );
    // @step And at least one row above row 22 contains "BACKLOG"
    let backlog_row = rows[..22].iter().position(|r| r.contains("BACKLOG"));
    assert!(backlog_row.is_some(), "expected BACKLOG on some row above 22");
    // @step And the work-unit id "AUTH-001" appears on a row strictly above the footer row
    let auth_row = rows[..22].iter().position(|r| r.contains("AUTH-001"));
    assert!(auth_row.is_some(), "expected AUTH-001 above row 22");
}
