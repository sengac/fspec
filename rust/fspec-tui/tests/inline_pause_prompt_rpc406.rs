// Feature: spec/features/inline-tool-approval-pause-prompt.feature
//! RPC-406 — inline tool-approval pause prompt acceptance tests.
//!
//! Covers the TS-parity inline prompt that replaces the RPC-053
//! PauseDialog modal: rendering (TestBackend buffer assertions), key
//! routing (Esc DENIES), chunk-driven store-slot lifecycle, draft
//! preservation, and the modal-removal source-shape lock.

use codelet_test_helpers::{OrFail, TestResult};

use crate::common::TaskOutcome;
use std::sync::Arc;

use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, SessionContext, ViewMode,
};
use codelet_rpc_types::{
    ApprovalChoice, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId,
    SessionState, SessionStatus, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::pause_route;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn triple_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        prompt: prompt.to_string(),
        details: details.map(str::to_string),
        tool_call_id: None,
    }
}

fn confirm_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Confirm,
        prompt: prompt.to_string(),
        details: details.map(str::to_string),
        tool_call_id: None,
    }
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// View-level harness (rpc404/rpc405 pattern)
// ─────────────────────────────────────────────────────────────────────────

fn view_harness(sessions: &[&str]) -> (AgentView, AgentViewStore) {
    let (tx, _rx) = unbounded_channel();
    let mut store = AgentViewStore::default();
    for s in sessions {
        store.append_session(SessionContext::new(sid(s)));
    }
    store.focus_session_index(0);
    (AgentView::new(tx), store)
}

/// Render one frame on a w x h TestBackend; return every terminal row
/// as a String, the raw buffer, and the input-area Rect.
fn render_frame(
    w: u16,
    h: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> Result<(Vec<String>, Buffer, Rect), codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(w, h)).or_fail("expected a value")?;
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), store))
        .or_fail("expected a value")?;
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    let input_area = view
        .last_input_area
        .or_fail("input area recorded by render")?;
    Ok((rows, buf, input_area))
}

/// Terminal x coordinate of the first char of `needle` inside `row`.
fn char_x(row: &str, needle: &str) -> Result<u16, codelet_test_helpers::TestError> {
    let pos = row
        .find(needle)
        .or_fail(&format!("`{needle}` not found in row: {row}"))?;
    Ok(row[..pos].chars().count() as u16)
}

// ─────────────────────────────────────────────────────────────────────────
// App-level harness (rpc053 pattern + rendered frame to seed key cache)
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app(sessions: &[&str]) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for s in sessions {
        app.dispatch(Action::SessionCreated(sid(s)));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Paint one App frame into a scratch buffer so the AgentView caches
/// the focused session's pause slot for key routing.
fn render_app(app: &mut App) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
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

async fn wait_until<F: FnMut() -> bool>(
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

// RPC-410: multi-question wire shape (single-question fixture).
fn hitl_request(id: &str, labels: &[&str]) -> HitlRequest {
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

fn pause_dialog_layer_mounted(app: &App) -> bool {
    app.compositor()
        .layer_ids()
        .iter()
        .any(|id| id == "pause-dialog")
}

include!("inline_pause_prompt_rpc406/rendering.rs");
include!("inline_pause_prompt_rpc406/keys.rs");
include!("inline_pause_prompt_rpc406/lifecycle.rs");
include!("inline_pause_prompt_rpc406/draft_and_source.rs");
