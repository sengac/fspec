//! Helpers for tests/pause_hitl_rpc053.rs (REFAC-050).

use crate::common::TaskOutcome;
use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId, SessionState,
    StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::{hitl_route, MockBackend};

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

pub fn pause_state(kind: PauseKind, prompt: &str) -> PauseState {
    PauseState {
        kind,
        prompt: prompt.to_string(),
        details: None,
        tool_call_id: None,
    }
}

// RPC-410 wire shape: multi-question request, freeform derivable.
pub fn hitl_request(id: &str, labels: &[&str]) -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: id.to_string(),
            header: "Apply the proposed change?".to_string(),
            question: "Continue?".to_string(),
            options: labels
                .iter()
                .map(|l| HitlOption {
                    label: (*l).to_string(),
                    description: format!("desc-{l}"),
                })
                .collect(),
        }],
    }
}

pub fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
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

pub fn fresh_app_with_session(session: &str) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid(session)));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Seed the per-session HITL slot (RPC-411) and paint one frame so
/// the AgentView caches the prompt for key routing.
pub fn seed_hitl_slot(app: &mut App, session: &SessionId, request: HitlRequest) {
    app.dispatch(Action::HitlPromptFetched {
        session_id: session.clone(),
        route: hitl_route(session),
        request,
    });
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

pub fn hitl_slot_set(app: &App, session: &SessionId) -> bool {
    app.agent_view_store().hitl_prompt_for(session).is_some()
}

// ─────────────────────────────────────────────────────────────────────────

/// Dispatch `ChunkReceived(session, SessionStateChange { state })`.
pub fn dispatch_state(app: &mut App, session: &str, state: SessionState) {
    app.dispatch(Action::ChunkReceived(
        sid(session),
        StreamChunk::SessionStateChange { state },
    ));
}

pub fn pause_slot_empty(app: &App, session: &str) -> bool {
    app.agent_view_store()
        .pause_state_for(&sid(session))
        .is_none()
}

pub fn hitl_slot_request(app: &App, session: &str) -> Option<HitlRequest> {
    app.agent_view_store()
        .hitl_prompt_for(&sid(session))
        .map(|slot| slot.request.clone())
}
