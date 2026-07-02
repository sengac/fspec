//! RPC-053 — Pause / HITL UI (chunk-driven trigger + HitlDialog end-to-end).
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! RPC-406 superseded the PauseDialog modal with the inline pause prompt
//! (see tests/inline_pause_prompt_rpc406.rs). This file retains the
//! chunk-driven Paused trigger (parallel get_pause_state + get_hitl_request,
//! HITL wins on tie, pause state lands in the AgentViewStore slot) and the
//! HitlDialog interactions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, Component, FspecBackend, HITL_DIALOG_ID};
use codelet_rpc_types::{
    HitlOption, HitlRequest, PauseKind, PauseState, SessionId, SessionState, StreamChunk,
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

fn mount_hitl_dialog(app: &mut App, session: &SessionId, request: HitlRequest) {
    app.dispatch(Action::OpenHitlDialog {
        session_id: session.clone(),
        request,
    });
}

// ─────────────────────────────────────────────────────────────────────────
// Chunk-driven trigger: SessionStateChange { Paused } routes correctly
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_hitl_request_some_opens_a_hitl_dialog() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Continue?", header: "Apply changes?", options: [Yes, No], allow_text_input: false } for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"], false)));

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
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
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_state_change_paused_with_both_some_hitl_wins() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    mock.script_pause_state(sid("s-1"), Some(pause_state(PauseKind::Confirm, "p")));
    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Q", header: "H", options: [Yes, No], allow_text_input: false } for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"], false)));

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
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
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
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
    // @step And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    assert!(!app.compositor().contains(HITL_DIALOG_ID));
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

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Idle }) is dispatched
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
    // @step And all pending tasks have drained
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
    // @step And all pending tasks have drained
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
    // @step And all pending tasks have drained
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
// HitlDialog rendering + free-text row inspections
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn hitl_dialog_rows_include_hotkey_letters_a_b_c() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No", "Maybe"] and allow_text_input=false
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
    // @step And the HitlDialog's selected row is the first option (index 0)
    // @step When the user presses Tab on the HitlDialog three times
    let _ = dialog.handle_event(&key_event(KeyCode::Tab));
    let _ = dialog.handle_event(&key_event(KeyCode::Tab));
    // @step Then the HitlDialog's selected row is the free-text input row
    assert!(dialog.is_free_text_selected());
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatch_pause_hitl_hosts_the_pause_hitl_helpers() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // @step Given the file codelet/fspec-tui/src/app/dispatch_pause_hitl.rs exists
    let dph_path = src.join("app").join("dispatch_pause_hitl.rs");
    assert!(dph_path.exists(), "dispatch_pause_hitl.rs must exist");

    // @step When the file is compiled as part of codelet-fspec-tui
    let dph = std::fs::read_to_string(&dph_path).expect("read dispatch_pause_hitl.rs");

    // @step Then it must declare impl App methods named handle_pause_chunk, handle_open_hitl_dialog, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    for method in [
        "fn handle_pause_chunk",
        "fn handle_open_hitl_dialog",
        "fn handle_pause_confirmed",
        "fn handle_pause_triple",
        "fn handle_pause_resumed",
        "fn handle_hitl_submitted",
        "fn handle_pause_cleared",
    ] {
        assert!(dph.contains(method), "missing {method}");
    }

    // @step And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_pause_hitl
    let app_mod = std::fs::read_to_string(src.join("app").join("mod.rs")).expect("read app/mod.rs");
    assert!(app_mod.contains("pub mod dispatch_pause_hitl"));

    // @step And codelet/fspec-tui/src/components/mod.rs must declare pub mod hitl_dialog and no pub mod pause_dialog
    let components = std::fs::read_to_string(src.join("components").join("mod.rs"))
        .expect("read components/mod.rs");
    assert!(components.contains("pub mod hitl_dialog"));
    assert!(!components.contains("pub mod pause_dialog"));

    // @step And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
    let dispatch =
        std::fs::read_to_string(src.join("app").join("dispatch.rs")).expect("read dispatch.rs");
    assert!(
        dispatch.lines().count() <= 300,
        "app/dispatch.rs exceeds 300 lines"
    );
}

#[test]
fn hitl_dialog_typing_updates_free_text_value() {
    use codelet_fspec_tui::HitlDialog;
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    // @step And a HitlDialog is mounted on the Compositor for s-1 with allow_text_input=true
    let mut dialog = HitlDialog::new(sid("s-1"), hitl_request("q-1", &["Yes", "No"], true));
    // @step And the HitlDialog's selected row is the free-text input row
    dialog.set_selected_index(2);
    assert!(dialog.is_free_text_selected());
    // @step When the user types "maybe later" into the free-text row
    for ch in "maybe later".chars() {
        let _ = dialog.handle_event(&key_event(KeyCode::Char(ch)));
    }
    // @step Then the free-text row's value equals "maybe later"
    assert_eq!(dialog.text_buffer(), "maybe later");
}
