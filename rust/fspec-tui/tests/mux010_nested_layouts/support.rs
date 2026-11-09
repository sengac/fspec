//! MUX-010 test fixture: an `App` over a `MockBackend`, slash submits,
//! sized renders and the focused session's scrollback text.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestError};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::common::{MockBackend, TaskOutcome};

pub const SESSION: &str = "s-1";

/// A fresh app with one open session `s-1` (slash commands and their
/// notices need a current session).
pub async fn app_with_session() -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new(SESSION)));
    drain(&mut app).await?;
    Ok((app, mock))
}

/// Run every pending task and queued action until nothing moves.
pub async fn drain(app: &mut App) -> Result<(), TestError> {
    loop {
        let mut moved = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            moved = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            moved = true;
        }
        if !moved {
            return Ok(());
        }
    }
}

/// Submit `text` as the current session's input.
pub async fn submit(app: &mut App, text: &str) -> Result<(), TestError> {
    let session = app.current_session().or_fail("a current session")?;
    app.dispatch(Action::InputSubmitted(session, text.to_string()));
    drain(app).await
}

/// Draw one frame of `width` x `height`; returns the rows as text.
pub fn render(app: &mut App, width: u16, height: u16) -> Result<Vec<String>, TestError> {
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = term.backend().buffer();
    Ok((0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
        .collect())
}

/// Turn mux on at `width` x `height` with `layout` (rendered once first
/// so the fit check sees the real body).
pub async fn mux_with(
    app: &mut App,
    layout: &str,
    width: u16,
    height: u16,
) -> Result<(), TestError> {
    render(app, width, height)?;
    submit(app, &format!("/mux {layout}")).await?;
    render(app, width, height)?;
    if app.navigator().active_view != ViewMode::Mux {
        return Err(TestError::from(format!("/mux {layout} did not enter mux")));
    }
    Ok(())
}

pub fn pane_rects(app: &App) -> Vec<Rect> {
    app.navigator().mux.pane_rects().to_vec()
}

pub fn layout_string(app: &App) -> String {
    app.navigator().mux.config().layout.to_string()
}

/// True when the scrollback holds `needle`, ignoring where the
/// scrollback wrapped its lines.
pub fn scrollback_has(app: &App, needle: &str) -> bool {
    let flat = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    flat(&scrollback(app)).contains(&flat(needle))
}

/// The scrollback text of session `s-1`.
pub fn scrollback(app: &App) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(&SessionId::new(SESSION))
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

pub fn shift(code: KeyCode) -> Event {
    key(code, KeyModifiers::SHIFT)
}

pub fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

pub fn left(kind: fn(MouseButton) -> MouseEventKind) -> MouseEventKind {
    kind(MouseButton::Left)
}

/// Press `event` through the App and run what it started.
pub async fn press(app: &mut App, event: Event) -> Result<(), TestError> {
    let _ = app.handle_event(&event);
    drain(app).await
}

/// Type `text` into whatever has the keyboard.
pub async fn type_text(app: &mut App, text: &str) -> Result<(), TestError> {
    for c in text.chars() {
        press(app, key(KeyCode::Char(c), KeyModifiers::NONE)).await?;
    }
    Ok(())
}
