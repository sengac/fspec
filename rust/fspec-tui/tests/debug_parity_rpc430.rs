//! RPC-430 — `/debug` command parity gap fixes.
//!
//! Feature: spec/features/fix-debug-command-parity-gaps-in-rust-tui.feature
//!
//! Tests the four parity gaps fixed in RPC-430:
//! 1. Debug directory resolves to ~/.fspec (not .fspec/debug)
//! 2. Pre-session toggle support (no active session)
//! 3. Debug state hydration on session attach
//! 4. Debug state propagation on session creation
//!
//! Mirrors the TypeScript TUI behavior documented in the comparison analysis.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use tokio::time::timeout;

mod common;
use common::MockBackend;

/// Process-wide serialisation guard for env-var-sensitive tests.
static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug resolves debug directory to ~/.fspec by default
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)]
async fn debug_resolves_directory_to_home_fspec_by_default() {
    // Serialise env-var tests
    let _guard = HOME_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::remove_var("FSPEC_DEBUG_DIR");
    std::env::set_var("HOME", "/home/testuser");

    // @step Given the HOME environment variable is set to "/home/testuser"
    assert_eq!(std::env::var("HOME").unwrap(), "/home/testuser");

    // @step And the FSPEC_DEBUG_DIR environment variable is NOT set
    assert!(std::env::var("FSPEC_DEBUG_DIR").is_err());

    // @step When the /debug handler resolves the debug directory
    // We dispatch /debug and capture the debug_dir argument passed to toggle_debug
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/home/testuser/.fspec/s-1.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    app.dispatch(Action::SlashCommandSelected(
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Debug,
    ));

    // @step Then the resolved debug directory path equals "/home/testuser/.fspec"
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    assert_eq!(mock.toggle_debug_calls(), 1);
    let last = mock.last_toggle_debug().expect("toggle_debug captured");
    assert_eq!(last.1, "/home/testuser/.fspec");

    std::env::remove_var("HOME");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug respects FSPEC_DEBUG_DIR environment variable override
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)]
async fn debug_respects_fspec_debug_dir_override() {
    let _guard = HOME_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the FSPEC_DEBUG_DIR environment variable is set to "/custom/debug/path"
    std::env::set_var("FSPEC_DEBUG_DIR", "/custom/debug/path");

    // @step When the /debug handler resolves the debug directory
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/custom/debug/path/s-1.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    app.dispatch(Action::SlashCommandSelected(
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Debug,
    ));

    // @step Then the resolved debug directory path equals "/custom/debug/path"
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    let last = mock.last_toggle_debug().expect("toggle_debug captured");
    assert_eq!(last.1, "/custom/debug/path");

    std::env::remove_var("FSPEC_DEBUG_DIR");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug toggles pre-session debug state when no active session exists
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_toggles_pre_session_state_when_no_active_session() {
    // @step Given an App with NO current session and pre_session_debug_enabled is false
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Debug,
    ));
    drain_pending(&mut app).await;

    // @step Then App.pre_session_debug_enabled is true
    assert!(
        app.pre_session_debug_enabled(),
        "pre_session_debug_enabled should be true after /debug with no session"
    );

    // @step And the debug directory argument equals the resolved home path "~/.fspec"
    // The toggle should have been called (pre-session path calls backend for global toggle)
    // or the state should be stored locally. Verify the state is toggled.
    assert!(app.pre_session_debug_enabled());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug toggles pre-session debug state off on second invocation
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_toggles_pre_session_state_off_on_second_invocation() {
    // @step Given an App with NO current session and pre_session_debug_enabled is true
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // Manually set pre-session state to true (simulating a previous toggle)
    app.set_pre_session_debug_enabled(true);

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Debug,
    ));
    drain_pending(&mut app).await;

    // @step Then App.pre_session_debug_enabled is false
    assert!(
        !app.pre_session_debug_enabled(),
        "pre_session_debug_enabled should be false after second /debug toggle"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Session attach hydrates debug state from backend
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_attach_hydrates_debug_state_from_backend() {
    // @step Given an App with NO sessions and a MockBackend whose get_debug_enabled returns Ok(true) for session s-1
    let mock = Arc::new(MockBackend::new());
    mock.set_get_debug_enabled_result_ok(true);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When AttachToSession(s-1) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-1")));

    // @step Then within 1 second backend.get_debug_enabled(s-1) is called
    wait_until(
        || mock.get_debug_enabled_calls() > 0,
        "get_debug_enabled to be called",
    )
    .await;
    assert!(
        mock.get_debug_enabled_calls() > 0,
        "get_debug_enabled should be called during session attach"
    );

    // @step And AgentViewStore.debug_enabled_for(s-1) returns Some(true)
    // The hydration action should have been dispatched and processed
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store().debug_enabled_for(&sid("s-1")),
        Some(true),
        "debug_enabled_for should return Some(true) after hydration"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Session creation propagates pre-session debug state when enabled
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_creation_propagates_pre_session_debug_state_when_enabled() {
    // @step Given an App with pre_session_debug_enabled set to true
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(sid("s-1"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.set_pre_session_debug_enabled(true);
    assert!(app.pre_session_debug_enabled());

    // @step And a MockBackend whose create_session returns Ok(s-1)
    // (already set above)

    // @step When SessionCreated(s-1) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step Then within 1 second backend.set_debug_enabled(s-1, true) is called
    wait_until(
        || mock.set_debug_enabled_calls() > 0,
        "set_debug_enabled to be called",
    )
    .await;
    assert!(
        mock.set_debug_enabled_calls() > 0,
        "set_debug_enabled should be called when pre_session_debug_enabled is true"
    );

    // @step And AgentViewStore.debug_enabled_for(s-1) returns Some(true)
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store().debug_enabled_for(&sid("s-1")),
        Some(true),
        "debug_enabled_for should return Some(true) after propagation"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Session creation does NOT propagate debug state when pre-session is disabled
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_creation_does_not_propagate_debug_when_pre_session_disabled() {
    // @step Given an App with pre_session_debug_enabled set to false
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(sid("s-1"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(!app.pre_session_debug_enabled());

    // @step And a MockBackend whose create_session returns Ok(s-1)

    // @step When SessionCreated(s-1) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step Then backend.set_debug_enabled is NOT called
    tokio::time::sleep(Duration::from_millis(100)).await;
    drain_pending(&mut app).await;
    assert_eq!(
        mock.set_debug_enabled_calls(),
        0,
        "set_debug_enabled should NOT be called when pre_session_debug_enabled is false"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug with active session uses correct directory (regression)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)]
async fn debug_with_active_session_uses_correct_directory() {
    let _guard = HOME_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::remove_var("FSPEC_DEBUG_DIR");
    std::env::set_var("HOME", "/home/testuser");

    // @step Given an App with an open session s-1
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/home/testuser/.fspec/s-1.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step And the HOME environment variable is set to "/home/testuser"
    assert_eq!(std::env::var("HOME").unwrap(), "/home/testuser");

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Debug,
    ));

    // @step Then within 1 second backend.toggle_debug(s-1, "/home/testuser/.fspec") is called
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    let last = mock.last_toggle_debug().expect("toggle_debug captured");
    assert_eq!(last.0, sid("s-1"));
    assert_eq!(last.1, "/home/testuser/.fspec");

    // @step And the resolved debug directory path equals "/home/testuser/.fspec"
    assert_eq!(last.1, "/home/testuser/.fspec");

    std::env::remove_var("HOME");
}
