// Feature: spec/features/inline-hitl-prompt.feature
//! RPC-411 — inline HITL prompt acceptance tests: rendering, selection
//! wrap, multi-question accumulation, Esc-cancel, slot lifecycle,
//! options-mode key/paste swallowing, freeform / Other mode (shared
//! composer input, empty-submit hint, draft consumption), and the
//! modal-removal source-shape lock.

use codelet_test_helpers::{OrFail, TestResult};

use crate::common::TaskOutcome;
use std::sync::Arc;

use codelet_fspec_tui::store::agent_view::hitl_state::HitlPromptState;
use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, SessionContext, ViewMode,
};
use codelet_rpc_types::{
    HitlAnswer, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId,
    SessionState, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::hitl_route;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn opt(label: &str, description: &str) -> HitlOption {
    HitlOption {
        label: label.to_string(),
        description: description.to_string(),
    }
}

fn question(id: &str, header: &str, text: &str, options: Vec<HitlOption>) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: text.to_string(),
        options,
    }
}

fn one_question_request(id: &str, header: &str, text: &str, opts: Vec<HitlOption>) -> HitlRequest {
    HitlRequest {
        questions: vec![question(id, header, text, opts)],
    }
}

fn yes_no_request(id: &str) -> HitlRequest {
    one_question_request(
        id,
        "Header",
        "Question?",
        vec![opt("Yes", "desc-Yes"), opt("No", "desc-No")],
    )
}

// ─────────────────────────────────────────────────────────────────────────
// View-level harness (rpc406 pattern)
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
// App-level harness (rpc406 pattern)
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

/// Paint one App frame so the AgentView caches the focused session's
/// HITL slot (`last_hitl`) for key routing.
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

/// Read the authoritative HITL slot (RPC-411 store/agent_view/hitl_state.rs).
fn hitl_slot<'a>(app: &'a App, session: &SessionId) -> Option<&'a HitlPromptState> {
    app.agent_view_store().hitl_prompt_for(session)
}

include!("inline_hitl_prompt_rpc411/rendering.rs");
include!("inline_hitl_prompt_rpc411/options_lifecycle.rs");
include!("inline_hitl_prompt_rpc411/options_and_source.rs");
include!("inline_hitl_prompt_rpc411/other_mode.rs");
include!("inline_hitl_prompt_rpc411/freeform_draft_paste.rs");
