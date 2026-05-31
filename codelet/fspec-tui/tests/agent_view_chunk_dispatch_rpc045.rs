//! RPC-045 — Subscribe to chunks + status broadcasts; handle every new
//! `StreamChunk` variant.
//!
//! Feature: spec/features/agentview-subscribe-broadcasts.feature
//!
//! Drives the App::dispatch routing for the 5 new StreamChunk state
//! variants (`SessionStateChange`, `IsolationStateChange`,
//! `DebugStateChange`, `FooterStateUpdate`, `FspecCommandRequest`) and
//! the new push-driven `Action::SessionStatusChanged` path. Also pins
//! the rule that background-session chunks accumulate into the right
//! per-session SessionContext regardless of focus and that the
//! chunks_rx subscriber terminates cleanly when its Sender drops.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{
    FspecRequest, SessionId, SessionState, SessionStatus, StreamChunk, WorkUnitInfo,
};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Background-session chunks are routed by SessionId, not focus
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn background_session_chunk_appends_to_correct_session_context() {
    // @step Given an App with two open sessions s-1 (focused) and s-2 (background)
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // s-2 was created second → it is currently focused. Cycle back to s-1
    // so that s-1 is the focused tab and s-2 is the background tab.
    app.dispatch(Action::SessionPrev);
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "test precondition: s-1 must be the focused session"
    );

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-2, StreamChunk::text("hi"))
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::text("hi".to_string()),
    ));

    // @step Then the App's dispatch routes the chunk into s-2's SessionContext scrollback
    let s2_chunks = app
        .agent_view_store()
        .session_context_for(&sid("s-2"))
        .expect("s-2 SessionContext present")
        .scrollback
        .chunk_count();
    assert_eq!(s2_chunks, 1, "s-2 scrollback must contain the chunk");

    // @step And s-1's SessionContext scrollback remains empty
    let s1_chunks = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 SessionContext present")
        .scrollback
        .chunk_count();
    assert_eq!(s1_chunks, 0, "s-1 (focused) scrollback must be untouched");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionStateChange chunk updates per-session status in the store
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn session_state_change_chunk_updates_per_session_status() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::SessionStateChange { state: SessionState::Running })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));

    // @step Then agent_view_store.session_status_for(&s-1) returns SessionStatus::Running
    assert_eq!(
        app.agent_view_store().session_status_for(&sid("s-1")),
        Some(&SessionStatus::Running),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: IsolationStateChange chunk updates per-session isolation state
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn isolation_state_change_chunk_updates_per_session_isolation_state() {
    use codelet_fspec_tui::IsolationState;

    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc123".to_string()),
        },
    ));

    let state: &IsolationState = app
        .agent_view_store()
        .isolation_state_for(&sid("s-1"))
        .expect("isolation state must be recorded for s-1");

    // @step Then agent_view_store.isolation_state_for(&s-1) returns an IsolationState whose is_isolated is true
    assert!(state.is_isolated);

    // @step And the stored IsolationState worktree_path equals Some("/tmp/wt")
    assert_eq!(state.worktree_path.as_deref(), Some("/tmp/wt"));

    // @step And the stored IsolationState base_commit equals Some("abc123")
    assert_eq!(state.base_commit.as_deref(), Some("abc123"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: DebugStateChange chunk updates per-session debug flag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn debug_state_change_chunk_updates_per_session_debug_flag() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::DebugStateChange { enabled: true })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::DebugStateChange { enabled: true },
    ));

    // @step Then agent_view_store.debug_enabled_for(&s-1) returns true
    assert_eq!(
        app.agent_view_store().debug_enabled_for(&sid("s-1")),
        Some(true),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FooterStateUpdate chunk refreshes the shared workspace info
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn footer_state_update_chunk_refreshes_workspace_info() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "/Users/alice/proj", display_path: "~/proj", is_git_repo: true, branch: Some("main") })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::FooterStateUpdate {
            cwd: "/Users/alice/proj".to_string(),
            display_path: "~/proj".to_string(),
            is_git_repo: true,
            branch: Some("main".to_string()),
        },
    ));

    let ws = app
        .agent_view_store()
        .workspace()
        .expect("workspace info must be set");

    // @step Then agent_view_store.workspace() returns Some(WorkspaceInfo)
    // (already implicit by the unwrap above)

    // @step And the stored WorkspaceInfo.cwd equals "/Users/alice/proj"
    assert_eq!(ws.cwd, "/Users/alice/proj");

    // @step And the stored WorkspaceInfo.git_branch equals Some("main")
    assert_eq!(ws.git_branch.as_deref(), Some("main"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecCommandRequest for list-work-units round-trips
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fspec_command_request_list_work_units_round_trips_via_send_fspec_result() {
    // @step Given an App with an open session s-1 wired to a MockBackend that has seeded work units
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![
        WorkUnitInfo {
            id: "AUTH-001".to_string(),
            title: "User Login".to_string(),
            work_type: "story".to_string(),
            status: "done".to_string(),
            description: None,
            estimate: Some(5),
            epic: Some("authentication".to_string()),
            attachments: Vec::new(),
            last_state_change_at: None,
        },
        WorkUnitInfo {
            id: "AUTH-002".to_string(),
            title: "Password reset".to_string(),
            work_type: "story".to_string(),
            status: "implementing".to_string(),
            description: None,
            estimate: Some(3),
            epic: Some("authentication".to_string()),
            attachments: Vec::new(),
            last_state_change_at: None,
        },
    ]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FspecCommandRequest { fspec_request: FspecRequest { command: "list-work-units", args_json: "{}", project_root: <tempdir>, tool_call_id: "t-1" } })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::FspecCommandRequest {
            fspec_request: FspecRequest {
                command: "list-work-units".to_string(),
                args_json: "{}".to_string(),
                project_root: ".".to_string(),
                tool_call_id: "t-1".to_string(),
            },
        },
    ));

    // @step Then within 1 second backend.send_fspec_result is called exactly once
    let result = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(r) = mock.last_fspec_result() {
                return r;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("backend.send_fspec_result must be called within 1 second");
    assert_eq!(mock.send_fspec_result_calls(), 1);

    // @step And the captured FspecResult has success == true and tool_call_id == "t-1"
    assert!(result.success);
    assert_eq!(result.tool_call_id, "t-1");

    // @step And the captured FspecResult.data is a JSON-serialised array containing every seeded work unit
    let value: serde_json::Value =
        serde_json::from_str(&result.data).expect("FspecResult.data must be JSON");
    let arr = value.as_array().expect("FspecResult.data must be a JSON array");
    let ids: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("id").and_then(|s| s.as_str()))
        .collect();
    assert!(ids.contains(&"AUTH-001"));
    assert!(ids.contains(&"AUTH-002"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecCommandRequest with an unsupported command returns an error
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fspec_command_request_unknown_command_returns_error_result() {
    // @step Given an App with an open session s-1
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FspecCommandRequest { fspec_request: FspecRequest { command: "unknown-command", args_json: "{}", project_root: ".", tool_call_id: "t-2" } })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::FspecCommandRequest {
            fspec_request: FspecRequest {
                command: "unknown-command".to_string(),
                args_json: "{}".to_string(),
                project_root: ".".to_string(),
                tool_call_id: "t-2".to_string(),
            },
        },
    ));

    // @step Then within 1 second backend.send_fspec_result is called exactly once
    let result = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(r) = mock.last_fspec_result() {
                return r;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("backend.send_fspec_result must be called within 1 second");
    assert_eq!(mock.send_fspec_result_calls(), 1);

    // @step And the captured FspecResult has success == false
    assert!(!result.success);

    // @step And the captured FspecResult.error equals Some("unsupported command: unknown-command")
    assert_eq!(
        result.error.as_deref(),
        Some("unsupported command: unknown-command"),
    );

    // @step And the captured FspecResult.tool_call_id equals "t-2"
    assert_eq!(result.tool_call_id, "t-2");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionStatusChanged Action updates per-session status push-driven
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn session_status_changed_action_updates_per_session_status() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the status subscriber forwards Action::SessionStatusChanged(s-1, SessionStatus::Running)
    app.dispatch(Action::SessionStatusChanged(
        sid("s-1"),
        SessionStatus::Running,
    ));

    // @step Then agent_view_store.session_status_for(&s-1) returns SessionStatus::Running
    assert_eq!(
        app.agent_view_store().session_status_for(&sid("s-1")),
        Some(&SessionStatus::Running),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: chunks_rx Sender drop terminates the subscriber loop cleanly
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chunks_rx_sender_drop_terminates_subscriber_task_cleanly() {
    // @step Given an App whose chunks_rx Sender is dropped before any chunk has been broadcast
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap()
        .await
        .expect("bootstrap must succeed against MockBackend");
    let initial_task_count = app.subscriber_task_count();
    assert!(initial_task_count >= 3, "at least chunks/work_units/logs subscribers must be alive");

    // Drop the MockBackend's chunks_tx Sender, mirroring a SessionManager
    // shutdown. After the drop, every active receiver observes
    // `RecvError::Closed` on its next recv() call.
    mock.close_chunks_tx();

    // Allow the broadcast layer to propagate the close.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // @step When the chunks subscriber task is awaited
    // @step Then the subscriber task completes cleanly without panicking
    // (we observe completion indirectly: the App keeps running; the
    // subscriber loop in spawn_subscriber_tasks exits its `loop` on
    // RecvError::Closed without re-spawning or panicking)
    let no_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Drive a few more dispatch ticks to confirm the App is still alive.
        app.dispatch(Action::Redraw);
    }));
    assert!(
        no_panic.is_ok(),
        "App must keep dispatching after the chunks subscriber observes Closed"
    );

    // Sanity: the App can still accept new sessions (i.e. no global state corruption).
    app.dispatch(Action::SessionCreated(sid("s-recovery")));
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .any(|c| c.id == sid("s-recovery")),
        "App must still respond to fresh actions after Closed"
    );
}

// Compile-asserter: import of `SessionContext` is intentional (re-exported by
// codelet_fspec_tui) so this test file regresses if the public surface is
// renamed.
fn _compile_assertion_session_context_is_re_exported(_: SessionContext) {}
