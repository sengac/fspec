// Feature: spec/features/paused-prompt-write-failures.feature
//! BUG-395 — a failed pause answer, tool-approval choice, pause resume or
//! question answer is reported in the session scrollback and the prompt is
//! read back from the backend; without a tokio runtime the prompt stays.

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    ApprovalChoice, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId, SessionState,
    StreamChunk,
};

mod common;
use common::MockBackend;
use common::{hitl_route, pause_route};

fn session() -> SessionId {
    SessionId::new("s-1")
}

fn new_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(session()));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

fn pause_prompt(kind: PauseKind) -> PauseState {
    PauseState {
        kind,
        prompt: "Run rm?".to_owned(),
        details: None,
        tool_call_id: None,
    }
}

fn question() -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "notes".to_owned(),
            header: "Notes".to_owned(),
            question: "Any notes?".to_owned(),
            options: vec![],
        }],
    }
}

/// Run every spawned task and every action it sends back until both drain.
async fn drain(app: &mut App) -> TestResult {
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

/// A session whose backend is paused on `pause` / `hitl`, with the Paused
/// probe already run so the prompt is shown.
async fn paused_app(
    pause: Option<PauseState>,
    hitl: Option<HitlRequest>,
) -> Result<(App, Arc<MockBackend>), Box<dyn std::error::Error>> {
    let (mut app, mock) = new_app();
    mock.script_pause_state(session(), pause);
    mock.script_hitl_request(session(), hitl);
    app.dispatch(Action::ChunkReceived(
        session(),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    drain(&mut app).await?;
    Ok((app, mock))
}

/// True when a scrollback line reports `write` failing with `reason`.
fn shows_failed_write(
    app: &App,
    write: &str,
    reason: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let ctx = app
        .agent_view_store()
        .session_context_for(&session())
        .ok_or("session context")?;
    let needle = format!("{write} failed");
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
        .any(|text| text.contains(&needle) && text.contains(reason)))
}

fn pause_kind_shown(app: &App) -> Option<PauseKind> {
    app.agent_view_store()
        .pause_state_for(&session())
        .map(|state| state.kind)
}

fn question_shown(app: &App) -> bool {
    app.agent_view_store().hitl_prompt_for(&session()).is_some()
}

#[tokio::test]
async fn scenario_failed_pause_answer_is_reported_and_the_pause_prompt_comes_back() -> TestResult {
    // @step Given a session paused with a confirm pause prompt shown
    let (mut app, mock) = paused_app(Some(pause_prompt(PauseKind::Confirm)), None).await?;
    assert_eq!(pause_kind_shown(&app), Some(PauseKind::Confirm));
    // @step And sending a pause answer fails with "connection reset"
    mock.set_pause_confirm_error("connection reset".to_owned());
    // @step When the user accepts the pause prompt
    app.dispatch(Action::PauseConfirmed {
        session_id: session(),
        accept: true,
    });
    drain(&mut app).await?;
    // @step Then the session scrollback shows that pause_confirm failed with "connection reset"
    assert!(shows_failed_write(
        &app,
        "pause_confirm",
        "connection reset"
    )?);
    // @step And the confirm pause prompt is shown again
    assert_eq!(pause_kind_shown(&app), Some(PauseKind::Confirm));
    Ok(())
}

#[tokio::test]
async fn scenario_failed_tool_approval_choice_is_reported_and_the_approval_prompt_comes_back(
) -> TestResult {
    // @step Given a session paused with a tool-approval prompt shown
    let (mut app, mock) = paused_app(Some(pause_prompt(PauseKind::Triple)), None).await?;
    assert_eq!(pause_kind_shown(&app), Some(PauseKind::Triple));
    // @step And sending a tool-approval choice fails with "connection reset"
    mock.set_pause_triple_error("connection reset".to_owned());
    // @step When the user approves the tool call once
    app.dispatch(Action::PauseTriple {
        session_id: session(),
        choice: ApprovalChoice::Approve,
    });
    drain(&mut app).await?;
    // @step Then the session scrollback shows that pause_triple failed with "connection reset"
    assert!(shows_failed_write(
        &app,
        "pause_triple",
        "connection reset"
    )?);
    // @step And the tool-approval prompt is shown again
    assert_eq!(pause_kind_shown(&app), Some(PauseKind::Triple));
    Ok(())
}

#[tokio::test]
async fn scenario_failed_question_answer_is_reported_and_the_question_comes_back() -> TestResult {
    // @step Given a session paused with a question shown
    let (mut app, mock) = paused_app(None, Some(question())).await?;
    assert!(question_shown(&app));
    // @step And sending a question answer fails with "connection reset"
    mock.set_send_hitl_response_error("connection reset".to_owned());
    // @step When the user submits an answer
    app.dispatch(Action::HitlAnswerCaptured {
        session_id: session(),
        text: "none".to_owned(),
    });
    drain(&mut app).await?;
    // @step Then the session scrollback shows that send_hitl_response failed with "connection reset"
    assert!(shows_failed_write(
        &app,
        "send_hitl_response",
        "connection reset"
    )?);
    // @step And the question is shown again
    assert!(question_shown(&app));
    Ok(())
}

#[test]
fn scenario_without_a_runtime_a_pause_answer_is_reported_and_the_pause_prompt_stays() -> TestResult
{
    // @step Given a session with a confirm pause prompt shown and no tokio runtime
    let (mut app, _mock) = new_app();
    app.dispatch(Action::PauseStateFetched {
        session_id: session(),
        route: pause_route(&session()),
        state: pause_prompt(PauseKind::Confirm),
    });
    // @step When the user accepts the pause prompt
    app.dispatch(Action::PauseConfirmed {
        session_id: session(),
        accept: true,
    });
    // @step Then the session scrollback shows that pause_confirm failed with "no tokio runtime"
    assert!(shows_failed_write(
        &app,
        "pause_confirm",
        "no tokio runtime"
    )?);
    // @step And the confirm pause prompt is still shown
    assert_eq!(pause_kind_shown(&app), Some(PauseKind::Confirm));
    Ok(())
}

#[test]
fn scenario_without_a_runtime_a_question_answer_is_reported_and_the_question_stays() -> TestResult {
    // @step Given a session with a question shown and no tokio runtime
    let (mut app, _mock) = new_app();
    app.dispatch(Action::HitlPromptFetched {
        session_id: session(),
        route: hitl_route(&session()),
        request: question(),
    });
    // @step When the user submits an answer
    app.dispatch(Action::HitlAnswerCaptured {
        session_id: session(),
        text: "none".to_owned(),
    });
    // @step Then the session scrollback shows that send_hitl_response failed with "no tokio runtime"
    assert!(shows_failed_write(
        &app,
        "send_hitl_response",
        "no tokio runtime"
    )?);
    // @step And the question is still shown
    assert!(question_shown(&app));
    Ok(())
}
