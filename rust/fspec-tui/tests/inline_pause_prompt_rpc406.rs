// Feature: spec/features/inline-tool-approval-pause-prompt.feature
//! RPC-406 — inline tool-approval pause prompt acceptance tests.
//!
//! Covers the TS-parity inline prompt that replaces the RPC-053
//! PauseDialog modal: rendering (TestBackend buffer assertions), key
//! routing (Esc DENIES), chunk-driven store-slot lifecycle, draft
//! preservation, and the modal-removal source-shape lock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

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
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn triple_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        prompt: prompt.to_string(),
        tool_call_id: details.map(str::to_string),
    }
}

fn confirm_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Confirm,
        prompt: prompt.to_string(),
        tool_call_id: details.map(str::to_string),
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
) -> (Vec<String>, Buffer, Rect) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), store))
        .unwrap();
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    let input_area = view.last_input_area.expect("input area recorded by render");
    (rows, buf, input_area)
}

/// Terminal x coordinate of the first char of `needle` inside `row`.
fn char_x(row: &str, needle: &str) -> u16 {
    let pos = row
        .find(needle)
        .unwrap_or_else(|| panic!("`{needle}` not found in row: {row}"));
    row[..pos].chars().count() as u16
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

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
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
