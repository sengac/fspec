// Feature: spec/features/a-submission-goes-to-the-session-it-was-typed-in.feature
//! BUG-552 — a submission goes to the session it was typed in.
//!
//! Drives the real `App` (handle_event + dispatch) against the recording
//! `MockBackend` on a paused tokio clock (BUG-549 harness style). The race
//! is reproduced deterministically: a focus change is put on the App's
//! action channel first ("waiting to be handled"), then the submit key is
//! handled, which queues the submission behind it; draining the channel
//! then handles the focus change BEFORE the submission, exactly as the run
//! loop would.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{InputMode, SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

/// Comfortably past the 300ms draft-save debounce.
const SAVE_WINDOW: Duration = Duration::from_millis(400);

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// An agent-view App with one created session per name; the last one is
/// focused (as after the create-session dialog).
async fn app_with(sessions: &[&str]) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    drain(&mut app).await?;
    Ok((app, mock))
}

/// Run every parked task and queued action until the App is quiet.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..50 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Ok(())
}

/// Move the paused clock past the draft-save debounce and settle.
async fn pass_save_window(app: &mut App) -> TestResult {
    for _ in 0..2 {
        tokio::time::advance(SAVE_WINDOW).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        drain(app).await?;
    }
    Ok(())
}

fn render(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
}

/// Render and deliver one key WITHOUT handling the queued actions, so a
/// submission it produces waits behind anything already queued.
fn key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    render(app);
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
}

async fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    key(app, code, modifiers);
    drain(app).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for character in text.chars() {
        press(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

/// Put `action` on the App's action channel without handling it: a focus
/// change that is waiting to be handled.
fn queue_action(app: &App, action: Action) -> TestResult {
    app.action_tx_clone()
        .send(action)
        .map_err(|error| TestError::from(format!("queue the waiting action: {error}")))
}

/// Focus s-1 (the first tab), type `s2_draft` into s-2 first when given.
async fn s2_holds_draft_then_focus_s1(app: &mut App, s2_draft: &str) -> TestResult {
    assert_eq!(app.current_session(), Some(sid("s-2")));
    type_text(app, s2_draft).await?;
    press(app, KeyCode::Left, KeyModifiers::SHIFT).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    Ok(())
}

fn composer(app: &App) -> String {
    app.navigator().agent.input.value()
}

fn memory_draft(app: &App, session: &str) -> Result<String, TestError> {
    Ok(app
        .agent_view_store()
        .session_context_for(&sid(session))
        .or_fail(&format!("session {session} is open"))?
        .input_draft
        .clone())
}

async fn daemon_draft(mock: &MockBackend, session: &str) -> Result<String, TestError> {
    let stored = FspecBackend::get_pending_input(mock, sid(session))
        .await
        .or_fail("read the daemon draft")?;
    Ok(stored.unwrap_or_default())
}

fn scrollback_text(app: &App, session: &str) -> String {
    app.agent_view_store()
        .session_context_for(&sid(session))
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| {
            chunk.lines.iter().map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────
// Text for the model goes to the typed-in session (rules 1-3)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A prompt sent while a chip click is waiting goes to the session it was typed in
#[tokio::test(start_paused = true)]
async fn a_prompt_sent_while_a_chip_click_is_waiting_goes_to_the_typed_in_session() -> TestResult {
    // @step Given session s-2 holds the draft "s2 text"
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    // @step And session s-1 is focused
    s2_holds_draft_then_focus_s1(&mut app, "s2 text").await?;
    type_text(&mut app, "deploy the fix").await?;
    // @step And a chip click focusing s-2 is waiting to be handled
    queue_action(&app, Action::MenuChipActivate(1))?;
    // @step When the user types "deploy the fix" and presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then "deploy the fix" is sent to s-1
    assert_eq!(
        mock.last_send_input(),
        Some((sid("s-1"), "deploy the fix".to_string()))
    );
    // @step And nothing is sent to s-2
    assert_eq!(mock.send_input_calls(), 1);
    assert!(mock.submitted_inputs().is_empty());
    // @step And the agent view shows s-2 with the composer holding "s2 text"
    assert_eq!(app.current_session(), Some(sid("s-2")));
    assert_eq!(composer(&app), "s2 text");
    // @step And the in-memory draft and the daemon draft of s-2 are "s2 text"
    assert_eq!(memory_draft(&app, "s-2")?, "s2 text");
    assert_eq!(daemon_draft(&mock, "s-2").await?, "s2 text");
    // @step And the daemon draft of s-1 is empty
    assert_eq!(daemon_draft(&mock, "s-1").await?, "");
    Ok(())
}

// Scenario: A steer submitted while a new session is taking focus goes to the running session
#[tokio::test(start_paused = true)]
async fn a_steer_submitted_while_a_new_session_is_taking_focus_goes_to_the_running_session(
) -> TestResult {
    // @step Given session s-1 is focused and its turn is running
    let (mut app, mock) = app_with(&["s-1"]).await?;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    type_text(&mut app, "also check the logs").await?;
    // @step And a new session s-2 taking focus is waiting to be handled
    queue_action(&app, Action::SessionCreated(sid("s-2")))?;
    // @step When the user types "also check the logs" and presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then "also check the logs" is submitted to s-1 as a steer
    // @step And nothing is submitted to s-2
    assert_eq!(
        mock.submitted_inputs(),
        vec![(
            sid("s-1"),
            "also check the logs".to_string(),
            InputMode::Steer
        )]
    );
    assert_eq!(mock.send_input_calls(), 0);
    // @step And the agent view shows s-2
    assert_eq!(app.current_session(), Some(sid("s-2")));
    Ok(())
}

// Scenario: A queued input submitted while an attach is waiting is queued on the session it was typed in
#[tokio::test(start_paused = true)]
async fn a_queued_input_submitted_while_an_attach_is_waiting_is_queued_on_the_typed_in_session(
) -> TestResult {
    // @step Given session s-1 is focused and its turn is running
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    press(&mut app, KeyCode::Left, KeyModifiers::SHIFT).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And session s-2 is open
    assert!(app
        .agent_view_store()
        .session_context_for(&sid("s-2"))
        .is_some());
    type_text(&mut app, "next task").await?;
    // @step And an attach to s-2 is waiting to be handled
    queue_action(&app, Action::AttachToSession(sid("s-2")))?;
    // @step When the user types "next task" and presses Alt+Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::ALT);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then "next task" is submitted to s-1 as a queued input
    // @step And nothing is submitted to s-2
    assert_eq!(
        mock.submitted_inputs(),
        vec![(sid("s-1"), "next task".to_string(), InputMode::Queue)]
    );
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the queued inputs of s-1 show "next task"
    let queued: Vec<String> = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .or_fail("session s-1 is open")?
        .busy_inputs
        .queued()
        .iter()
        .map(|input| input.text.clone())
        .collect();
    assert_eq!(queued, vec!["next task".to_string()]);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Slash commands (rule 4)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A /clear submitted while focus is moving clears no other session
#[tokio::test(start_paused = true)]
async fn a_clear_submitted_while_focus_is_moving_clears_no_other_session() -> TestResult {
    // @step Given session s-2 holds the draft "s2 text"
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    // @step And session s-1 is focused
    s2_holds_draft_then_focus_s1(&mut app, "s2 text").await?;
    type_text(&mut app, "/clear").await?;
    // The slash popup is open on "/clear"; Esc closes it so Enter submits
    // the typed line rather than picking from the popup.
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE).await?;
    assert_eq!(composer(&app), "/clear");
    // @step And a chip click focusing s-2 is waiting to be handled
    queue_action(&app, Action::MenuChipActivate(1))?;
    // @step When the user types "/clear" and presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then no session history is cleared
    assert_eq!(mock.clear_history_calls(), 0);
    // @step And the composer holds "s2 text"
    assert_eq!(app.current_session(), Some(sid("s-2")));
    assert_eq!(composer(&app), "s2 text");
    // @step And the daemon draft of s-2 is "s2 text"
    assert_eq!(daemon_draft(&mock, "s-2").await?, "s2 text");
    // @step And s-1 shows a notice that /clear was not run because another session was focused
    let scrollback = scrollback_text(&app, "s-1");
    assert!(
        scrollback.contains("/clear was not run: another session was focused"),
        "s-1 must say why /clear was not run:\n{scrollback}"
    );
    Ok(())
}

// Scenario: A /role submitted while focus is moving sets the role on the session it was typed in
#[tokio::test(start_paused = true)]
async fn a_role_submitted_while_focus_is_moving_sets_the_role_on_the_typed_in_session() -> TestResult
{
    // @step Given session s-1 is focused and session s-2 is open
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    press(&mut app, KeyCode::Left, KeyModifiers::SHIFT).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    type_text(&mut app, "/role reviewer").await?;
    // @step And a chip click focusing s-2 is waiting to be handled
    queue_action(&app, Action::MenuChipActivate(1))?;
    // @step When the user types "/role reviewer" and presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step Then the role "reviewer" is set on s-1
    assert_eq!(
        mock.last_set_session_role(),
        Some((sid("s-1"), Some("reviewer".to_string())))
    );
    assert_eq!(mock.set_session_role_calls(), 1);
    Ok(())
}
