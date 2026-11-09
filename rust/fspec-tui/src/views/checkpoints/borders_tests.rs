//! Feature: spec/features/rust-tui-pane-borders-checkpoints.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

// ───────────────────────── board + navigator wiring ─────────────────────

// ───────────────────────── RPC-367: pane borders ────────────────────────
// Feature: spec/features/rust-tui-pane-borders-checkpoints.feature

/// Scenario: Checkpoints view shows a vertical divider between the
/// Checkpoints and Files panes
#[test]
fn checkpoints_view_shows_vertical_divider_between_checkpoints_and_files_panes() {
    // @step Given the Checkpoints view has at least one checkpoint to display
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    // Load files for the selected checkpoint so the top Files pane renders too.
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt", "M")]);

    // @step When the view is rendered to the terminal buffer
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let cp_rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step Then a vertical divider glyph is drawn in the column between the Checkpoints pane and the Files pane
    // The divider lives in the gutter column directly to the right of the
    // Checkpoints pane content (~40% of the top-row width).
    let divider_col = cp_rect.x + cp_rect.width;
    let divider_cell = (0..buf.area.height)
        .map(|y| &buf[(divider_col, y)])
        .find(|cell| cell.symbol() == "│");
    assert!(
        divider_cell.is_some(),
        "expected a '│' divider in column {divider_col} between the Checkpoints and Files panes"
    );

    // @step And the divider uses the default terminal colour with no explicit colour set
    let divider_fg = divider_cell
        .map(|cell| cell.fg)
        .expect("divider cell present");
    assert_eq!(
        divider_fg,
        Color::Reset,
        "divider should use the default terminal colour (Color::Reset)"
    );
}
