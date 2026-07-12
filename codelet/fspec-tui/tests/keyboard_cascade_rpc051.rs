//! RPC-051 — Keyboard shortcut parity (Esc interrupt cascade, Ctrl+R
//! focus, Shift+↑/↓ history recall).
//!
//! Feature: spec/features/keyboard-shortcut-cascade-parity.feature
//!
//! Drives the App::handle_event compositor → AgentView routing for the
//! five-level Esc cascade plus the Ctrl+R focus and Shift+↑/↓ recall
//! parity regressions that this slice pins.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::{
    Action, App, FspecBackend, HelpDialog, ResumeSessionView, SearchHistoryView, ViewMode,
};
use codelet_rpc_types::{SessionId, SessionInfo, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn esc() -> Event {
    key(KeyCode::Esc, KeyModifiers::NONE)
}

fn ctrl_r() -> Event {
    key(KeyCode::Char('r'), KeyModifiers::CONTROL)
}

fn shift_up() -> Event {
    key(KeyCode::Up, KeyModifiers::SHIFT)
}

fn shift_down() -> Event {
    key(KeyCode::Down, KeyModifiers::SHIFT)
}

fn fake_session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
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

/// Build an App in ViewMode::Agent with a single open session at the
/// supplied status. Returns the App + MockBackend for assertion access.
fn agent_app_with_status(status: SessionStatus) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), status);
    (app, mock)
}

