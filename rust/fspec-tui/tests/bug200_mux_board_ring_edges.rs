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

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind,
    ViewMode,
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

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0) — the `menu004_mux_surface` `enable_mux` mirror (incl. the
/// lazy-pane initial loads).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes: panes.clone(),
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
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
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
async fn walk_bar_to(app: &mut App, steps: usize) {
    focus_column(app, 6);
    for _ in 0..=steps {
        app.handle_event(&key(KeyCode::Right));
        drain_pending(app).await;
    }
}

/// The [Board | Agent | Agent] grid with 2 open sessions, Board pane
/// focused — the bug report's layout.
async fn board_agent_agent(app: &mut App) {
    seed_sessions(app, 2).await;
    enable_mux(
        app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
}

/// Scenario: Left from the first menu item lands on the last column
#[tokio::test]
async fn scenario_left_from_the_first_menu_item_lands_on_the_last_column() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the first menu item (Kanban)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await;
    walk_bar_to(&mut app, 0).await;
    assert_eq!(app.navigator().mux.menu_focus(), Some(MenuFocus::Item(0)));

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;

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
    drain_pending(&mut app).await;

    // @step Then the board's second-to-last column (done) is focused (the columns keep their own walk)
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        5,
        "Left from the last column walks the columns (done)"
    );
}

/// Scenario: Right from the last chip lands on the first column
#[tokio::test]
async fn scenario_right_from_the_last_chip_lands_on_the_first_column() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the last chip (#2)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await;
    // Item(0) + 6 stops = ZoneB(2) (the last chip of [Board, #1, #2]).
    walk_bar_to(&mut app, 6).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "the arrange must land on the last chip"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the highlight clears from the top bar and the board's first column (backlog) is focused
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        0,
        "Right from the last chip must land on the first column (backlog)"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar ring must clear on the column landing"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the board's second column (specifying) is focused (the columns keep their own walk)
    assert_eq!(
        app.board_store_mut().focused_column_index(),
        1,
        "Right from the first column walks the columns (specifying)"
    );
}

/// Scenario: Left from the first column wraps to the last chip
#[tokio::test]
async fn scenario_left_from_the_first_column_wraps_to_the_last_chip() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the first column (backlog)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await;
    focus_column(&mut app, 0);
    assert_eq!(app.navigator().mux.menu_focus(), None);

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the last chip (#2) in the top bar (the board ring's left edge re-enters the bar's chip segment, not the menu items)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "Left from the first column must wrap to the last Zone B cell (chip #2 — \
         cells [Board, #1, #2])"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on chip #1
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the bar's chips keep their own walk"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the 'Board' pane view label (the first Zone B cell)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Left from chip #1 lands on the Board view label"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the last menu item (Help) and keeps walking the items from there (Settings then Tools on the next Lefts — the bar's items⇄chips ring has no column stop until the first item)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(3)),
        "Left from the Board view label lands on the last item (Help)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(2)),
        "Left from Help walks the items (Settings)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(1)),
        "Left from Settings walks the items (Tools)"
    );
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left from Tools walks the items (Kanban — the ring's left edge)"
    );
    // The seam: Left from the first item wraps back onto the last column
    // (the ring is lossless — no stop swallowed, none repeated).
    app.handle_event(&key(KeyCode::Left));
    drain_pending(&mut app).await;
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
}

/// Scenario: Right from the last column re-enters the bar on the first
/// menu item (the pre-existing edge rule — unchanged by BUG-200)
#[tokio::test]
async fn scenario_right_from_the_last_column_enters_the_bar_at_the_first_item() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column (blocked)
    let (mut app, _mock) = fresh_app();
    board_agent_agent(&mut app).await;
    focus_column(&mut app, 6);
    assert_eq!(app.navigator().mux.menu_focus(), None);

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the first menu item (Kanban) in the top bar (the pre-existing edge rule, unchanged by BUG-200)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the last column's Right edge must enter the bar at the first item (unchanged)"
    );
}

/// Scenario: Left from the first column with no open sessions lands on
/// the last view-label cell
#[tokio::test]
async fn scenario_left_from_the_first_column_with_no_open_sessions_lands_on_the_last_view_label_cell(
) {
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
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the 'Files' pane label (the last Zone B cell — the chips' absence makes the view labels the ring's last stop)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "with no chips the last Zone B cell is the Files view label"
    );
}
