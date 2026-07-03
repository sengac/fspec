//! RPC-053 — Pause / HITL chunk-driven trigger (rewritten by RPC-411).
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! RPC-406 superseded the PauseDialog modal with the inline pause prompt
//! (see tests/inline_pause_prompt_rpc406.rs). RPC-411 superseded the
//! HitlDialog modal with the inline HITL prompt (see
//! tests/inline_hitl_prompt_rpc411.rs). This file retains the
//! chunk-driven Paused trigger (parallel get_pause_state +
//! get_hitl_request, HITL wins on tie, results land in per-session
//! store slots), the Esc-cancel wire contract, and error tolerance.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId, SessionState,
    StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn pause_state(kind: PauseKind, prompt: &str) -> PauseState {
    PauseState {
        kind,
        prompt: prompt.to_string(),
        tool_call_id: None,
    }
}

// RPC-410 wire shape: multi-question request, freeform derivable.
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

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
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

fn fresh_app_with_session(session: &str) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid(session)));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Seed the per-session HITL slot (RPC-411) and paint one frame so
/// the AgentView caches the prompt for key routing.
fn seed_hitl_slot(app: &mut App, session: &SessionId, request: HitlRequest) {
    app.dispatch(Action::HitlPromptFetched {
        session_id: session.clone(),
        request,
    });
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

fn hitl_slot_set(app: &App, session: &SessionId) -> bool {
    app.agent_view_store().hitl_prompt_for(session).is_some()
}

// ─────────────────────────────────────────────────────────────────────────
// Chunk-driven trigger: SessionStateChange { Paused } sets the right slot
// ─────────────────────────────────────────────────────────────────────────

// Scenario: SessionStateChange{Paused} with hitl_request Some stores a HITL slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_hitl_request_some_stores_a_hitl_slot() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Continue?", header: "Apply changes?", options: [Yes, No] } for s-1
    let request = hitl_request("q-1", &["Yes", "No"]);
    mock.script_hitl_request(sid("s-1"), Some(request.clone()));

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore HITL slot for s-1 holds the fetched request
    wait_until(|| hitl_slot_set(&app, &sid("s-1")), "HITL slot set").await;
    assert_eq!(
        app.agent_view_store()
            .hitl_prompt_for(&sid("s-1"))
            .map(|s| s.request.clone()),
        Some(request)
    );
    // @step And no modal layer is mounted on the Compositor
    assert_eq!(
        app.compositor().len(),
        0,
        "no modal may mount for a HITL request; ids: {:?}",
        app.compositor().layer_ids()
    );
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
}

// Scenario: SessionStateChange{Paused} with both pause_state Some and hitl_request Some — HITL wins
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_both_some_hitl_wins() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    mock.script_pause_state(sid("s-1"), Some(pause_state(PauseKind::Confirm, "p")));
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Q", header: "H", options: [Yes, No] } for s-1
    let request = hitl_request("q-1", &["Yes", "No"]);
    mock.script_hitl_request(sid("s-1"), Some(request.clone()));

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore HITL slot for s-1 holds the fetched request
    wait_until(|| hitl_slot_set(&app, &sid("s-1")), "HITL wins on tie").await;
    assert_eq!(
        app.agent_view_store()
            .hitl_prompt_for(&sid("s-1"))
            .map(|s| s.request.clone()),
        Some(request)
    );
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
}

// Scenario: SessionStateChange{Paused} with both returning None sets no slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_both_none_sets_no_slot() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
}

