// Feature: spec/features/opening-an-agent-view-from-the-board-focuses-that-session.feature
//! BUG-553 — opening an agent view from the board focuses exactly the
//! requested session, through the same focus path as every other route.
//!
//! Drives the real `App` (handle_event + dispatch) against the recording
//! `MockBackend` on a paused tokio clock (BUG-549 harness style): the
//! daemon's active-session pointer, the supervisor load, the transcript
//! resume and the pending-input store are read back from the mock.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn work_unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// An agent-view App with one created session per name; the last one is
/// focused (as after the create-session dialog). The board holds the work
/// unit `unit`, selected, attached to `attached` when given.
async fn app_with(
    sessions: &[&str],
    unit: &str,
    attached: Option<&str>,
) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.board_store_mut()
        .replace_work_units(vec![work_unit(unit)]);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    app.navigator_mut().active_view = ViewMode::Agent;
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    if let Some(session) = attached {
        app.board_store_mut().attach_session(unit, sid(session));
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

fn render(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
}

async fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    render(app);
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for character in text.chars() {
        press(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

async fn shift(app: &mut App, code: KeyCode) -> TestResult {
    press(app, code, KeyModifiers::SHIFT).await
}

/// Leave the agent view for the board (the Esc / back-to-board route).
async fn go_to_board(app: &mut App) -> TestResult {
    app.dispatch(Action::BackToBoard);
    drain(app).await?;
    render(app);
    assert_eq!(app.navigator().active_view, ViewMode::Board);
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

/// The agent view is showing and it shows `session`.
fn assert_agent_view_shows(app: &App, session: &str) {
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    assert_eq!(app.current_session(), Some(sid(session)));
}

// ─────────────────────────────────────────────────────────────────────
// Shift+Right on a work unit with an attached session (rules 1, 2, 5)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Shift+Right on a work unit attached to an open session focuses that session
#[tokio::test(start_paused = true)]
async fn shift_right_on_a_work_unit_attached_to_an_open_session_focuses_that_session() -> TestResult
{
    // @step Given sessions s-1 and s-2 are open
    let (mut app, mock) = app_with(&["s-1", "s-2"], "AUTH-001", Some("s-1")).await?;
    // @step And session s-1 holds the draft "s1 text" and is attached to work unit AUTH-001
    shift(&mut app, KeyCode::Left).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    type_text(&mut app, "s1 text").await?;
    // @step And session s-2 is focused with the draft "s2 text"
    shift(&mut app, KeyCode::Right).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    type_text(&mut app, "s2 text").await?;
    mock.seed_supervisors_for(sid("s-1"), vec![sid("sup-1")]);
    // @step And the user is on the board with AUTH-001 selected
    go_to_board(&mut app).await?;
    // @step When the user presses Shift+Right
    shift(&mut app, KeyCode::Right).await?;
    // @step Then the agent view shows session s-1
    assert_agent_view_shows(&app, "s-1");
    // @step And the composer shows "s1 text"
    assert_eq!(composer(&app), "s1 text");
    // @step And the in-memory draft of s-2 is "s2 text"
    assert_eq!(memory_draft(&app, "s-2")?, "s2 text");
    // @step And the daemon's active session is s-1
    assert_eq!(mock.last_set_active_session(), Some(sid("s-1")));
    // @step And the supervisors of s-1 are loaded
    assert_eq!(
        app.agent_view_store().supervisors_for(&sid("s-1")),
        [sid("sup-1")].as_slice()
    );
    Ok(())
}

// Scenario: Shift+Right on a work unit attached to a session with no open tab attaches it
#[tokio::test(start_paused = true)]
async fn shift_right_on_a_work_unit_attached_to_a_session_with_no_open_tab_attaches_it(
) -> TestResult {
    // @step Given session s-1 is open and focused
    let (mut app, mock) = app_with(&["s-1"], "AUTH-001", Some("s-3")).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And work unit AUTH-001 is attached to session s-3, which has no open tab
    assert!(app
        .agent_view_store()
        .session_context_for(&sid("s-3"))
        .is_none());
    // @step And the daemon holds the draft "resume me" for session s-3
    mock.script_pending_input(sid("s-3"), Some("resume me".to_string()));
    // @step And the user is on the board with AUTH-001 selected
    go_to_board(&mut app).await?;
    // @step When the user presses Shift+Right
    shift(&mut app, KeyCode::Right).await?;
    // @step Then the agent view shows session s-3
    assert_agent_view_shows(&app, "s-3");
    // @step And session s-3 has an open tab
    assert!(app
        .agent_view_store()
        .session_context_for(&sid("s-3"))
        .is_some());
    // @step And the transcript of s-3 is resumed
    assert_eq!(mock.last_resume_session(), Some(sid("s-3")));
    // @step And the composer shows "resume me"
    assert_eq!(composer(&app), "resume me");
    // @step And the daemon's active session is s-3
    assert_eq!(mock.last_set_active_session(), Some(sid("s-3")));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Shift+Right on a work unit with no attached session (rule 3)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Shift+Right on an unattached work unit resumes the first open session
#[tokio::test(start_paused = true)]
async fn shift_right_on_an_unattached_work_unit_resumes_the_first_open_session() -> TestResult {
    // @step Given sessions s-1 and s-2 are open
    let (mut app, mock) = app_with(&["s-1", "s-2"], "AUTH-002", None).await?;
    // @step And session s-2 is focused with the draft "s2 text"
    assert_eq!(app.current_session(), Some(sid("s-2")));
    type_text(&mut app, "s2 text").await?;
    // @step And the user is on the board with an unattached work unit selected
    go_to_board(&mut app).await?;
    assert!(app.board_store().session_for("AUTH-002").is_none());
    // @step When the user presses Shift+Right
    shift(&mut app, KeyCode::Right).await?;
    // @step Then the agent view shows session s-1
    assert_agent_view_shows(&app, "s-1");
    // @step And the composer is empty
    assert_eq!(composer(&app), "");
    // @step And the in-memory draft of s-2 is "s2 text"
    assert_eq!(memory_draft(&app, "s-2")?, "s2 text");
    // @step And the daemon's active session is s-1
    assert_eq!(mock.last_set_active_session(), Some(sid("s-1")));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Board menu-bar chip (rule 4)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Activating a session chip on the board focuses that session through the focus path
#[tokio::test(start_paused = true)]
async fn activating_a_session_chip_on_the_board_focuses_that_session_through_the_focus_path(
) -> TestResult {
    // @step Given sessions s-1 and s-2 are open
    let (mut app, mock) = app_with(&["s-1", "s-2"], "AUTH-001", None).await?;
    // @step And session s-2 is focused
    assert_eq!(app.current_session(), Some(sid("s-2")));
    mock.seed_supervisors_for(sid("s-1"), vec![sid("sup-1")]);
    // @step And the user is on the board
    go_to_board(&mut app).await?;
    // @step When the user activates the s-1 chip in the board menu bar
    app.dispatch(Action::MenuChipActivate(0));
    drain(&mut app).await?;
    // @step Then the agent view shows session s-1
    assert_agent_view_shows(&app, "s-1");
    // @step And the daemon's active session is s-1
    assert_eq!(mock.last_set_active_session(), Some(sid("s-1")));
    // @step And the supervisors of s-1 are loaded
    assert_eq!(
        app.agent_view_store().supervisors_for(&sid("s-1")),
        [sid("sup-1")].as_slice()
    );
    Ok(())
}
