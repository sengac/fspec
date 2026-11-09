//! MENU-007 test support for `menu007_agent_board_view_item.rs` (split out
//! by REFAC-037 to keep the test file under 300 lines).
//!
//! Observation points: the agent bar row text (row 1: header row 0, bar
//! row 1), the item highlight (`buf[(x, y)].bg == Color::Cyan`), view flips
//! (`app.active_view()`), ring state (`app.navigator().agent.menu_focus()`)
//! and session switches (`app.current_session()`).

use crate::common::TaskOutcome;
use codelet_test_helpers::OrFail;
use std::sync::Arc;

pub use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use crate::common::MockBackend;

pub const AGENT_BAR_ROW: u16 = 1;

pub fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

pub async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
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

pub fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

pub fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
pub fn render_app(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

pub fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The x of the first occurrence of `needle` on `y` (None when absent).
/// `String::find` returns a BYTE index — the row contains multi-byte
/// glyphs (`│`, `●`), so convert to the CHAR (cell) index the buffer
/// addresses by.
pub fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

/// The x of the 'Board View' Zone A item on the agent bar row.
pub fn board_view_x(buf: &Buffer) -> Result<u16, codelet_test_helpers::TestError> {
    Ok(find_x(buf, AGENT_BAR_ROW, "Board View")
        .or_fail("the 'Board View' item must paint on the bar row")?)
}

/// The x of chip `n` (1-based) on the agent bar row.
pub fn chip_x(buf: &Buffer, n: usize) -> Result<u16, codelet_test_helpers::TestError> {
    Ok(
        find_x(buf, AGENT_BAR_ROW, &format!("#{n}")).or_fail(&format!(
            "chip #{n} must paint on the agent bar row:\n{}",
            row_text(buf, AGENT_BAR_ROW)
        ))?,
    )
}

/// The x of a Zone C button label ("New Agent" / "Close") on the
/// agent bar row (MENU-009).
pub fn zone_c_button_x(buf: &Buffer, label: &str) -> Result<u16, codelet_test_helpers::TestError> {
    Ok(find_x(buf, AGENT_BAR_ROW, label).or_fail(&format!(
        "the Zone C button '{label}' must paint on the agent bar row:\n{}",
        row_text(buf, AGENT_BAR_ROW)
    ))?)
}

/// Seed `n` open sessions (s-1..s-n, Idle by default) and focus s-1.
pub async fn seed_sessions(app: &mut App, n: usize) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    // `append_session` focuses the LAST appended session — reset to s-1.
    app.agent_view_store_mut().focus_session_index(0);
    drain_pending(app).await?;
    Ok(())
}

/// Flip the App into the Agent view on the given open session (the
/// RPC-097 jump-into-existing-session path).
pub async fn enter_agent_view(
    app: &mut App,
    session: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new(session.to_string()),
    )));
    drain_pending(app).await?;
    Ok(())
}

/// The agent view on s-1 with `n` open sessions.
pub async fn agent_view_with_sessions(n: usize) -> Result<App, codelet_test_helpers::TestError> {
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, n).await?;
    enter_agent_view(&mut app, "s-1").await?;
    Ok(app)
}

/// Press a bare key and let the App settle.
pub async fn press(app: &mut App, code: KeyCode) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&key(code, KeyModifiers::NONE));
    drain_pending(app).await?;
    Ok(())
}

/// The agent bar's ring focus is `expected`.
#[track_caller]
pub fn assert_focus(app: &App, expected: MenuFocus, why: &str) {
    assert_eq!(app.navigator().agent.menu_focus(), Some(expected), "{why}");
}
