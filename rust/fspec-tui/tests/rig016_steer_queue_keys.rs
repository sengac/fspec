//! RIG-016 — the agent view steers on Enter and queues on Alt+Enter while
//! the focused session's turn is running.
//!
//! Feature: spec/features/agent-view-steer-and-queue-keys.feature
//!
//! Harness: App + MockBackend, as the TUI-114 tests; the session's running
//! status is set on the agent-view store the way status broadcasts land.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::{InputMode, SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::MockBackend;

async fn drain(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

async fn key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await?;
    Ok(())
}

async fn type_text(app: &mut App, text: &str) -> Result<(), codelet_test_helpers::TestError> {
    for character in text.chars() {
        key(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

fn render(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(160, 30)).or_fail("terminal")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(())
}

/// The agent view of a new session whose turn is running.
async fn running_agent_view(
) -> Result<(App, Arc<MockBackend>, SessionId), codelet_test_helpers::TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    drain(&mut app).await?;
    let session = SessionId::new("s-busy");
    mock.script_create_session(session.clone());
    key(&mut app, KeyCode::Char('.'), KeyModifiers::NONE).await?;
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    assert_eq!(app.active_view(), ViewMode::Agent);
    app.agent_view_store_mut()
        .set_session_status(session.clone(), SessionStatus::Running);
    render(&mut app)?;
    Ok((app, mock, session))
}

#[tokio::test(flavor = "current_thread")]
async fn enter_steers_and_alt_enter_queues_while_a_turn_runs() -> TestResult {
    // @step Given the agent view of a session whose turn is running
    let (mut app, mock, session) = running_agent_view().await?;

    // @step When the user types "check the logs" and presses Enter
    type_text(&mut app, "check the logs").await?;
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then the input is submitted as a steer
    assert_eq!(
        mock.submitted_inputs(),
        [(
            session.clone(),
            "check the logs".to_string(),
            InputMode::Steer
        )]
    );
    assert_eq!(
        mock.send_input_calls(),
        0,
        "a busy session is not sent a new turn"
    );

    // @step And typing "and then deploy" with Alt+Enter submits it as a queued input
    render(&mut app)?;
    type_text(&mut app, "and then deploy").await?;
    key(&mut app, KeyCode::Enter, KeyModifiers::ALT).await?;
    assert_eq!(
        mock.submitted_inputs().last(),
        Some(&(session, "and then deploy".to_string(), InputMode::Queue))
    );
    assert_eq!(mock.submitted_inputs().len(), 2);
    Ok(())
}
