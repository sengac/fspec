//! RPC-017 — App-level dispatch wiring for `Action::ReorderUp / ReorderDown`.
//!
//! Feature: spec/features/rpc017-app-dispatch-reorder.feature
//!
//! Drives `App::dispatch(Action::ReorderUp / ReorderDown)` against a
//! `MockBackend` whose BoardStore has a known focused-column selection,
//! then awaits the fire-and-forget tokio task that App::dispatch spawns
//! and asserts that `backend.move_work_unit_up/_down` was invoked with
//! the SELECTED work-unit id (not the position).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::WorkUnitInfo;

mod common;

use common::MockBackend;

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

/// Wait until either the mock backend records `expected` calls or the
/// timeout elapses. The fire-and-forget tokio task App::dispatch spawns
/// races the test thread, so we poll instead of asserting immediately.
async fn await_call_count(actual: impl Fn() -> usize, expected: usize, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if actual() >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        actual(),
        expected,
        "timed out waiting for {expected} calls"
    );
}

/// Scenario: Action::ReorderUp dispatches backend.move_work_unit_up against the selected work unit
#[tokio::test]
async fn action_reorder_up_dispatches_move_work_unit_up_for_selected_id() {
    // @step Given an App constructed against a mock backend whose BoardStore has "B-002" selected in the backlog column
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let units = vec![wu("A-001", "backlog"), wu("B-002", "backlog"), wu("C-003", "backlog")];
    app.board_store_mut().replace_work_units(units);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 1);

    // @step When the App dispatches Action::ReorderUp
    app.dispatch(Action::ReorderUp);

    // Wait for the fire-and-forget tokio task.
    await_call_count(
        || mock.move_work_unit_up_calls(),
        1,
        Duration::from_secs(1),
    )
    .await;

    // @step Then the mock backend records exactly one move_work_unit_up call with id "B-002"
    assert_eq!(mock.move_work_unit_up_calls(), 1);
    assert_eq!(mock.last_move_work_unit_up_id(), Some("B-002".to_string()));

    // @step And the mock backend records zero move_work_unit_down calls
    assert_eq!(mock.move_work_unit_down_calls(), 0);
}

/// Scenario: Action::ReorderDown dispatches backend.move_work_unit_down against the selected work unit
#[tokio::test]
async fn action_reorder_down_dispatches_move_work_unit_down_for_selected_id() {
    // @step Given an App constructed against a mock backend whose BoardStore has "A-001" selected in the backlog column
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let units = vec![wu("A-001", "backlog"), wu("B-002", "backlog"), wu("C-003", "backlog")];
    app.board_store_mut().replace_work_units(units);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When the App dispatches Action::ReorderDown
    app.dispatch(Action::ReorderDown);

    await_call_count(
        || mock.move_work_unit_down_calls(),
        1,
        Duration::from_secs(1),
    )
    .await;

    // @step Then the mock backend records exactly one move_work_unit_down call with id "A-001"
    assert_eq!(mock.move_work_unit_down_calls(), 1);
    assert_eq!(
        mock.last_move_work_unit_down_id(),
        Some("A-001".to_string())
    );

    // @step And the mock backend records zero move_work_unit_up calls
    assert_eq!(mock.move_work_unit_up_calls(), 0);
}

/// Scenario: Action::ReorderUp is a no-op when the focused column is empty
#[tokio::test]
async fn action_reorder_up_is_a_noop_when_focused_column_is_empty() {
    // @step Given an App constructed against a mock backend whose BoardStore's focused column is empty
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // Empty board.
    app.board_store_mut().replace_work_units(Vec::new());
    app.board_store_mut().set_focused_column("backlog");

    // @step When the App dispatches Action::ReorderUp
    app.dispatch(Action::ReorderUp);

    // Brief settle window so any (incorrect) spawn would have a chance
    // to record a call before we assert zero.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // @step Then the mock backend records zero move_work_unit_up calls
    assert_eq!(mock.move_work_unit_up_calls(), 0);

    // @step And the App does not panic
    // (reaching this line is the assertion)
}

/// Scenario: BoardStore::replace_work_units re-anchors per-column selection to the previously-selected work unit id
#[test]
fn replace_work_units_reanchors_selection_to_previously_selected_id() {
    use codelet_fspec_tui::BoardStore;

    // @step Given a BoardStore seeded with [A-001 backlog, B-002 backlog, C-003 backlog] and the backlog column has selected_index 2 (C-003)
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        wu("A-001", "backlog"),
        wu("B-002", "backlog"),
        wu("C-003", "backlog"),
    ]);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 2);
    assert_eq!(store.selected_index_for("backlog"), 2);
    assert_eq!(
        store.selected_work_unit().map(|u| u.id.as_str()),
        Some("C-003")
    );

    // @step When store.replace_work_units is called with [A-001 backlog, C-003 backlog, B-002 backlog] (C-003 moved up by one)
    store.replace_work_units(vec![
        wu("A-001", "backlog"),
        wu("C-003", "backlog"),
        wu("B-002", "backlog"),
    ]);

    // @step Then store.selected_index_for("backlog") returns 1 (C-003's new position)
    assert_eq!(store.selected_index_for("backlog"), 1);

    // @step Then store.selected_work_unit().map(|u| u.id.as_str()) returns Some("C-003")
    assert_eq!(
        store.selected_work_unit().map(|u| u.id.as_str()),
        Some("C-003")
    );
}
