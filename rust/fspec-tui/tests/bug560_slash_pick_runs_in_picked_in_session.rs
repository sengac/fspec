// Feature: spec/features/a-command-picked-from-the-slash-popup-runs-in-the-session-it-was-picked-in.feature
//! BUG-560 — a command picked from the slash popup runs in the session it
//! was picked in.
//!
//! Same deterministic harness as BUG-552 (paused tokio clock, recording
//! `MockBackend`, the real `App` driven through handle_event + dispatch).
//! The race: a focus change is put on the App's action channel first
//! ("waiting to be handled"), then Enter picks the highlighted popup row,
//! which queues the picked command behind it; draining the channel handles
//! the focus change BEFORE the command, exactly as the run loop would.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
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
/// command it picks waits behind anything already queued.
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

/// Type `command` so the slash popup opens on it with its row highlighted
/// (the pick itself is the Enter key handled by the scenario).
async fn open_popup_on(app: &mut App, command: &str) -> TestResult {
    type_text(app, command).await?;
    assert_eq!(composer(app), command);
    Ok(())
}

/// Put `action` on the App's action channel without handling it: a focus
/// change that is waiting to be handled.
fn queue_action(app: &App, action: Action) -> TestResult {
    app.action_tx_clone()
        .send(action)
        .map_err(|error| TestError::from(format!("queue the waiting action: {error}")))
}

/// Type `s2_draft` into the focused s-2, then focus s-1 (the first tab).
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

const NOT_RUN: &str = "was not run: another session was focused";

fn assert_not_run_notice(app: &App, session: &str, command: &str) {
    let scrollback = scrollback_text(app, session);
    assert!(
        scrollback.contains(&format!("{command} {NOT_RUN}")),
        "{session} must say why {command} was not run:\n{scrollback}"
    );
}

fn assert_no_not_run_notice(app: &App, session: &str) {
    let scrollback = scrollback_text(app, session);
    assert!(
        !scrollback.contains(NOT_RUN),
        "{session} must show no not-run notice:\n{scrollback}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// A focus change waiting to be handled never takes the pick (rules 1-3)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A /clear picked while a chip click is waiting clears no other session
#[tokio::test(start_paused = true)]
async fn a_clear_picked_while_a_chip_click_is_waiting_clears_no_other_session() -> TestResult {
    // @step Given session s-2 holds the draft "s2 text"
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    // @step And session s-1 is focused
    s2_holds_draft_then_focus_s1(&mut app, "s2 text").await?;
    open_popup_on(&mut app, "/clear").await?;
    // @step And a chip click focusing s-2 is waiting to be handled
    queue_action(&app, Action::MenuChipActivate(1))?;
    // @step When the user picks "/clear" from the slash popup
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then no session history is cleared
    assert_eq!(mock.clear_history_calls(), 0);
    // @step And the agent view shows s-2 with the composer holding "s2 text"
    assert_eq!(app.current_session(), Some(sid("s-2")));
    assert_eq!(composer(&app), "s2 text");
    // @step And the daemon draft of s-2 is "s2 text"
    assert_eq!(daemon_draft(&mock, "s-2").await?, "s2 text");
    // @step And s-1 shows a notice that /clear was not run because another session was focused
    assert_not_run_notice(&app, "s-1", "/clear");
    Ok(())
}

// Scenario: A /compact picked while a new session is taking focus compacts no session
#[tokio::test(start_paused = true)]
async fn a_compact_picked_while_a_new_session_is_taking_focus_compacts_no_session() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    open_popup_on(&mut app, "/compact").await?;
    // @step And a new session s-2 taking focus is waiting to be handled
    queue_action(&app, Action::SessionCreated(sid("s-2")))?;
    // @step When the user picks "/compact" from the slash popup
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then no session is compacted
    assert_eq!(mock.compact_session_calls(), 0);
    // @step And the agent view shows s-2
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And s-1 shows a notice that /compact was not run because another session was focused
    assert_not_run_notice(&app, "s-1", "/compact");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Without a focus change a pick runs as before (rule 4)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A /clear picked with nothing waiting clears the focused session
#[tokio::test(start_paused = true)]
async fn a_clear_picked_with_nothing_waiting_clears_the_focused_session() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    open_popup_on(&mut app, "/clear").await?;
    // @step When the user picks "/clear" from the slash popup
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then the history of s-1 is cleared on the daemon
    assert_eq!(mock.clear_history_calls(), 1);
    assert_eq!(mock.last_clear_history_session(), Some(sid("s-1")));
    // @step And s-1 shows no notice that /clear was not run
    assert_no_not_run_notice(&app, "s-1");
    Ok(())
}

// Scenario: A /help picked while no session is open opens the help dialog
#[tokio::test(start_paused = true)]
async fn a_help_picked_while_no_session_is_open_opens_the_help_dialog() -> TestResult {
    // @step Given no session is open
    let (mut app, _mock) = app_with(&[]).await?;
    assert_eq!(app.current_session(), None);
    open_popup_on(&mut app, "/help").await?;
    // @step When the user picks "/help" from the slash popup
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then the help dialog is open
    assert!(app.compositor().contains("help-dialog"));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// A pick made in no session (rule 5)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A /clear picked while no session is open is not run on a session that takes focus first
#[tokio::test(start_paused = true)]
async fn a_clear_picked_while_no_session_is_open_is_not_run_on_a_session_that_takes_focus_first(
) -> TestResult {
    // @step Given no session is open
    let (mut app, mock) = app_with(&[]).await?;
    assert_eq!(app.current_session(), None);
    open_popup_on(&mut app, "/clear").await?;
    // @step And a new session s-1 taking focus is waiting to be handled
    queue_action(&app, Action::SessionCreated(sid("s-1")))?;
    // @step When the user picks "/clear" from the slash popup
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // @step And the TUI handles the waiting actions
    drain(&mut app).await?;
    // @step Then no session history is cleared
    assert_eq!(mock.clear_history_calls(), 0);
    // @step And the agent view shows s-1
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And s-1 shows no notice that /clear was not run
    assert_no_not_run_notice(&app, "s-1");
    Ok(())
}
