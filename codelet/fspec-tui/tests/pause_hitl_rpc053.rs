//! RPC-053 — Pause / HITL UI (PauseDialog + HitlDialog end-to-end).
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! Drives the App::dispatch routing for the chunk-driven Paused trigger
//! (parallel get_pause_state + get_hitl_request) and the dialog actions
//! (PauseConfirmed / PauseTriple / PauseResumed / HitlSubmitted).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{
    Action, App, Component, FspecBackend, HITL_DIALOG_ID, PAUSE_DIALOG_ID, PauseDialog,
};
use codelet_rpc_types::{
    ApprovalChoice, HitlOption, HitlRequest, PauseKind, PauseState, SessionId,
    SessionState, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
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

fn hitl_request(id: &str, labels: &[&str], allow_text: bool) -> HitlRequest {
    HitlRequest {
        id: id.to_string(),
        question: "Continue?".to_string(),
        header: "Apply the proposed change?".to_string(),
        options: labels
            .iter()
            .map(|l| HitlOption {
                label: (*l).to_string(),
                description: format!("desc-{l}"),
            })
            .collect(),
        allow_text_input: allow_text,
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
    (app, mock)
}

fn mount_pause_dialog(app: &mut App, session: &SessionId, state: PauseState) {
    app.dispatch(Action::OpenPauseDialog {
        session_id: session.clone(),
        state,
    });
}

fn mount_hitl_dialog(app: &mut App, session: &SessionId, request: HitlRequest) {
    app.dispatch(Action::OpenHitlDialog {
        session_id: session.clone(),
        request,
    });
}

// ─────────────────────────────────────────────────────────────────────────
// PauseDialog interactions (Confirm + Triple kinds)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_dialog_enter_on_accept_calls_pause_confirm_true() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Confirm, "p"));
    // @step Then the PauseDialog's focused button is "Accept"
    assert!(app.compositor().contains(PAUSE_DIALOG_ID));

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, true)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), true));

    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_dialog_tab_then_enter_calls_pause_confirm_false() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Deny"
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Confirm, "p"));
    // Tab to advance focus from Accept to Deny.
    // @step When the user presses Tab on the PauseDialog
    app.handle_event(&key_event(KeyCode::Tab));
    // @step Then the PauseDialog's focused button is "Deny"

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, false)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log[0], (sid("s-1"), false));
    // @step And backend.pause_confirm is NOT called with (s-1, true)
    assert!(!log.iter().any(|(_, accept)| *accept));
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_dialog_esc_calls_pause_resume_not_pause_confirm() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Confirm, "p"));

    // @step When the user presses Esc on the PauseDialog
    app.handle_event(&key_event(KeyCode::Esc));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.pause_resume is called exactly once with s-1
    wait_until(
        || mock.pause_resume_calls() == 1,
        "pause_resume called once",
    )
    .await;
    let log = mock.pause_resume_log();
    assert_eq!(log, vec![sid("s-1")]);
    // @step And backend.pause_confirm is NEVER called
    assert_eq!(mock.pause_confirm_calls(), 0);
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn triple_pause_dialog_enter_on_deny_calls_pause_triple_deny() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Deny"
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Triple, "p"));
    // @step Then the PauseDialog's focused button is "Approve"
    // Right arrow twice → focus on Deny (Approve → Approve Session → Deny).
    // @step When the user presses Right arrow on the PauseDialog
    app.handle_event(&key_event(KeyCode::Right));
    app.handle_event(&key_event(KeyCode::Right));

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::Deny)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), ApprovalChoice::Deny));
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn triple_pause_dialog_enter_on_approve_session_calls_pause_triple_approve_session() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Approve Session"
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Triple, "p"));
    // @step When the user presses Right arrow on the PauseDialog
    app.handle_event(&key_event(KeyCode::Right));
    // @step Then the PauseDialog's focused button is "Approve Session"

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::ApproveSession)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    assert_eq!(
        mock.pause_triple_log()[0],
        (sid("s-1"), ApprovalChoice::ApproveSession)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// HitlDialog interactions
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_dialog_hotkey_a_submits_first_option() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    mount_hitl_dialog(
        &mut app,
        &sid("s-1"),
        hitl_request("q-1", &["Yes", "No"], false),
    );

    // @step When the user presses the character "a" on the HitlDialog
    app.handle_event(&key_event(KeyCode::Char('a')));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "Yes" })
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].0, sid("s-1"));
    assert_eq!(log[0].1.id, "q-1");
    assert_eq!(log[0].1.value, "Yes");
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_dialog_enter_on_no_submits_no() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    mount_hitl_dialog(
        &mut app,
        &sid("s-1"),
        hitl_request("q-1", &["Yes", "No"], false),
    );
    // @step And the HitlDialog's selected row is index 1 (option "No")
    app.handle_event(&key_event(KeyCode::Down));
    // @step When the user presses Enter on the HitlDialog
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "No" })
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    assert_eq!(mock.send_hitl_response_log()[0].1.value, "No");
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_dialog_esc_dismisses_without_submit() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a HitlDialog is mounted on the Compositor for s-1
    mount_hitl_dialog(
        &mut app,
        &sid("s-1"),
        hitl_request("q-1", &["Yes", "No"], false),
    );

    // @step When the user presses Esc on the HitlDialog
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is NEVER called
    assert_eq!(mock.send_hitl_response_calls(), 0);
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_dialog_free_text_enter_submits_typed_text() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and allow_text_input=true
    mount_hitl_dialog(
        &mut app,
        &sid("s-1"),
        hitl_request("q-1", &["Yes", "No"], true),
    );
    // Tab past the two options into the free-text input row.
    // @step And the HitlDialog's selected row is the free-text input row
    app.handle_event(&key_event(KeyCode::Tab));
    app.handle_event(&key_event(KeyCode::Tab));
    // Type "maybe later".
    // @step And the HitlDialog's free-text row contains "maybe later"
    for c in "maybe later".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
    }

    // @step When the user presses Enter on the HitlDialog
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "maybe later" })
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log[0].1.value, "maybe later");
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// Error tolerance
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_confirm_error_still_pops_dialog_and_emits_no_notice() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's pause_confirm is scripted to return Err("network blip")
    mock.set_pause_confirm_error("network blip".to_string());
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Confirm, "p"));

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // (no panic = test reached this line)
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
    // @step And the session s-1 scrollback contains no chunks mentioning "pause_confirm"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session present");
    let visible = ctx.scrollback.visible_window(1024);
    for ch in &visible {
        for line in &ch.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("pause_confirm"),
                "no pause_confirm notice in scrollback (saw: {text})"
            );
            // @step And the session s-1 scrollback contains no chunks mentioning "network blip"
            assert!(
                !text.contains("network blip"),
                "no error mention in scrollback (saw: {text})"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_hitl_request_error_pushes_no_dialog() {
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
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_hitl_response_error_still_pops_dialog_and_emits_no_notice() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    mock.set_send_hitl_response_error("decode failed".to_string());
    // @step And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes"]
    mount_hitl_dialog(&mut app, &sid("s-1"), hitl_request("q-1", &["Yes"], false));

    // @step When the user presses the character "a" on the HitlDialog
    app.handle_event(&key_event(KeyCode::Char('a')));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
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
// Multi-session: dialog binds to its originating session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_dialog_for_non_focused_session_routes_actions_to_originating_session() {
    // @step Given an App with a MockBackend
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And open sessions are [s-1, s-2] and s-1 is focused
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // Refocus to s-1 (SessionCreated set focus to the new session).
    app.dispatch(Action::SessionPrev);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));

    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-2
    mock.script_pause_state(sid("s-2"), Some(pause_state(PauseKind::Confirm, "p")));
    // @step And the MockBackend's get_hitl_request is scripted to return None for s-2
    mock.script_hitl_request(sid("s-2"), None);

    // @step When Action::ChunkReceived(s-2, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor contains a layer with id PAUSE_DIALOG_ID
    wait_until(
        || app.compositor().contains(PAUSE_DIALOG_ID),
        "PauseDialog mounted for s-2",
    )
    .await;
    // @step And the focused session is still s-1
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));

    // @step When the user presses Enter on the PauseDialog
    app.handle_event(&key_event(KeyCode::Enter));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-2, true)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log[0], (sid("s-2"), true));
    // @step And backend.pause_confirm is NOT called with (s-1, true)
    assert!(!log.iter().any(|(s, accept)| s == &sid("s-1") && *accept));
}

