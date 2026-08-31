#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// Feature: spec/features/equal-division-pane-splits-no-minimums-with-live-resize.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.
//
// MUX-003 — equal-division pane splits (no minimums) with live resize.
// Layout-math scenarios call `calculate_pane_rects` directly; the
// render/resize scenario drives `Navigator::render_with_stores` through
// a `TestBackend` at two different terminal widths.

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::store::{AgentViewStore, BoardStore};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::multiplex::{calculate_pane_rects, MuxOrientation, MuxPaneKind};
use codelet_fspec_tui::views::{Navigator, ViewMode};

fn area(w: u16, h: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

fn widths(rects: &[Rect]) -> Vec<u16> {
    rects.iter().map(|r| r.width).collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board agent agent divides the width equally across
// three panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board agent agent divides the width equally across three panes
#[test]
fn mux_board_agent_agent_divides_the_width_equally_across_three_panes() {
    // @step Given mux mode is active on a 120-column terminal
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux board agent agent"
    // (the dispatcher folds a pane list without a trailing percent into
    // an equal-division split — the layout math consumes it here)
    let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[]);
    // @step Then the grid shows three panes: Board, Agent and Agent
    assert_eq!(rects.len(), 3, "three panes must be laid out");
    // @step And each pane gets an equal share of the width (39, 39 and 40 columns with the 1-col dividers; the last pane absorbs the integer-division remainder)
    assert_eq!(
        widths(&rects),
        vec![39, 39, 40],
        "118 available cols across 3 panes = 39/39/40 (last absorbs the remainder)"
    );
    // @step And no pane collapses to a 1-column sliver
    for (i, r) in rects.iter().enumerate() {
        assert!(
            r.width >= 39,
            "pane {i} must not collapse to a sliver, got {} cols",
            r.width
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux 4 divides the width equally across four panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux 4 divides the width equally across four panes
#[test]
fn mux_4_divides_the_width_equally_across_four_panes() {
    // @step Given mux mode is active on a 200-column terminal
    let panes = [
        MuxPaneKind::Board,
        MuxPaneKind::Agent,
        MuxPaneKind::ChangedFiles,
        MuxPaneKind::Checkpoints,
    ];
    // @step When I submit the slash command "/mux 4"
    // (the dispatcher expands the pane count with an equal-division
    // split — the layout math consumes it here)
    let rects = calculate_pane_rects(area(200, 24), MuxOrientation::Horizontal, &panes, &[]);
    // @step Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    assert_eq!(rects.len(), 4, "four panes must be laid out");
    // @step And each pane gets an equal share of the width (49, 49, 49 and 50 columns with the 3-col dividers; the last pane absorbs the integer-division remainder)
    assert_eq!(
        widths(&rects),
        vec![49, 49, 49, 50],
        "197 available cols across 4 panes = 49/49/49/50 (last absorbs the remainder)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: two panes divide the width equally
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: two panes divide the width equally
#[test]
fn two_panes_divide_the_width_equally() {
    // @step Given mux mode is active on a 120-column terminal
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux board agent"
    let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[]);
    // @step Then the grid shows two panes: Board and Agent
    assert_eq!(rects.len(), 2, "two panes must be laid out");
    // @step And each pane gets an equal share of the width (59 and 60 columns with the 1-col divider; the last pane absorbs the integer-division remainder)
    // (119 available across 2 panes = 59 + 60; the last pane absorbs
    // the integer-division remainder)
    assert_eq!(
        widths(&rects),
        vec![59, 60],
        "119 available cols across 2 panes = 59/60 (last absorbs the remainder)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a board pane narrower than 64 columns is not clamped up
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a board pane narrower than 64 columns is not clamped up
#[test]
fn a_board_pane_narrower_than_64_columns_is_not_clamped_up() {
    // @step Given mux mode is active on a 100-column terminal
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux board agent agent"
    let rects = calculate_pane_rects(area(100, 24), MuxOrientation::Horizontal, &panes, &[]);
    // @step Then the Board pane is 32 columns wide (an equal third, not clamped to the 64-column minimum)
    assert_eq!(
        rects[0].width, 32,
        "board pane must be an equal third (32), not clamped up to the old 64-col minimum"
    );
    // @step And the remaining panes take the other two equal thirds (32 and 34 columns; the last pane absorbs the integer-division remainder)
    assert_eq!(
        widths(&rects),
        vec![32, 32, 34],
        "98 available cols across 3 panes = 32/32/34 (last absorbs the remainder)"
    );
    // @step And the board view degrades gracefully when it cannot fit its columns
    // (the board render bails to a blank pane below its fit width —
    // covered by the board's own render tests; here we assert the layout
    // no longer protects the board at the expense of its siblings)
    assert!(
        rects[1].width >= 32 && rects[2].width >= 32,
        "sibling panes must keep their equal share"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the divider produces a non-equal split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the divider produces a non-equal split
#[test]
fn dragging_the_divider_produces_a_non_equal_split() {
    // @step Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    let equal = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[]);
    assert_eq!(
        widths(&equal),
        vec![59, 60],
        "precondition: equal split (last absorbs the remainder)"
    );
    // @step When I press the mouse down on the divider and drag it to the 40 percent position
    // (the drag stores a 40 percent split in the config — the layout
    // math consumes it here)
    let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[40]);
    // @step Then the Board pane is 40 percent of the width and the Agent pane takes the remainder
    assert_eq!(
        widths(&rects),
        vec![47, 72],
        "40% of 119 available = 47; the agent pane takes the 72-col remainder"
    );
    // @step And the drag state is cleared after the release
    // (covered by the MUX-001 drag-state tests in mux001.rs; here we
    // assert the stored percent survives a recompute as a plain split)
    let again = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[40]);
    assert_eq!(
        widths(&again),
        vec![47, 72],
        "the stored split is stable across recomputes"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a terminal resize re-divides the panes equally
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a terminal resize re-divides the panes equally
#[test]
fn a_terminal_resize_re_divides_the_panes_equally() {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.mux.enable_default();
    nav.mux.set_pane_list(
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
        None,
    );
    nav.active_view = ViewMode::Mux;
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    // MUX-002: agent slots only render when sessions are open — seed two
    // so both agent panes render (three equal thirds).
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-2"),
    ));

    let mut term_120 = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term_120
        .draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .expect("draw at 120 cols");
    let first = widths(nav.mux.pane_rects());
    assert_eq!(
        first,
        vec![39, 39, 40],
        "precondition: three equal thirds at 120 cols"
    );

    // @step When the terminal is resized to 180 columns
    // (the run loop re-renders on Event::Resize with the new frame area)
    let mut term_180 = Terminal::new(TestBackend::new(180, 24)).expect("Terminal::new");
    term_180
        .draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .expect("draw at 180 cols");

    // @step Then the next frame re-divides the three panes equally across the new width (59, 59 and 60 columns)
    assert_eq!(
        widths(nav.mux.pane_rects()),
        vec![59, 59, 60],
        "178 available cols across 3 panes = 59/59/60 after the resize"
    );
    // @step And no stale cached pane rect is used
    assert_eq!(
        nav.mux.pane_rects()[2].x + nav.mux.pane_rects()[2].width,
        180,
        "the last pane must extend to the new terminal edge"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board agent 40 honors the percent even below the old
// minimum
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board agent 40 honors the percent even below the old minimum
#[test]
fn mux_board_agent_40_honors_the_percent_even_below_the_old_minimum() {
    // @step Given mux mode is active on a 120-column terminal
    let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
    // @step When I submit the slash command "/mux board agent 40"
    // (the dispatcher folds the trailing percent into splits = [40] —
    // the layout math consumes it here)
    let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[40]);
    // @step Then the Board pane is 40 percent of the width (47 columns)
    assert_eq!(
        rects[0].width, 47,
        "40% of 119 available = 47 — honored as-is"
    );
    // @step And the Agent pane takes the remainder (72 columns)
    assert_eq!(
        rects[1].width, 72,
        "the agent pane takes the 72-col remainder"
    );
    // @step And the Board pane is not clamped up to the 64-column minimum
    assert!(
        rects[0].width < 64,
        "the board pane must stay at 47 cols, not clamp up to 64"
    );
}
