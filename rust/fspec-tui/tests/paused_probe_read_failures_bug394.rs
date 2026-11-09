// Feature: spec/features/paused-probe-read-failures.feature
//! BUG-394 — a failed pause or question read in the Paused probe is
//! reported in the session scrollback instead of being dropped.

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{PauseKind, PauseState, SessionId, SessionState, StreamChunk};

mod common;
use common::MockBackend;

fn session() -> SessionId {
    SessionId::new("s-1")
}

fn paused_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(session()));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Deliver the Paused chunk and drain every task and action it causes.
async fn run_paused_probe(app: &mut App) -> TestResult {
    app.dispatch(Action::ChunkReceived(
        session(),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    loop {
        while let Some(task) = app.next_pending_task() {
            codelet_test_helpers::deadlock_guard("the pending task", task)
                .await?
                .completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            return Ok(());
        };
        app.dispatch(action);
    }
}

fn no_prompt(app: &App) -> bool {
    let store = app.agent_view_store();
    store.hitl_prompt_for(&session()).is_none() && store.pause_state_for(&session()).is_none()
}

/// True when a scrollback line reports `read` failing with "decode failed".
fn shows_failed_read(app: &App, read: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let ctx = app
        .agent_view_store()
        .session_context_for(&session())
        .ok_or("session context")?;
    let needle = format!("{read} failed");
    Ok(ctx
        .scrollback
        .visible_window(1024)
        .iter()
        .flat_map(|chunk| chunk.lines.iter())
        .map(|line| {
            line.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .any(|text| text.contains(&needle) && text.contains("decode failed")))
}

#[tokio::test]
async fn scenario_failed_question_read_is_reported_and_sets_no_prompt() -> TestResult {
    // @step Given a session paused with no pause prompt
    let (mut app, mock) = paused_app();
    mock.script_pause_state(session(), None);
    // @step When the question read fails during the Paused probe
    mock.set_get_hitl_request_error("decode failed".to_owned());
    run_paused_probe(&mut app).await?;
    // @step Then no question or pause prompt is set for the session
    assert!(no_prompt(&app));
    // @step And the session scrollback shows the failed question read
    assert!(shows_failed_read(&app, "get_hitl_request")?);
    Ok(())
}

#[tokio::test]
async fn scenario_failed_pause_read_is_reported_and_sets_no_prompt() -> TestResult {
    // @step Given a session paused with no question
    let (mut app, mock) = paused_app();
    mock.script_hitl_request(session(), None);
    // @step When the pause read fails during the Paused probe
    mock.set_get_pause_state_error("decode failed".to_owned());
    run_paused_probe(&mut app).await?;
    // @step Then no question or pause prompt is set for the session
    assert!(no_prompt(&app));
    // @step And the session scrollback shows the failed pause read
    assert!(shows_failed_read(&app, "get_pause_state")?);
    Ok(())
}

#[tokio::test]
async fn scenario_failed_question_read_does_not_let_a_pause_prompt_take_its_place() -> TestResult {
    // @step Given a session paused with a confirm pause prompt
    let (mut app, mock) = paused_app();
    let prompt = PauseState {
        kind: PauseKind::Confirm,
        prompt: "Run rm?".to_owned(),
        details: None,
        tool_call_id: None,
    };
    mock.script_pause_state(session(), Some(prompt));
    // @step When the question read fails during the Paused probe
    mock.set_get_hitl_request_error("decode failed".to_owned());
    run_paused_probe(&mut app).await?;
    // @step Then no pause prompt is shown for the session
    assert!(no_prompt(&app));
    // @step And the session scrollback shows the failed question read
    assert!(shows_failed_read(&app, "get_hitl_request")?);
    Ok(())
}
