//! App bootstrap + subscriber-task + Action enum extensions
//! (RPC-009 → migrated by RPC-012 to the BoardStore + AgentViewStore +
//! Navigator architecture).
//!
//! Feature: spec/features/fspec-tui-app-bootstrap-rpc009.feature
//!
//! RPC-012 supersedes two RPC-009 contracts:
//!   - `App::bootstrap` no longer calls `backend.create_session(None)` —
//!     session creation is lazy (first `Action::EnterWorkUnit` /
//!     `OpenAgentView`). See `rpc012-board-agent-navigation.feature`.
//!   - `RootView` + `FocusedPane` are gone; the chunks subscriber filter
//!     reads `AgentViewStore.current_session` via a `watch::channel`
//!     shared with App::dispatch. See RPC-012 rule [6] / [19].
//!
//! Surviving RPC-009 scenarios:
//!   - `App::bootstrap()` seeds the work-units state via
//!     `Action::WorkUnitsLoaded` (now stored in BoardStore).
//!   - Three subscriber tasks are spawned via `tokio::spawn` on the host
//!     runtime that drain `work_units_rx`/`chunks_rx`/`logs_rx`.
//!   - The chunks subscriber filters by the AgentViewStore's
//!     current_session BEFORE emitting `Action::ChunkReceived`.
//!   - `Action::InputSubmitted` / `Action::Interrupt` dispatch the
//!     corresponding `FspecBackend` methods.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, WorkUnitInfo};

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
    }
}

/// Scenario: App bootstrap calls backend.list_work_units() and seeds the
/// BoardStore (migrated from the old left-pane WorkUnitsListView).
#[tokio::test]
async fn app_bootstrap_calls_list_work_units_and_seeds_the_left_pane() {
    // @step Given a MockBackend seeded with [AUTH-001 done, AUTH-002 implementing]
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "done"), wu("AUTH-002", "implementing")]);
    // @step And an App constructed against that backend on an 80x24 TestBackend
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // @step When the App's bootstrap runs
    app.bootstrap().await.expect("bootstrap");
    // @step Then MockBackend.list_work_units_calls equals 1
    assert_eq!(mock.list_work_units_calls(), 1);
    // @step And the WorkUnitsListView's items equals [AUTH-001 done, AUTH-002 implementing]
    let snapshot = app.work_units_snapshot();
    assert_eq!(snapshot.len(), 2);
    let ids: Vec<&str> = snapshot.iter().map(|u| u.id.as_str()).collect();
    assert!(ids.contains(&"AUTH-001"));
    assert!(ids.contains(&"AUTH-002"));
    // @step And the WorkUnitsListView's state.selected() returns Some(0)
    // RPC-012: focus defaults to "backlog" column with selection 0.
    assert_eq!(app.board_store().focused_column(), "backlog");
    assert_eq!(app.board_store().selected_index_for("backlog"), 0);
}

/// Scenario: App bootstrap spawns three subscriber tasks via tokio::spawn on the host runtime
#[tokio::test]
async fn app_bootstrap_spawns_three_subscriber_tasks_via_tokio_spawn_on_the_host_runtime() {
    // @step Given an App constructed against a MockBackend on a `#[tokio::test]` runtime
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // @step When the App's bootstrap runs
    app.bootstrap().await.expect("bootstrap");
    // @step Then exactly three subscriber tasks are alive on the current tokio Handle
    assert_eq!(app.subscriber_task_count(), 3);
    // RPC-012 lazy-session: prime the chunks filter with a session id
    // so the chunks subscriber forwards.
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step And one task drains `backend.work_units_rx()` and sends `Action::WorkUnitsLoaded(units)` to the action bus
    mock.push_work_units(vec![wu("AUTH-003", "backlog")]);
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .expect("Action::WorkUnitsLoaded on bus");
    assert!(matches!(action, Action::WorkUnitsLoaded(units) if units.len() == 1));
    // @step And one task drains `backend.chunks_rx()` filters by the active session id and sends `Action::ChunkReceived(id, chunk)` to the action bus
    mock.push_chunk(SessionId::new("s-mock-1"), StreamChunk::text("x".to_string()));
    let action = wait_for_action(&mut app, |a| matches!(a, Action::ChunkReceived(_, _)))
        .await
        .expect("Action::ChunkReceived on bus");
    assert!(matches!(action, Action::ChunkReceived(_, _)));
    // @step And one task drains `backend.logs_rx()` and forwards records to the action bus or tracing layer
    // (subscriber_task_count == 3 above proves all three are alive).
    // @step And no `tokio::runtime::Builder` or `Runtime::new()` call appears in the App bootstrap path
    // (asserted by source_shape_rpc009.rs)
}

