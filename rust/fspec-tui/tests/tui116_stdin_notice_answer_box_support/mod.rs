//! TUI-116 test support: App + MockBackend harness and screen helpers for
//! `tui116_stdin_notice_answer_box.rs` (kept apart so the test file stays
//! under 300 lines). Every helper returns its failure instead of panicking.

use crate::common::TaskOutcome;
use std::error::Error;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{ExecStdinControl, ExecStdinRequest, SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::MockBackend;

pub type TestResult = Result<(), Box<dyn Error>>;
pub type Outcome<T> = Result<T, Box<dyn Error>>;

pub const NOTICE: &str = "git commit may be waiting for input · Ctrl+O answer · Ctrl+X dismiss";
pub const BOX_HINT: &str = "Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input";

pub fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

pub fn exec_request(exec_session: &str, command: &str) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: exec_session.to_string(),
        command: command.to_string(),
        quiet_seconds: 5,
        ts_ms: 1_700_000_000_000,
    }
}

/// An agent-view App with one created session per name (the last one is
/// focused) on a recording MockBackend.
pub fn app_with(sessions: &[&str]) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Session `s-1` is Running and `command` in `exec_session` may be waiting
/// for input; one frame is painted so the view caches the notice.
pub async fn running_with_request(
    exec_session: &str,
    command: &str,
) -> Outcome<(App, Arc<MockBackend>, SessionId)> {
    let (mut app, mock) = app_with(&["s-1"]);
    let session = sid("s-1");
    app.agent_view_store_mut()
        .set_session_status(session.clone(), SessionStatus::Running);
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request(exec_session, command),
        },
    ));
    drain(&mut app).await?;
    screen(&mut app)?;
    Ok((app, mock, session))
}

/// Run every pending task and queued action until the app is quiet.
pub async fn drain(app: &mut App) -> TestResult {
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

/// Paint a frame (as the event loop does between events), send the key,
/// then let the app settle.
pub async fn key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    screen(app)?;
    app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

pub async fn ctrl(app: &mut App, character: char) -> TestResult {
    key(app, KeyCode::Char(character), KeyModifiers::CONTROL).await
}

pub async fn type_text(app: &mut App, text: &str) -> TestResult {
    for character in text.chars() {
        key(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

/// The text of each row of `area` in `buffer`.
pub fn rows_of(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.y.saturating_add(area.height))
        .map(|y| {
            (area.x..area.x.saturating_add(area.width))
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

/// Render one 160x30 frame and return its rows.
pub fn screen(app: &mut App) -> Outcome<Vec<String>> {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    Ok(rows_of(&buffer, area))
}

/// Index of the first row containing `needle`.
pub fn row_of(rows: &[String], needle: &str) -> Outcome<usize> {
    rows.iter()
        .position(|row| row.contains(needle))
        .ok_or_else(|| format!("no row contains {needle:?} in:\n{}", rows.join("\n")).into())
}

pub fn shows(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|row| row.contains(needle))
}

/// Fail with the screen when `rows` does not show `needle`.
pub fn expect_shown(rows: &[String], needle: &str) -> TestResult {
    row_of(rows, needle).map(|_| ())
}

/// Fail with the screen when `rows` shows `needle`.
pub fn expect_hidden(rows: &[String], needle: &str) -> TestResult {
    if shows(rows, needle) {
        return Err(format!("{needle:?} is still shown in:\n{}", rows.join("\n")).into());
    }
    Ok(())
}

pub fn chat_draft(app: &App) -> String {
    app.navigator().agent.input.value()
}

pub fn exec_controls(mock: &MockBackend) -> Outcome<Vec<(SessionId, String, ExecStdinControl)>> {
    Ok(mock.exec_control().calls()?)
}

/// Wait for `predicate` under the deadlock guard (TEST-042), failing with
/// `label` when the guard expires.
pub async fn wait_for<F: FnMut() -> bool>(mut predicate: F, label: &str) -> TestResult {
    codelet_test_helpers::wait_until(label, || std::future::ready(predicate())).await?;
    Ok(())
}
