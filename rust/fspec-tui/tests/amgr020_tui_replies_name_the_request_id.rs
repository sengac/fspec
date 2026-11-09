//! Feature: spec/features/questions-and-approvals-are-requests-answered-by-id.feature
//!
//! AMGR-020: the TUI reads a paused session's approval and question with
//! get_pause_state / get_hitl_request, which return the pending request with
//! its id, and answers each prompt naming that id (pause_triple and
//! send_hitl_response), so an answer can never land on a newer request.

use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    ApprovalChoice, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, PendingRequest,
    RequestHolder, RequestKind, RequestRoute, SessionId, SessionState, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::{MockBackend, TaskOutcome};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn route(request_id: &str, session: &str, kind: RequestKind) -> RequestRoute {
    RequestRoute {
        request_id: request_id.into(),
        origin: sid(session),
        kind,
        chain: vec![sid(session)],
        holder: RequestHolder::User,
    }
}

fn enter() -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn render(app: &mut App) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

async fn drain(app: &mut App) -> TestResult {
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

fn paused(app: &mut App) {
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_tui_answers_a_prompt_with_its_request_id() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step Given the TUI shows a session's approval prompt and question prompt read with get_pause_state and get_hitl_request
    mock.script_pending_pause(
        sid("s-1"),
        Some(PendingRequest {
            route: route("req-approval-7", "s-1", RequestKind::Approval),
            payload: PauseState {
                kind: PauseKind::Triple,
                prompt: "Bash: Command requires approval".into(),
                details: None,
                tool_call_id: None,
            },
        }),
    );
    mock.script_pending_hitl(sid("s-1"), None);
    paused(&mut app);
    drain(&mut app).await?;
    render(&mut app);

    // @step When the user approves the approval and submits the question
    app.handle_event(&enter());
    drain(&mut app).await?;
    mock.script_pending_pause(sid("s-1"), None);
    mock.script_pending_hitl(
        sid("s-1"),
        Some(PendingRequest {
            route: route("req-question-8", "s-1", RequestKind::Question),
            payload: HitlRequest {
                questions: vec![HitlQuestion {
                    id: "auth_library".into(),
                    header: "Auth".into(),
                    question: "Which auth library?".into(),
                    options: vec![
                        HitlOption {
                            label: "jose".into(),
                            description: "Use jose.".into(),
                        },
                        HitlOption {
                            label: "jsonwebtoken".into(),
                            description: "Use jsonwebtoken.".into(),
                        },
                    ],
                }],
            },
        }),
    );
    paused(&mut app);
    drain(&mut app).await?;
    render(&mut app);
    app.handle_event(&enter());
    drain(&mut app).await?;

    // @step Then the backend receives pause_triple and send_hitl_response naming that session and each request's id
    assert_eq!(
        mock.pause_triple_log(),
        vec![(sid("s-1"), ApprovalChoice::Approve)]
    );
    assert_eq!(mock.send_hitl_response_calls(), 1);
    assert_eq!(
        mock.reply_request_ids(),
        vec![
            ("pause_triple", sid("s-1"), "req-approval-7".to_string()),
            (
                "send_hitl_response",
                sid("s-1"),
                "req-question-8".to_string()
            ),
        ]
    );
    Ok(())
}