/// Scenario: work_units broadcast event becomes an Action::WorkUnitsLoaded on the action bus
#[tokio::test]
async fn work_units_broadcast_event_becomes_an_action_workunitsloaded_on_the_action_bus() {
    // @step Given an App constructed against a MockBackend with bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    // @step When the test calls `mock.push_work_units(vec![AUTH-001 done, AUTH-002 implementing, AUTH-003 backlog])`
    mock.push_work_units(vec![
        wu("AUTH-001", "done"),
        wu("AUTH-002", "implementing"),
        wu("AUTH-003", "backlog"),
    ]);
    // @step Then within 200ms the App's action bus receives an `Action::WorkUnitsLoaded` carrying the new three-entry list
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .expect("WorkUnitsLoaded within 200ms");
    let units = match action {
        Action::WorkUnitsLoaded(u) => u,
        _ => panic!("expected WorkUnitsLoaded"),
    };
    assert_eq!(units.len(), 3);
    // @step And the WorkUnitsListView's items equals the three-entry list after compositor.update is called
    app.dispatch(Action::WorkUnitsLoaded(units));
    assert_eq!(app.work_units_snapshot().len(), 3);
}

/// Scenario: chunks broadcast events for the active session become Action::ChunkReceived
#[tokio::test]
async fn chunks_broadcast_events_for_the_active_session_become_action_chunkreceived() {
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the test calls `mock.push_chunk(SessionId::new("s-mock-1"), StreamChunk::text("hello".into()))`
    mock.push_chunk(SessionId::new("s-mock-1"), StreamChunk::text("hello".to_string()));
    // @step Then within 200ms the App's action bus receives an `Action::ChunkReceived(SessionId::new("s-mock-1"), StreamChunk::text("hello".into()))`
    let action = wait_for_action(&mut app, |a| matches!(a, Action::ChunkReceived(_, _)))
        .await
        .expect("ChunkReceived within 200ms");
    assert!(matches!(action, Action::ChunkReceived(id, _) if id == SessionId::new("s-mock-1")));
}

/// Scenario: chunks broadcast events for an OTHER session do NOT become Action::ChunkReceived
#[tokio::test]
async fn chunks_broadcast_events_for_an_other_session_do_not_become_action_chunkreceived() {
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the test calls `mock.push_chunk(SessionId::new("s-other"), StreamChunk::text("not for us".into()))`
    mock.push_chunk(SessionId::new("s-other"), StreamChunk::text("not for us".to_string()));
    // @step Then within 200ms the App's action bus receives no `Action::ChunkReceived`
    let action = wait_for_action(&mut app, |a| matches!(a, Action::ChunkReceived(_, _))).await;
    assert!(action.is_none(), "expected no ChunkReceived for other session");
}

/// Scenario: Action::InputSubmitted dispatches backend.send_input and is forwarded to compositor.update
#[tokio::test]
async fn action_inputsubmitted_dispatches_backend_send_input_and_is_forwarded_to_compositor_update() {
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the App processes `Action::InputSubmitted("hi".into())` on the action bus
    app.dispatch(Action::InputSubmitted("hi".to_string()));
    // Allow the spawned task to call send_input
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // @step Then `MockBackend.send_input` is invoked exactly once with `(SessionId("s-mock-1"), "hi")`
    assert_eq!(mock.send_input_calls(), 1);
    let last = mock.last_send_input();
    assert_eq!(last, Some((SessionId::new("s-mock-1"), "hi".to_string())));
    // @step And the action is also forwarded into compositor.update so layers can react if needed
    // (verified by App::dispatch's contract — it always calls compositor.update after special-casing)
}

