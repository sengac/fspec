// Feature: spec/features/equal-division-pane-splits-no-minimums-with-live-resize.feature
//
// This test file validates the acceptance criteria defined in the feature
// file. Scenarios map directly to Gherkin scenarios.
//
// MUX-003 — equal-division pane splits (no minimums) with live resize.
// Layout-math scenarios call `layout_tree::compute` directly; the
// render/resize scenario drives `Navigator::render_with_stores` through
// a `TestBackend` at two different terminal widths.

use codelet_test_helpers::{OrFail, TestResult};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::store::{AgentViewStore, BoardStore};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::multiplex::layout_tree::{compute, drag::reweight};
use codelet_fspec_tui::views::multiplex::{MuxLayout, MuxPaneKind};
use codelet_fspec_tui::views::{Navigator, ViewMode};

fn area(w: u16, h: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

/// The pane rects of the layout string `layout` in `area`.
fn rects_of(layout: &str, area: Rect) -> Result<Vec<Rect>, codelet_test_helpers::TestError> {
    let layout = MuxLayout::parse(layout).or_fail("layout")?;
    Ok(compute(area, &layout, &[]).panes)
}

fn widths(rects: &[Rect]) -> Vec<u16> {
    rects.iter().map(|r| r.width).collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board, agent, agent divides the width equally across
// three panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board, agent, agent divides the width equally across three panes
#[test]
fn mux_board_agent_agent_divides_the_width_equally_across_three_panes() -> TestResult {
    // @step Given mux mode is active on a 120-column terminal
    let area = area(120, 24);
    // @step When I submit the slash command "/mux board, agent, agent"
    // (a layout without weights is an equal division — the layout math
    // consumes it here)
    let layout = MuxLayout::parse("board, agent, agent").or_fail("layout")?;
    let rects = compute(area, &layout, &[]).panes;
    // @step Then the grid shows three panes: Board, Agent and Agent
    assert_eq!(rects.len(), 3, "three panes must be laid out");
    assert_eq!(
        layout.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent]
    );
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board, agent, files, checkpoints divides the width
// equally across four panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board, agent, files, checkpoints divides the width equally across four panes
#[test]
fn mux_four_pane_layout_divides_the_width_equally_across_four_panes() -> TestResult {
    // @step Given mux mode is active on a 200-column terminal
    let area = area(200, 24);
    // @step When I submit the slash command "/mux board, agent, files, checkpoints"
    // (a layout without weights is an equal division — the layout math
    // consumes it here)
    let layout = MuxLayout::parse("board, agent, files, checkpoints").or_fail("layout")?;
    let rects = compute(area, &layout, &[]).panes;
    // @step Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    assert_eq!(rects.len(), 4, "four panes must be laid out");
    assert_eq!(
        layout.leaves(),
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ]
    );
    // @step And each pane gets an equal share of the width (49, 49, 49 and 50 columns with the 3-col dividers; the last pane absorbs the integer-division remainder)
    assert_eq!(
        widths(&rects),
        vec![49, 49, 49, 50],
        "197 available cols across 4 panes = 49/49/49/50 (last absorbs the remainder)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: two panes divide the width equally
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: two panes divide the width equally
#[test]
fn two_panes_divide_the_width_equally() -> TestResult {
    // @step Given mux mode is active on a 120-column terminal
    let area = area(120, 24);
    // @step When I submit the slash command "/mux board, agent"
    let rects = rects_of("board, agent", area)?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a board pane narrower than 64 columns is not clamped up
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a board pane narrower than 64 columns is not clamped up
#[test]
fn a_board_pane_narrower_than_64_columns_is_not_clamped_up() -> TestResult {
    // @step Given mux mode is active on a 100-column terminal
    let area = area(100, 24);
    // @step When I submit the slash command "/mux board, agent, agent"
    let rects = rects_of("board, agent, agent", area)?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: dragging the divider produces a non-equal split
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: dragging the divider produces a non-equal split
#[test]
fn dragging_the_divider_produces_a_non_equal_split() -> TestResult {
    // @step Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    let area = area(120, 24);
    let mut layout = MuxLayout::parse("board, agent").or_fail("layout")?;
    let equal = compute(area, &layout, &[]);
    assert_eq!(
        widths(&equal.panes),
        vec![59, 60],
        "precondition: equal split (last absorbs the remainder)"
    );
    // @step When I press the mouse down on the divider and drag it to the 40 percent position
    // (48 of the pair's 119 cells = 40 percent — the release re-weights
    // the pair; the layout math consumes the stored weights here)
    let divider = equal.dividers.first().or_fail("one divider")?;
    assert!(
        reweight(&mut layout, divider, 48),
        "the drag re-weights the pair"
    );
    assert_eq!(layout.to_string(), "board:2, agent:3");
    let rects = compute(area, &layout, &[]).panes;
    // @step Then the Board pane is 40 percent of the width and the Agent pane takes the remainder
    assert_eq!(
        widths(&rects),
        vec![47, 72],
        "40% of 119 available = 47; the agent pane takes the 72-col remainder"
    );
    // @step And the drag state is cleared after the release
    // (covered by the MUX-001 drag-state tests in mux001.rs; here we
    // assert the stored weights survive a recompute)
    let again = compute(area, &layout, &[]).panes;
    assert_eq!(
        widths(&again),
        vec![47, 72],
        "the stored split is stable across recomputes"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a terminal resize re-divides the panes equally
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a terminal resize re-divides the panes equally
#[test]
fn a_terminal_resize_re_divides_the_panes_equally() -> TestResult {
    // @step Given mux mode is active with three panes on a 120-column terminal
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.mux.enable_default();
    if let Some(mux_layout) =
        MuxLayout::columns(&[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent])
    {
        nav.mux.set_layout(mux_layout);
    }
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

    let mut term_120 = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term_120
        .draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .or_fail("draw at 120 cols")?;
    let first = widths(nav.mux.pane_rects());
    assert_eq!(
        first,
        vec![39, 39, 40],
        "precondition: three equal thirds at 120 cols"
    );

    // @step When the terminal is resized to 180 columns
    // (the run loop re-renders on Event::Resize with the new frame area)
    let mut term_180 = Terminal::new(TestBackend::new(180, 24)).or_fail("Terminal::new")?;
    term_180
        .draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), &board, &mut agent);
        })
        .or_fail("draw at 180 cols")?;

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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux board:2, agent:3 honors the weights even below the old
// minimum
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux board:2, agent:3 honors the weights even below the old minimum
#[test]
fn mux_board_2_agent_3_honors_the_weights_even_below_the_old_minimum() -> TestResult {
    // @step Given mux mode is active on a 120-column terminal
    let area = area(120, 24);
    // @step When I submit the slash command "/mux board:2, agent:3"
    let rects = rects_of("board:2, agent:3", area)?;
    // @step Then the Board pane gets two fifths of the width (47 columns)
    assert_eq!(
        rects[0].width, 47,
        "two fifths of 119 available = 47 — honored as-is"
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
    Ok(())
}
