//! BUG-200 — Mux board-pane ring: the columns re-enter the mux bar
//! (Right) and the bar re-enters the columns (Left) — the continuous
//! column⇄item⇄chip ring in both directions.
//!
//! Feature: spec/features/mux-board-pane-ring-edges-re-enter-columns-and-chips.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions).
//!
//! Harness: the `menu004_mux_surface.rs` pattern — full-App +
//! MockBackend, real key events through `App::handle_event`,
//! observation on `app.navigator().mux.menu_focus()` (the mux bar's
//! ring focus) and `app.board_store_mut().focused_column_index()`
//! (the columns' ring half, col0 = backlog .. col6 = blocked). Bar
//! focus is reached through the production edge rule + ring walk
//! (Right off the last column enters the bar at Item(0), then Right
//! advances stop by stop) — no test-side state writes.
//!
//! Ring order (the BUG-200 directive):
//!   col0 … col6 → Item0 … ItemN → ZoneB0 … ZoneB_last → col0 …
//!
//! The pre-fix behavior these tests pin (the bug report):
//!   - Left from the first column wrapped to the LAST menu item (Help)
//!     instead of the last chip (the board store's own ZoneC-less edge);
//!   - Right from the last chip wrapped back to the first menu item
//!     (the mux snapshot ring has no column stops) — the 7 columns
//!     were unreachable from the bar side.

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxConfig, MuxLayout, MuxPaneKind, ViewMode,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0) — the `menu004_mux_surface` `enable_mux` mirror (incl. the
/// lazy-pane initial loads).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let Some(layout) = MuxLayout::columns(&panes) else {
        return;
    };
    let config = MuxConfig {
        layout,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    if panes.contains(&MuxPaneKind::ChangedFiles) {
        app.navigator_mut().changed_files = codelet_fspec_tui::views::ChangedFilesView::new();
        app.dispatch(codelet_fspec_tui::Action::ChangedFilesLoaded(Vec::new()));
    }
}

/// Seed `n` open sessions (s-1..s-n, Idle).
async fn seed_sessions(app: &mut App, n: usize) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await?;
    Ok(())
}

/// Arrange: the board pane's cursor on the given column index
/// (0 = backlog, 6 = blocked).
fn focus_column(app: &mut App, column: usize) {
    let column_names = [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ];
    app.board_store_mut()
        .set_focused_column(column_names[column]);
}

/// Arrange: walk the ring until the bar's focus reaches `steps` stops
/// past Item(0) (0 = Item(0) itself, 3 = Item(3) Help, 5 = ZoneB(2)
/// with the [Board, #1, #2] zone_b). Enters the bar through the
/// production edge rule (Right off the last column).
async fn walk_bar_to(app: &mut App, steps: usize) -> Result<(), codelet_test_helpers::TestError> {
    focus_column(app, 6);
    for _ in 0..=steps {
        app.handle_event(&key(KeyCode::Right));
        drain_pending(app).await?;
    }
    Ok(())
}

/// The [Board | Agent | Agent] grid with 2 open sessions, Board pane
/// focused — the bug report's layout.
async fn board_agent_agent(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    seed_sessions(app, 2).await?;
    enable_mux(
        app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    Ok(())
}

/// Scenario: Left from the first menu item lands on the last column
#[tokio::test]
async fn scenario_left_from_the_first_menu_item_lands_on_the_last_column() -> TestResult {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the first menu item (Kanban)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await?;
    walk_bar_to(&mut app, 0).await?;
    assert_eq!(app.navigator().mux.menu_focus(), Some(MenuFocus::Item(0)));

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight clears from the top bar and the board's last column (blocked) is focused
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        6,
        "Left from the first item must land on the last column (blocked)"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar ring must clear on the column landing"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the board's second-to-last column (done) is focused (the columns keep their own walk)
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        5,
        "Left from the last column walks the columns (done)"
    );
    Ok(())
}

/// Scenario: Right from the last button lands on the first column
#[tokio::test]
async fn scenario_right_from_the_last_button_lands_on_the_first_column() -> TestResult {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the last button ([ New Agent ], MUX-013)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await?;
    // Item(0) + 8 stops = ZoneC(1) (cells [Board, #1, #2], then
    // [ New Bot ] [ New Agent ] — no Close on the board pane).
    walk_bar_to(&mut app, 8).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "the arrange must land on the last button"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await?;

    // @step Then the highlight clears from the top bar and the board's first column (backlog) is focused
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        0,
        "Right from the last button must land on the first column (backlog)"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar ring must clear on the column landing"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await?;

    // @step Then the board's second column (specifying) is focused (the columns keep their own walk)
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        1,
        "Right from the first column walks the columns (specifying)"
    );
    Ok(())
}

/// Scenario: Left from the first column wraps to the last button
#[tokio::test]
async fn scenario_left_from_the_first_column_wraps_to_the_last_button() -> TestResult {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the first column (backlog)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await?;
    focus_column(&mut app, 0);
    assert_eq!(app.navigator().mux.menu_focus(), None);

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight lands on the last button ([ New Agent ]) in the top bar (the board ring's left edge re-enters the bar's last stop, not the menu items)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Left from the first column must wrap to the last button"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight lands on [ New Bot ]
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "the buttons keep their own walk"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight lands on the last chip (#2)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "Left from [ New Bot ] lands on the last chip (cells [Board, #1, #2])"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight keeps walking left through chip #1, the 'Board' pane view label and the menu items (Help, Settings, Tools, Kanban) with no column stop until the first item
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "Left from chip #2 lands on chip #1"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Left from chip #1 lands on the Board view label"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(3)),
        "Left from the Board view label lands on the last item (Help)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(2)),
        "Left from Help walks the items (Settings)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(1)),
        "Left from Settings walks the items (Tools)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left from Tools walks the items (Kanban — the ring's left edge)"
    );
    // The seam: Left from the first item wraps back onto the last column
    // (the ring is lossless — no stop swallowed, none repeated).
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        6,
        "Left from the first item wraps back to the last column (blocked)"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar ring must clear on the column landing"
    );
    Ok(())
}

/// Scenario: Right from the last column re-enters the bar on the first
/// menu item (the pre-existing edge rule — unchanged by BUG-200)
#[tokio::test]
async fn scenario_right_from_the_last_column_enters_the_bar_at_the_first_item() -> TestResult {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column (blocked)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await?;
    focus_column(&mut app, 6);
    assert_eq!(app.navigator().mux.menu_focus(), None);

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await?;

    // @step Then the highlight lands on the first menu item (Kanban) in the top bar (the pre-existing edge rule, unchanged by BUG-200)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the last column's Right edge must enter the bar at the first item (unchanged)"
    );
    Ok(())
}

/// Scenario: Left from the first column with no open sessions lands on the last button
#[tokio::test]
async fn scenario_left_from_the_first_column_with_no_open_sessions_lands_on_the_last_button(
) -> TestResult {
    // @step Given the mux grid is [Board | Files] with no open sessions and the Board pane is focused with the cursor on the first column (backlog)
    let (mut app, _mock) = fresh_app();
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    );
    focus_column(&mut app, 0);
    assert_eq!(app.navigator().mux.menu_focus(), None);

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await?;

    // @step Then the highlight lands on the [ New Agent ] button (the bar's last stop; Close is not shown while the board pane is focused)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "the last ring stop is the [ New Agent ] button"
    );
    Ok(())
}