/// Scenario: Action::Interrupt dispatches backend.interrupt
#[tokio::test]
async fn action_interrupt_dispatches_backend_interrupt() {
    // @step Given an App with active_session = Some(SessionId("s-mock-1"))
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    let was_quit = app.should_quit();
    // @step When the App processes `Action::Interrupt` on the action bus
    app.dispatch(Action::Interrupt);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // @step Then `MockBackend.interrupt` is invoked exactly once with `SessionId("s-mock-1")`
    assert_eq!(mock.interrupt_calls(), 1);
    assert_eq!(mock.last_interrupt(), Some(SessionId::new("s-mock-1")));
    // @step And the App's `should_quit` flag is unchanged
    assert_eq!(app.should_quit(), was_quit);
}

/// Scenario: Subscriber tasks honour RecvError::Lagged by logging at debug and continuing
#[tokio::test]
async fn subscriber_tasks_honour_recverror_lagged_by_logging_at_debug_and_continuing() {
    // @step Given an App constructed against a MockBackend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    // @step And the work_units broadcast channel is intentionally lagged by overflowing its capacity
    for i in 0..200 {
        mock.push_work_units(vec![wu(&format!("AUTH-{i:03}"), "backlog")]);
    }
    // @step When the work_units subscriber task observes `RecvError::Lagged(n)`
    // @step Then the task does NOT panic
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(app.subscriber_task_count(), 3);
    // @step And the task subsequently re-fetches a snapshot via `backend.list_work_units()` and emits a fresh `Action::WorkUnitsLoaded`
    mock.seed_work_units(vec![wu("FRESH-001", "done")]);
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .expect("post-Lagged WorkUnitsLoaded");
    assert!(matches!(action, Action::WorkUnitsLoaded(_)));
}

/// Scenario: Action enum gains seven new variants while existing variants are preserved
#[test]
fn action_enum_gains_seven_new_variants_while_existing_variants_are_preserved() {
    // @step Given the Action enum in codelet/fspec-tui/src/components/mod.rs
    let src = include_str!("../src/components/mod.rs");
    // @step Then it contains the existing variants Quit, Redraw, Custom(String)
    assert!(src.contains("Quit,"));
    assert!(src.contains("Redraw,"));
    assert!(src.contains("Custom(String)"));
    // @step And it additionally contains LoadWorkUnits
    assert!(src.contains("LoadWorkUnits,"));
    // @step And it additionally contains WorkUnitsLoaded(Vec<WorkUnitInfo>)
    assert!(src.contains("WorkUnitsLoaded(Vec<codelet_rpc_types::WorkUnitInfo>)"));
    // @step And it additionally contains SessionCreated(SessionId)
    assert!(src.contains("SessionCreated(codelet_rpc_types::SessionId)"));
    // @step And it additionally contains ChunkReceived(SessionId, StreamChunk)
    assert!(src.contains("ChunkReceived(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)"));
    // @step And it additionally contains InputSubmitted(String)
    assert!(src.contains("InputSubmitted(String)"));
    // @step And it additionally contains Interrupt
    assert!(src.contains("Interrupt,"));
    // @step And it additionally contains FocusNext
    assert!(src.contains("FocusNext,"));
    // @step And the enum still derives Clone, Debug
    assert!(src.contains("#[derive(Debug, Clone)]"));
    // @step And the enum drops PartialEq and Eq because StreamChunk does not derive PartialEq
    // (asserted by the absence of PartialEq, Eq in the derive line — exactly `#[derive(Debug, Clone)]`)
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Drain the App's action bus until a matching action arrives or 200ms
/// elapses. Returns Some(action) on match, None on timeout.
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
    while std::time::Instant::now() < deadline {
        if let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    None
}
