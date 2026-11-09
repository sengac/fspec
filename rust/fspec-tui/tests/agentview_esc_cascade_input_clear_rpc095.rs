//! Feature: spec/features/agentview-esc-cascade-input-clear.feature
//!
//! Moved out of keyboard_cascade_rpc051.rs (COV-075: one test file per feature).

use crate::common::TaskOutcome;
use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus};
use codelet_test_helpers::TestResult;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::sync::Arc;
mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn esc() -> Event {
    key(KeyCode::Esc, KeyModifiers::NONE)
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

/// Build an App in ViewMode::Agent with a single open session at the
/// supplied status. Returns the App + MockBackend for assertion access.
fn agent_app_with_status(status: SessionStatus) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), status);
    (app, mock)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-095 L6): Esc when idle with non-empty input clears the buffer
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_6_idle_session_with_nonempty_input_clears_buffer() -> TestResult {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Idle
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer contains the text "hello world"
    app.navigator_mut().agent.input.set_value("hello world");

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await?;

    // @step Then the MultiLineInput value equals ""
    assert_eq!(app.navigator().agent.input.value(), "");
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), 0);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-095): Esc when idle with whitespace-only input is treated as empty
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_idle_session_with_whitespace_only_input_navigates_back() -> TestResult {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Idle
    let (mut app, _mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer contains only whitespace "   "
    app.navigator_mut().agent.input.set_value("   ");

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await?;

    // RPC-098: whitespace-only input is treated as empty by trim(), so
    // L6 short-circuits and L7 opens ExitConfirmationDialog.
    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "idle whitespace-only ESC must push ExitConfirmationDialog (RPC-098)"
    );
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    Ok(())
}