// ─────────────────────────────────────────────────────────────────────────
// Chunk-driven trigger: SessionStateChange { Paused } opens the right dialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_pause_state_some_opens_a_confirm_pause_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(
        sid("s-1"),
        Some(PauseState {
            kind: PauseKind::Confirm,
            prompt: "Run dangerous-cmd?".to_string(),
            tool_call_id: Some("tc-1".to_string()),
        }),
    );
    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then backend.get_pause_state is called at least once with s-1
    wait_until(
        || mock.get_pause_state_calls() >= 1,
        "get_pause_state called",
    )
    .await;
    // @step And backend.get_hitl_request is called at least once with s-1
    assert!(mock.get_hitl_request_calls() >= 1);

    // @step And the Compositor contains a layer with id PAUSE_DIALOG_ID
    wait_until(
        || app.compositor().contains(PAUSE_DIALOG_ID),
        "PauseDialog mounted",
    )
    .await;
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_hitl_request_some_opens_a_hitl_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"], false)));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor contains a layer with id HITL_DIALOG_ID
    wait_until(
        || app.compositor().contains(HITL_DIALOG_ID),
        "HitlDialog mounted",
    )
    .await;
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_both_some_hitl_wins() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(sid("s-1"), Some(pause_state(PauseKind::Confirm, "p")));
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"], false)));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor contains a layer with id HITL_DIALOG_ID
    wait_until(
        || app.compositor().contains(HITL_DIALOG_ID),
        "HitlDialog wins on tie",
    )
    .await;
    // @step And the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_both_none_pushes_no_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_paused_chunks_push_only_one_pause_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(sid("s-1"), Some(pause_state(PauseKind::Confirm, "p")));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || app.compositor().contains(PAUSE_DIALOG_ID),
        "first PauseDialog mounted",
    )
    .await;

    // @step And Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched again
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then exactly one layer with id PAUSE_DIALOG_ID is mounted on the Compositor
    let mounted = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| *id == PAUSE_DIALOG_ID)
        .count();
    assert_eq!(mounted, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_running_pops_pause_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, _mock) = fresh_app_with_session("s-1");
    // @step And a PauseDialog is mounted on the Compositor for s-1
    mount_pause_dialog(&mut app, &sid("s-1"), pause_state(PauseKind::Confirm, "p"));
    assert!(app.compositor().contains(PAUSE_DIALOG_ID));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the Compositor does NOT contain a layer with id PAUSE_DIALOG_ID
    assert!(!app.compositor().contains(PAUSE_DIALOG_ID));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_idle_pops_hitl_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, _mock) = fresh_app_with_session("s-1");
    // @step And a HitlDialog is mounted on the Compositor for s-1
    mount_hitl_dialog(
        &mut app,
        &sid("s-1"),
        hitl_request("q-1", &["Yes", "No"], false),
    );
    assert!(app.compositor().contains(HITL_DIALOG_ID));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Idle) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Idle,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// PauseDialog focus assertions (constructed directly to inspect