// Scenario: SessionStateChange{Idle} clears the HITL slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_idle_clears_the_hitl_slot() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And session s-1 has an active HITL slot
    seed_hitl_slot(&mut app, &sid("s-1"), hitl_request("q-1", &["Yes", "No"]));
    assert!(hitl_slot_set(&app, &sid("s-1")));

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Idle }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Idle,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
    // @step And backend.send_hitl_response is NEVER called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "server-side clear must not double-send a response"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Esc-cancel wire correctness (the stranding-bug fix)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Esc on the inline HITL prompt cancels through the backend
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_the_inline_hitl_prompt_cancels_through_the_backend() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And session s-1 has an active HITL slot
    seed_hitl_slot(&mut app, &sid("s-1"), hitl_request("q-1", &["Yes", "No"]));

    // @step When the user presses Esc on the inline HITL prompt
    app.handle_event(&key_event(KeyCode::Esc));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ cancelled: true, answers: [] })
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].0, sid("s-1"));
    assert!(log[0].1.cancelled, "Esc must SEND cancelled:true");
    assert!(log[0].1.answers.is_empty());
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
}

// ─────────────────────────────────────────────────────────────────────────
// Error tolerance
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Backend.get_hitl_request error during the chunk-driven probe sets no slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_hitl_request_error_sets_no_slot() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return Err("decode failed") for s-1
    mock.set_get_hitl_request_error("decode failed".to_string());

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
    // @step And the session s-1 scrollback contains no chunks mentioning "get_hitl_request"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session present");
    for ch in &ctx.scrollback.visible_window(1024) {
        for line in &ch.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("get_hitl_request"),
                "no error notice in scrollback (saw: {text})"
            );
        }
    }
}

// Scenario: Backend.send_hitl_response error on Esc-cancel still clears the slot and emits no scrollback notice
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_hitl_response_error_still_clears_slot_and_emits_no_notice() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    mock.set_send_hitl_response_error("decode failed".to_string());
    // @step And session s-1 has an active HITL slot
    seed_hitl_slot(&mut app, &sid("s-1"), hitl_request("q-1", &["Yes"]));

    // @step When the user presses Esc on the inline HITL prompt
    app.handle_event(&key_event(KeyCode::Esc));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
    // @step And the session s-1 scrollback contains no chunks mentioning "send_hitl_response"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session present");
    for ch in &ctx.scrollback.visible_window(1024) {
        for line in &ch.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("send_hitl_response"),
                "no error notice in scrollback (saw: {text})"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape
// ─────────────────────────────────────────────────────────────────────────

// Scenario: codelet/fspec-tui/src/app/dispatch_pause_hitl.rs hosts the new pause/HITL helpers
#[test]
fn dispatch_pause_hitl_hosts_the_pause_hitl_helpers() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // @step Given the file codelet/fspec-tui/src/app/dispatch_pause_hitl.rs exists
    let dph_path = src.join("app").join("dispatch_pause_hitl.rs");
    assert!(dph_path.exists(), "dispatch_pause_hitl.rs must exist");

    // @step When the file is compiled as part of codelet-fspec-tui
    let dph = std::fs::read_to_string(&dph_path).expect("read dispatch_pause_hitl.rs");

    // @step Then it must declare impl App methods named handle_pause_chunk, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    for method in [
        "fn handle_pause_chunk",
        "fn handle_pause_confirmed",
        "fn handle_pause_triple",
        "fn handle_pause_resumed",
        "fn handle_hitl_submitted",
        "fn handle_pause_cleared",
    ] {
        assert!(dph.contains(method), "missing {method}");
    }

    // @step And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_pause_hitl and pub mod dispatch_hitl_prompt
    let app_mod = std::fs::read_to_string(src.join("app").join("mod.rs")).expect("read app/mod.rs");
    assert!(app_mod.contains("pub mod dispatch_pause_hitl"));
    assert!(app_mod.contains("pub mod dispatch_hitl_prompt"));

    // @step And codelet/fspec-tui/src/components/mod.rs must declare no pub mod hitl_dialog and no pub mod pause_dialog
    let components = std::fs::read_to_string(src.join("components").join("mod.rs"))
        .expect("read components/mod.rs");
    assert!(!components.contains("pub mod hitl_dialog"));
    assert!(!components.contains("pub mod pause_dialog"));

    // @step And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
    let dispatch =
        std::fs::read_to_string(src.join("app").join("dispatch.rs")).expect("read dispatch.rs");
    assert!(
        dispatch.lines().count() <= 300,
        "app/dispatch.rs exceeds 300 lines"
    );
}
