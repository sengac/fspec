//! Helpers for `exec_stdin_push_chunk_bug171.rs` (REFAC-043): the
//! BUG-171 push-trigger harness (fresh_app + MockBackend +
//! render_app_frame + drain_pending).

use crate::common::TaskOutcome;
use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{ExecStdinRequest, SessionId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::MockBackend;

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

pub fn exec_request(exec_id: &str, command: &str, quiet_seconds: i64) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: exec_id.to_string(),
        command: command.to_string(),
        quiet_seconds,
        ts_ms: 1_700_000_000_000,
    }
}

pub fn key_event(code: crossterm::event::KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(crossterm::event::KeyEvent {
        code,
        modifiers: crossterm::event::KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

pub fn fresh_app(sessions: &[&str]) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for s in sessions {
        app.dispatch(Action::SessionCreated(sid(s)));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

pub fn render_app_frame(app: &mut App) -> Vec<String> {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
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

pub async fn wait_until<F: FnMut() -> bool>(
    mut predicate: F,
    label: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    codelet_test_helpers::deadlock_guard(label, async {
        loop {
            if predicate() {
                return;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await
    .or_fail(&format!("timed out waiting for: {label}"))?;
    Ok(())
}

pub fn exec_stdin_slot<'a>(app: &'a App, session: &SessionId) -> Option<&'a ExecStdinRequest> {
    app.agent_view_store().exec_stdin_for(session)
}