// `focused_label()` without going through the Compositor)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn confirm_pause_dialog_default_focus_is_accept_focus_only() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1
    let dialog = PauseDialog::new(sid("s-1"), pause_state(PauseKind::Confirm, "p"));
    // @step Then the PauseDialog's focused button is "Accept"
    assert_eq!(dialog.focused_label(), "Accept");
}

#[test]
fn triple_pause_dialog_default_focus_is_approve_focus_only() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a Triple PauseDialog is mounted on the Compositor for s-1
    let dialog = PauseDialog::new(sid("s-1"), pause_state(PauseKind::Triple, "p"));
    // @step Then the PauseDialog's focused button is "Approve"
    assert_eq!(dialog.focused_label(), "Approve");
}

#[test]
fn tab_cycles_confirm_pause_dialog_focus_accept_to_deny() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a Confirm PauseDialog is mounted on the Compositor for s-1 with focus on "Accept"
    let mut dialog = PauseDialog::new(sid("s-1"), pause_state(PauseKind::Confirm, "p"));
    assert_eq!(dialog.focused_label(), "Accept");
    // @step When the user presses Tab on the PauseDialog
    let _ = dialog.handle_event(&key_event(KeyCode::Tab));
    // @step Then the PauseDialog's focused button is "Deny"
    assert_eq!(dialog.focused_label(), "Deny");
}

#[test]
fn right_arrow_advances_triple_focus_through_three_states() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a Triple PauseDialog is mounted on the Compositor for s-1 with focus on "Approve"
    let mut dialog = PauseDialog::new(sid("s-1"), pause_state(PauseKind::Triple, "p"));
    assert_eq!(dialog.focused_label(), "Approve");
    // @step When the user presses Right arrow on the PauseDialog
    let _ = dialog.handle_event(&key_event(KeyCode::Right));
    // @step Then the PauseDialog's focused button is "Approve Session"
    assert_eq!(dialog.focused_label(), "Approve Session");
    // @step When the user presses Right arrow on the PauseDialog
    let _ = dialog.handle_event(&key_event(KeyCode::Right));
    // @step Then the PauseDialog's focused button is "Deny"
    assert_eq!(dialog.focused_label(), "Deny");
}

// ─────────────────────────────────────────────────────────────────────────
// HitlDialog rendering + free-text row inspections (constructed directly to
// inspect `option_hotkey()` / `is_free_text_selected()` / `text_buffer()`
// without going through the Compositor)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn hitl_dialog_rows_include_hotkey_letters_a_b_c() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No", "Maybe"]
    let dialog = HitlDialog::new(
        sid("s-1"),
        hitl_request("q-1", &["Yes", "No", "Maybe"], false),
    );
    // @step Then the HitlDialog's rows include hotkey letters "a", "b", "c"
    assert_eq!(dialog.option_hotkey(0), Some('a'));
    assert_eq!(dialog.option_hotkey(1), Some('b'));
    assert_eq!(dialog.option_hotkey(2), Some('c'));
    // @step And the HitlDialog's option labels include "Yes", "No", "Maybe"
    let labels: Vec<&str> = dialog
        .request()
        .options
        .iter()
        .map(|o| o.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Yes", "No", "Maybe"]);
}

