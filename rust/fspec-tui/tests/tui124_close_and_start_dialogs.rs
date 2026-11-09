//! TUI-124 — agent and bot close, start and delete dialogs say exactly what
//! will happen.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature
//!
//! Harness: the real `App` dispatch over `MockBackend` (the BUG-549/552
//! pattern) on a paused tokio clock. Every backend reply is awaited through
//! the App's own pending tasks and action bus (`drain`), so nothing depends
//! on wall-clock time. Dialog texts are read from a full-App render.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

#[path = "tui124_close_and_start_dialogs/close_agent.rs"]
mod close_agent;
#[path = "tui124_close_and_start_dialogs/close_bot.rs"]
mod close_bot;
#[path = "tui124_close_and_start_dialogs/start_and_delete.rs"]
mod start_and_delete;

/// The compositor id of the close dialog (ExitConfirmationDialog).
pub(crate) const CLOSE_DIALOG_ID: &str = "exit-confirmation-dialog";

/// The agent bar row in a full-App render (the SessionHeader is row 0).
pub(crate) const AGENT_BAR_ROW: u16 = 1;

pub(crate) const WIDTH: u16 = 120;
pub(crate) const HEIGHT: u16 = 30;

pub(crate) fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// A fresh App over a fresh MockBackend, on the Agent view.
pub(crate) fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Run every pending task and every queued action until nothing moves.
pub(crate) async fn drain(app: &mut App) -> TestResult {
    for _ in 0..100 {
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
    Err(TestError::from("the App never settled"))
}

/// Open `sessions` (in order) and focus `focused` on the Agent view.
pub(crate) async fn open(app: &mut App, sessions: &[&str], focused: &str) -> TestResult {
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    drain(app).await?;
    app.dispatch(Action::OpenAgentView(Some(sid(focused))));
    drain(app).await
}

/// Mark `session` as a bot (the New Bot creation path's action).
pub(crate) async fn make_bot(app: &mut App, session: &str) -> TestResult {
    app.dispatch(Action::BotSessionCreated(sid(session)));
    drain(app).await
}

pub(crate) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

pub(crate) fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

pub(crate) async fn press(app: &mut App, code: KeyCode) -> TestResult {
    let _ = app.handle_event(&key(code));
    drain(app).await
}

pub(crate) fn render(app: &mut App) -> Buffer {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
}

pub(crate) fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol().to_string())
        .collect::<String>()
}

/// The whole render, one line per row.
pub(crate) fn screen(app: &mut App) -> String {
    let buffer = render(app);
    (0..buffer.area.height)
        .map(|y| row_text(&buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The cell of the first `needle` on the screen (column of its first char).
pub(crate) fn find(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
    (0..buffer.area.height).find_map(|y| {
        let row = row_text(buffer, y);
        row.find(needle)
            .map(|byte| (row[..byte].chars().count() as u16, y))
    })
}

/// Left-click the middle of the first painted `label`.
pub(crate) async fn click_label(app: &mut App, label: &str) -> TestResult {
    let buffer = render(app);
    let (x, y) = find(&buffer, label).or_fail(&format!("{label} is painted"))?;
    let mid = x + label.chars().count() as u16 / 2;
    let _ = app.handle_event(&click(mid, y));
    drain(app).await
}

/// Left-click the agent bar's "[ Close ]" button.
pub(crate) async fn activate_close(app: &mut App) -> TestResult {
    let buffer = render(app);
    let row = row_text(&buffer, AGENT_BAR_ROW);
    let byte = row
        .find("[ Close ]")
        .or_fail(&format!("the agent bar paints [ Close ]:\n{row}"))?;
    let x = row[..byte].chars().count() as u16 + 2;
    let _ = app.handle_event(&click(x, AGENT_BAR_ROW));
    drain(app).await
}

pub(crate) fn open_ids(app: &App) -> Vec<SessionId> {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .map(|context| context.id.clone())
        .collect()
}

pub(crate) fn assert_shows(text: &str, needle: &str) -> TestResult {
    if text.contains(needle) {
        Ok(())
    } else {
        Err(TestError::from(format!("missing {needle:?} in:\n{text}")))
    }
}
