//! App-level snapshot checkpoints for the AgentView flow.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature (RPC-012)
//!   - spec/features/fspec-tui-agent-repl-rpc009.feature (RPC-009,
//!     superseded by RPC-012 for the rendered shape)
//!
//! RPC-012 migrated three insta snapshot checkpoints from the old
//! two-pane layout to the new Navigator-based layout. Each snapshot
//! pins the screen rendering against an 80x24 TestBackend driven by a
//! `MockBackend`:
//!   (a) `repl_bootstrap` — initial frame after bootstrap (BoardView is
//!       visible because Navigator.active_view defaults to Board).
//!   (b) `repl_after_first_chunk` — frame after the App has entered
//!       AgentView (Action::EnterWorkUnit) and processed an
//!       `Action::ChunkReceived` (scrollback shows assistant text).
//!   (c) `repl_after_submit` — frame after the user types text and
//!       presses Enter while AgentView is active (input box cleared,
//!       scrollback shows the user message routed back via
//!       MockBackend::send_input).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, StreamChunk, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::{buffer_to_rows, render_one_frame, MockBackend};

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

/// Build an App against a MockBackend pre-seeded with two work units
/// and a scripted SessionId for lazy creation. Bootstrap completes;
/// pending Actions are drained through `App::dispatch` so the stores
/// reflect the seed list.
async fn bootstrap_app_with_mock() -> (App, Arc<MockBackend>, Terminal<TestBackend>) {
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![
        wu("AUTH-001", "done"),
        wu("AUTH-002", "implementing"),
    ]);
    mock.script_create_session(SessionId::new("s-mock-1"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    let term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    (app, mock, term)
}

fn synth_key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// RPC-012 rendered-frame checkpoint (a): initial frame after bootstrap.
/// Navigator.active_view defaults to Board so the BoardView's seven
/// column headers are visible.
#[tokio::test]
async fn repl_bootstrap_snapshot_captures_initial_frame_after_bootstrap() {
    let (mut app, _mock, mut term) = bootstrap_app_with_mock().await;
    let buf = render_one_frame(&mut term, &mut app);
    let rows = buffer_to_rows(&buf);
    let joined = rows.join("\n");
    assert!(joined.contains("BACKLOG"), "expected BoardView header");
    assert_eq!(app.active_view(), ViewMode::Board);
    insta::assert_yaml_snapshot!("repl_bootstrap_rpc012", rows);
}

/// RPC-012 rendered-frame checkpoint (b): frame after EnterWorkUnit
/// drives the Navigator into Agent view and a ChunkReceived populates
/// the scrollback.
#[tokio::test]
async fn repl_after_first_chunk_snapshot_captures_assistant_text() {
    let (mut app, _mock, mut term) = bootstrap_app_with_mock().await;
    // Enter the AgentView and lazy-create a session.
    app.dispatch(Action::EnterWorkUnit("AUTH-002".to_string()));
    if let Some(handle) = app.next_pending_task() {
        handle.await.expect("lazy create_session join");
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    assert_eq!(app.active_view(), ViewMode::Agent);
    // Stream an assistant chunk for the active session.
    let session = app
        .current_session()
        .expect("lazy create_session must have produced a session");
    app.dispatch(Action::ChunkReceived(
        session,
        StreamChunk::text("Hello!".to_string()),
    ));
    let buf = render_one_frame(&mut term, &mut app);
    let rows = buffer_to_rows(&buf);
    let joined = rows.join("\n");
    assert!(joined.contains("Hello!"), "expected scrollback to include 'Hello!'");
    insta::assert_yaml_snapshot!("repl_after_first_chunk_rpc012", rows);
}

/// RPC-012 rendered-frame checkpoint (c): frame after the user types
/// text and presses Enter while AgentView is active. Tab no longer
/// toggles focus (RPC-012 rule [19]); navigation into AgentView is
/// driven by `EnterWorkUnit` instead.
#[tokio::test]
async fn repl_after_submit_snapshot_captures_input_cleared_after_enter() {
    let (mut app, mock, mut term) = bootstrap_app_with_mock().await;
    // Drive Navigator into Agent view via EnterWorkUnit and complete
    // the lazy create_session task.
    app.dispatch(Action::EnterWorkUnit("AUTH-002".to_string()));
    if let Some(handle) = app.next_pending_task() {
        handle.await.expect("lazy create_session join");
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    assert_eq!(app.active_view(), ViewMode::Agent);

    // Type "hi" and press Enter — characters go to AgentView's input
    // because the Navigator routes events to the active sub-view.
    let _ = app.handle_event(&synth_key(KeyCode::Char('h')));
    let _ = app.handle_event(&synth_key(KeyCode::Char('i')));
    let _ = app.handle_event(&synth_key(KeyCode::Enter));
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(mock.send_input_calls(), 1);
    assert_eq!(
        mock.last_send_input(),
        Some((SessionId::new("s-mock-1"), "hi".to_string())),
    );
    let buf = render_one_frame(&mut term, &mut app);
    let rows = buffer_to_rows(&buf);
    insta::assert_yaml_snapshot!("repl_after_submit_rpc012", rows);
}