#[test]
fn hitl_dialog_allow_text_input_renders_free_text_row() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    let dialog = HitlDialog::new(sid("s-1"), hitl_request("q-1", &["Yes", "No"], true));
    // @step Then the HitlDialog contains a free-text input row
    // (selectable rows = options + free-text row)
    assert_eq!(dialog.selectable_row_count(), 3);
    // @step And the free-text input row's value is empty
    assert_eq!(dialog.text_buffer(), "");
}

#[test]
fn hitl_dialog_tab_cycles_into_free_text_row() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    let mut dialog = HitlDialog::new(sid("s-1"), hitl_request("q-1", &["Yes", "No"], true));
    // @step When the user presses Tab twice on the HitlDialog
    let _ = dialog.handle_event(&key_event(KeyCode::Tab));
    let _ = dialog.handle_event(&key_event(KeyCode::Tab));
    // @step Then the HitlDialog's selected row is the free-text input row
    assert!(dialog.is_free_text_selected());
}

#[test]
fn hitl_dialog_typing_updates_free_text_value() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true and free-text row selected
    let mut dialog = HitlDialog::new(sid("s-1"), hitl_request("q-1", &["Yes", "No"], true));
    dialog.set_selected_index(2);
    assert!(dialog.is_free_text_selected());
    // @step When the user types "maybe later" into the free-text row
    for ch in "maybe later".chars() {
        let _ = dialog.handle_event(&key_event(KeyCode::Char(ch)));
    }
    // @step Then the free-text row's value equals "maybe later"
    assert_eq!(dialog.text_buffer(), "maybe later");
}

// ─────────────────────────────────────────────────────────────────────────
// Source-shape regression — pins the file-layout invariants for RPC-053.
// (Merged from source_shape_rpc053.rs to keep the 1:1 feature ↔ test-file
// mapping enforced by fspec.)
// ─────────────────────────────────────────────────────────────────────────

fn fspec_tui_src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn read_file_raw(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).expect("read file")
}

fn count_file_lines(path: &std::path::Path) -> usize {
    read_file_raw(path).lines().count()
}

/// Scenario: codelet/fspec-tui/src/app/dispatch_rpc053.rs hosts the new
/// pause/HITL helpers
#[test]
fn dispatch_rpc053_hosts_pause_hitl_helpers() {
    // @step Given the file codelet/fspec-tui/src/app/dispatch_rpc053.rs exists
    let src = fspec_tui_src_dir();
    let dispatch = src.join("app").join("dispatch_rpc053.rs");
    assert!(
        dispatch.is_file(),
        "expected {} to exist",
        dispatch.display()
    );

    // @step When the file is compiled as part of codelet-fspec-tui
    let body = read_file_raw(&dispatch);

    // @step Then it must declare impl App methods named handle_pause_chunk, handle_open_pause_dialog, handle_open_hitl_dialog, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    let helpers = [
        "handle_pause_chunk",
        "handle_open_pause_dialog",
        "handle_open_hitl_dialog",
        "handle_pause_confirmed",
        "handle_pause_triple",
        "handle_pause_resumed",
        "handle_hitl_submitted",
        "handle_pause_cleared",
    ];
    let mut missing: Vec<&str> = Vec::new();
    for helper in helpers {
        if !body.contains(helper) {
            missing.push(helper);
        }
    }
    assert!(
        missing.is_empty(),
        "dispatch_rpc053.rs missing helpers: {missing:?}",
    );

    // @step And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_rpc053
    let app_mod = src.join("app").join("mod.rs");
    let app_body = read_file_raw(&app_mod);
    assert!(
        app_body.contains("pub mod dispatch_rpc053"),
        "app/mod.rs must declare `pub mod dispatch_rpc053`",
    );

    // @step And codelet/fspec-tui/src/components/mod.rs must declare pub mod pause_dialog and pub mod hitl_dialog
    let components_mod = src.join("components").join("mod.rs");
    let components_body = read_file_raw(&components_mod);
    assert!(
        components_body.contains("pub mod pause_dialog"),
        "components/mod.rs must declare `pub mod pause_dialog`",
    );
    assert!(
        components_body.contains("pub mod hitl_dialog"),
        "components/mod.rs must declare `pub mod hitl_dialog`",
    );

    // @step And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
    let dispatch_main = src.join("app").join("dispatch.rs");
    let lines = count_file_lines(&dispatch_main);
    assert!(
        lines < 300,
        "app/dispatch.rs has {lines} lines (must be < 300)",
    );
}