/// Build an App in ViewMode::Agent with NO open session.
fn agent_app_no_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 1 — slash popup dismiss takes precedence over interrupt
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_1_slash_popup_dismiss_takes_precedence_over_interrupt() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And the slash command popup is open in the AgentView
    app.navigator_mut().agent.input.set_value("/");
    app.navigator_mut().agent.sync_popups();
    assert!(app.navigator().agent.slash_popup.is_some());
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    let interrupt_calls_before = mock.interrupt_calls();
    let starting_view = app.navigator().active_view;

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the slash popup closes
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), interrupt_calls_before);
    // @step And no Action::BackToBoard is dispatched
    assert_eq!(app.navigator().active_view, starting_view);
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 2 — HelpDialog dismiss takes precedence over interrupt
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_2_help_dialog_dismiss_takes_precedence_over_interrupt() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And the HelpDialog is pushed on the compositor
    app.compositor_mut().push(Box::new(HelpDialog::new()));
    assert!(app.compositor().contains("help-dialog"));
    let interrupt_calls_before = mock.interrupt_calls();

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the HelpDialog is removed from the compositor
    assert!(!app.compositor().contains("help-dialog"));
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), interrupt_calls_before);
    // @step And no Action::BackToBoard is dispatched
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 3 — resume mode-view dismiss takes precedence
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_3_resume_mode_view_dismiss_takes_precedence() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And the AgentView's resume_view is open
    let mut rsv = ResumeSessionView::new();
    rsv.set_sessions(vec![fake_session("s-1")]);
    app.navigator_mut().agent.resume_view = Some(rsv);
    let interrupt_calls_before = mock.interrupt_calls();

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then Action::CloseResumeView is dispatched
    // (observed indirectly via resume_view dropping to None)
    // @step And the AgentView's resume_view becomes None
    assert!(app.navigator().agent.resume_view.is_none());
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), interrupt_calls_before);
    // @step And no Action::BackToBoard is dispatched
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 3 — search mode-view dismiss takes precedence
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_3_search_mode_view_dismiss_takes_precedence() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And the AgentView's search_view is open
    app.navigator_mut().agent.search_view = Some(SearchHistoryView::new());
    let interrupt_calls_before = mock.interrupt_calls();

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then Action::CloseSearchView is dispatched (search_view = None)
    // @step And the AgentView's search_view becomes None
    assert!(app.navigator().agent.search_view.is_none());
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), interrupt_calls_before);
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 4 — Running session interrupts without navigating back
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_4_running_session_interrupts_without_navigating_back() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And no popup is open
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And no dialog is on the compositor
    assert!(!app.compositor().contains("help-dialog"));
    // @step And no mode-view is open
    assert!(app.navigator().agent.resume_view.is_none());
    assert!(app.navigator().agent.search_view.is_none());

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.interrupt is called exactly once with s-1
    wait_until(
        || mock.interrupt_calls() == 1,
        "interrupt_calls() to reach 1 for Running session",
    )
    .await;
    let last = mock.last_interrupt();
    assert_eq!(last, Some(sid("s-1")));
    // @step And no Action::BackToBoard is dispatched
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 4 — Compacting session interrupts without navigating
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_4_compacting_session_interrupts_without_navigating() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Compacting
    let (mut app, mock) = agent_app_with_status(SessionStatus::Compacting);
    // @step And no popup, dialog, or mode-view is active

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.interrupt is called exactly once with s-1
    wait_until(
        || mock.interrupt_calls() == 1,
        "interrupt_calls() to reach 1 for Compacting session",
    )
    .await;
    assert_eq!(mock.last_interrupt(), Some(sid("s-1")));
    // @step And no Action::BackToBoard is dispatched
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 5 — Idle session opens ExitConfirmationDialog (RPC-098)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_5_idle_session_navigates_back_to_board() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Idle
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And no popup, dialog, or mode-view is active

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // RPC-098: idle ESC at L7 now opens ExitConfirmationDialog instead of
    // dispatching Action::BackToBoard directly.
    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "idle-session ESC must push ExitConfirmationDialog (RPC-098)"
    );
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), 0);
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 5 — session with unknown status opens ExitConfirmationDialog (RPC-098)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_5_unknown_status_navigates_back_to_board() {
    // @step Given session s-1 is the current session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And session s-1 has no recorded SessionStatus
    assert!(app
        .agent_view_store()
        .session_status_for(&sid("s-1"))
        .is_none());
    // @step And no popup, dialog, or mode-view is active

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // RPC-098: unknown-status ESC at L7 now opens ExitConfirmationDialog
    // (treated as idle by the cascade).
    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "unknown-status ESC must push ExitConfirmationDialog (RPC-098)"
    );
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), 0);
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc level 5 — no current session navigates back to Board
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_5_no_current_session_navigates_back_to_board() {
    // @step Given there is NO current session
    let (mut app, mock) = agent_app_no_session();
    assert!(app.agent_view_store().current_session().is_none());
    // @step And no popup, dialog, or mode-view is active

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then Action::BackToBoard is dispatched
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), 0);
    // @step And Navigator.active_view becomes ViewMode::Board
    assert_eq!(app.navigator().active_view, ViewMode::Board);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Ctrl+R opens SearchHistoryView with input field focused
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_r_opens_search_history_view_with_input_focused() {
    // @step Given session s-1 is the current session
    let (mut app, _mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And no popup, dialog, or mode-view is active

    // @step When the user presses Ctrl+R
    let _ = app.handle_event(&ctrl_r());
    drain_pending(&mut app).await;

    // @step Then Action::OpenSearchView is dispatched
    // @step And the AgentView's search_view becomes Some
    assert!(app.navigator().agent.search_view.is_some());

    // @step When the user types the character "h"
    let _ = app.handle_event(&key(KeyCode::Char('h'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the search_view's query equals "h"
    let sv = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view stays open after first keystroke");
    assert_eq!(sv.query(), "h");
    // @step And Action::SearchHistory("h") is dispatched
    // (observed indirectly via the side-effect: search_view query reflects
    // the typed char, which only happens when the key is routed through
    // SearchHistoryView::handle_key.)
}
// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+up/down snapshots draft, walks history, restores draft
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shift_up_down_snapshots_walks_and_restores_draft() {
    // @step Given session s-1 is the current session
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the MockBackend's persistence_get_history scripted to return ["first", "second", "third"] for s-1
    mock.script_history(
        sid("s-1"),
        vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ],
    );
    // @step And the live MultiLineInput contains "draft-text"
    app.navigator_mut().agent.input.set_value("draft-text");

    // @step When the user presses Shift+Up once and waits for the snapshot to load
    let _ = app.handle_event(&shift_up());
    drain_pending(&mut app).await;
    wait_until(
        || {
            app.agent_view_store()
                .cached_history_snapshot(&sid("s-1"))
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        },
        "history snapshot to load for s-1",
    )
    .await;

    // @step Then the MultiLineInput value equals "first"
    assert_eq!(app.navigator().agent.input.value(), "first");
    // @step And AgentViewStore.history_state_for(s-1).cached_draft equals "draft-text"
    let st = app
        .agent_view_store()
        .history_state_for(&sid("s-1"))
        .expect("history state seeded");
    assert_eq!(st.cached_draft.as_deref(), Some("draft-text"));
    // @step And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
    assert_eq!(st.recall_index, Some(0));

    // @step When the user presses Shift+Up again
    let _ = app.handle_event(&shift_up());
    drain_pending(&mut app).await;
    // @step Then the MultiLineInput value equals "second"
    assert_eq!(app.navigator().agent.input.value(), "second");
    // @step And AgentViewStore.history_state_for(s-1).recall_index equals Some(1)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(1),
    );

    // @step When the user presses Shift+Down
    let _ = app.handle_event(&shift_down());
    drain_pending(&mut app).await;
    // @step Then the MultiLineInput value equals "first"
    assert_eq!(app.navigator().agent.input.value(), "first");
    // @step And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(0),
    );

    // @step When the user presses Shift+Down again
    let _ = app.handle_event(&shift_down());
    drain_pending(&mut app).await;
    // @step Then the MultiLineInput value equals "draft-text"
    assert_eq!(app.navigator().agent.input.value(), "draft-text");
    // @step And AgentViewStore.history_state_for(s-1).recall_index equals None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        None,
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+up at end of history is a no-op (clamped at tail)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shift_up_at_end_of_history_clamps_at_tail() {
    // @step Given session s-1 is the current session
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the MockBackend's persistence_get_history scripted to return ["only-entry"] for s-1
    mock.script_history(sid("s-1"), vec!["only-entry".to_string()]);
    // @step And the live MultiLineInput contains "draft"
    app.navigator_mut().agent.input.set_value("draft");

    // @step When the user presses Shift+Up and waits for the snapshot to load
    let _ = app.handle_event(&shift_up());
    drain_pending(&mut app).await;
    wait_until(
        || {
            app.agent_view_store()
                .cached_history_snapshot(&sid("s-1"))
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        },
        "history snapshot to load (single-entry) for s-1",
    )
    .await;
    // @step Then the MultiLineInput value equals "only-entry"
    assert_eq!(app.navigator().agent.input.value(), "only-entry");

    // @step When the user presses Shift+Up four more times
    for _ in 0..4 {
        let _ = app.handle_event(&shift_up());
        drain_pending(&mut app).await;
    }
    // @step Then the MultiLineInput value still equals "only-entry"
    assert_eq!(app.navigator().agent.input.value(), "only-entry");
    // @step And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(0),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-095 L6): Esc when idle with non-empty input clears the buffer
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_level_6_idle_session_with_nonempty_input_clears_buffer() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Idle
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer contains the text "hello world"
    app.navigator_mut().agent.input.set_value("hello world");

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the MultiLineInput value equals ""
    assert_eq!(app.navigator().agent.input.value(), "");
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And backend.interrupt is NEVER called
    assert_eq!(mock.interrupt_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-095): Esc when idle with whitespace-only input is treated as empty
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_idle_session_with_whitespace_only_input_navigates_back() {
    // @step Given session s-1 is the current session
    // @step And session s-1 has SessionStatus::Idle
    let (mut app, _mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer contains only whitespace "   "
    app.navigator_mut().agent.input.set_value("   ");

    // @step When the user presses Esc
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // RPC-098: whitespace-only input is treated as empty by trim(), so
    // L6 short-circuits and L7 opens ExitConfirmationDialog.
    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "idle whitespace-only ESC must push ExitConfirmationDialog (RPC-098)"
    );
    // @step And Navigator.active_view stays at ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}
