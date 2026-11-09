//! BUG-179 test support for `mux_agent_input_paste_bug179.rs` (split out
//! by REFAC-036 to keep the test file under 300 lines): the App fixture,
//! the mux entry, click-to-focus and the one-frame render.

use crate::common::TaskOutcome;
use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::MockBackend;

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

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

pub fn submit(app: &mut App, text: &str) {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, text.to_string()));
    }
}

/// Open two sessions (s-1, s-2) and enter mux with `/mux board, agent,
/// agent` (the feature's shared Given).
///
/// Natural post-`/mux` state: mux focus sits on pane 1 (the first agent
/// pane, window session s-1) while the store's current session is s-2
/// (last created) — a genuine focus/store divergence. A click inside a
/// pane converges it (the production click-to-focus path + the
/// post-Navigator BUG-163 sync in `App::handle_event`).
pub async fn enter_mux_board_agent_agent(
    app: &mut App,
) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=2 {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await?;
    submit(app, "/mux board, agent, agent");
    drain_pending(app).await?;
    assert_eq!(app.active_view(), ViewMode::Mux, "/mux must enter the grid");
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config flag must be ON inside the grid"
    );
    Ok(())
}

/// Focus the Nth (0-based) rendered pane by clicking inside its cached
/// rect — the production click-to-focus path (Navigator → mux_mouse).
/// The click also runs the post-Navigator BUG-163 sync, converging the
/// store's current session onto the clicked agent pane's window session.
pub fn click_pane(app: &mut App, pane_index: usize) -> Result<(), codelet_test_helpers::TestError> {
    let rect = app
        .navigator()
        .mux
        .pane_rects()
        .get(pane_index)
        .copied()
        .or_fail("pane rect must exist")?;
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 1,
        row: rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let _ = app.handle_event(&event);
    Ok(())
}

/// Render the App once into an off-screen buffer so per-frame caches
/// (`AgentView::last_is_compacting` — the RPC-095 gate input) refresh.
pub fn render_once(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}
