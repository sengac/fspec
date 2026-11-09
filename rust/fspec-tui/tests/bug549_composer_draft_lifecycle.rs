// Feature: spec/features/composer-draft-belongs-to-one-session.feature
//! BUG-549 — a composer draft belongs to one session and ends when it is
//! submitted.
//!
//! Drives the real `App` (handle_event + dispatch) against the recording
//! `MockBackend`, whose in-memory pending-input store stands in for the
//! daemon copy of each draft. Every test runs on a paused tokio clock: the
//! 300ms draft save only fires when a step moves the clock past it
//! (`pass_save_window`), so "before the save fires" is exact, with no real
//! sleeps.

#![allow(clippy::too_many_lines)]

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::hitl_route;
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

/// Run every parked task and queued action until the App is quiet. Parked
/// tasks that sleep would let the paused clock run, but none of the paths
/// these tests drive park a sleeping task.
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

/// Move the paused clock past the draft-save debounce, let the woken save
/// tasks run, then settle the App. Twice, so a save scheduled while
/// settling also lands.
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

/// Paint one frame, as the run loop does between events (the view caches
/// the running state, prompts and mux pane rects at render time).
fn render(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
}

/// Render, deliver one key, settle (no clock movement).
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

/// Focus the first open session through the Shift+Left cycle.
async fn focus_previous(app: &mut App) -> TestResult {
    shift(app, KeyCode::Left).await
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

/// The daemon copy (the mock's pending-input store); an absent and an
/// empty draft are both empty.
async fn daemon_draft(mock: &MockBackend, session: &str) -> Result<String, TestError> {
    let stored = FspecBackend::get_pending_input(mock, sid(session))
        .await
        .or_fail("read the daemon draft")?;
    Ok(stored.unwrap_or_default())
}

/// The composer, the in-memory draft and the daemon draft of `session`.
async fn all_copies(
    app: &App,
    mock: &MockBackend,
    session: &str,
) -> Result<[String; 3], TestError> {
    Ok([
        composer(app),
        memory_draft(app, session)?,
        daemon_draft(mock, session).await?,
    ])
}

fn empty_copies() -> [String; 3] {
    [String::new(), String::new(), String::new()]
}

/// True when any draft save carried `text`.
fn saved(mock: &MockBackend, text: &str) -> bool {
    mock.pending_input_writes()
        .iter()
        .any(|(_, written)| written.as_deref() == Some(text))
}

/// MUX-010: the layout string of a space-separated pane list
/// (`board agent agent` -> `board, agent, agent`).
fn layout_of(panes: &str) -> String {
    panes.split_whitespace().collect::<Vec<_>>().join(", ")
}

/// Enter mux mode with `/mux <panes>` (submitted like a typed command).
async fn enter_mux(app: &mut App, panes: &str) -> TestResult {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(
            session,
            format!("/mux {}", layout_of(panes)),
        ));
    }
    drain(app).await?;
    render(app);
    Ok(())
}

/// Click inside the Nth rendered mux pane (production click-to-focus).
async fn click_pane(app: &mut App, pane_index: usize) -> TestResult {
    render(app);
    let rect = app
        .navigator()
        .mux
        .pane_rects()
        .get(pane_index)
        .copied()
        .or_fail("the mux pane is rendered")?;
    let _ = app.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 1,
        row: rect.y + 1,
        modifiers: KeyModifiers::NONE,
    }));
    drain(app).await
}

include!("bug549_composer_draft_lifecycle/submit.rs");
include!("bug549_composer_draft_lifecycle/focus_routes.rs");
include!("bug549_composer_draft_lifecycle/ordering_hydration.rs");
include!("bug549_composer_draft_lifecycle/code_driven.rs");
include!("bug549_composer_draft_lifecycle/clear_keep.rs");
