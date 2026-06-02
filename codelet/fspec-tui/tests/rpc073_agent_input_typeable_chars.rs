//! RPC-073 regression tests for the `?` / `q` global trap in AgentView input.
//!
//! Feature: spec/features/rpc-073-agent-input-typeable-chars.feature
//!
//! Before the RPC-073 fix, `App::handle_event` dispatched the
//! app-shortcut handler (`?`/`q`/`Ctrl+D`) BEFORE the compositor and
//! navigator. That meant typing `?` while the AgentView's
//! MultiLineInput was focused opened the HelpDialog instead of being
//! inserted into the input buffer. Same for `q` — it quit the app.
//!
//! After the fix, dispatch order is:
//!   1. DisconnectDialog (critical, unchanged — RPC-011 CR-1)
//!   2. Compositor.handle_event
//!   3. Navigator.handle_event
//!   4. handle_app_shortcut — only if 2 and 3 returned Ignored
//!
//! This makes the AgentView's MultiLineInput first to receive `?`/`q`
//! and matches the TS Ink frontend's InputManager priority chain (text
//! input at MEDIUM > view shortcuts at LOW).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::disconnect_dialog::DisconnectDialog, synth_key, Action, App, FspecBackend,
    Priority, ViewMode,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

mod common;

use common::{harness::AppTestHarness, test_app, MockBackend};

/// Build a fresh App focused on the AgentView, mirroring how
/// `AppTestHarness::new()` switches via `Action::OpenAgentView`.
fn agent_view_app() -> App {
    AppTestHarness::new().app
}

/// Build a fresh App on the BoardView (no view switch).
fn board_view_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let (app, _term) = test_app(backend);
    app
}

// =============================================================================
// Scenario: Typing ? while the AgentView input is focused appends ? to the
// buffer and does not open the HelpDialog
// =============================================================================
#[test]
fn question_mark_in_agent_view_input_appends_to_buffer_no_help_dialog() {
    // @step Given an App is constructed with active_view = Agent and a focused MultiLineInput with empty buffer
    let mut app = agent_view_app();
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(app.navigator().agent.input.value(), "");
    assert_eq!(app.compositor().len(), 0);

    // @step When the app handles a KeyCode::Char('?') event with KeyModifiers::NONE
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));

    // @step Then the AgentView input buffer contains the literal '?' character
    assert_eq!(
        app.navigator().agent.input.value(),
        "?",
        "RPC-073 bug 2: `?` was trapped by app-shortcut instead of appended to AgentView input",
    );

    // @step Then the Compositor does not contain a HelpDialog layer
    assert!(
        !app.compositor().contains("help-dialog"),
        "RPC-073 bug 2: HelpDialog opened while typing in AgentView input",
    );
}

// =============================================================================
// Scenario: Typing q while the AgentView input is focused appends q to the
// buffer and does not quit the app
// =============================================================================
#[test]
fn q_in_agent_view_input_appends_to_buffer_no_quit() {
    // @step Given an App is constructed with active_view = Agent and a focused MultiLineInput with empty buffer
    let mut app = agent_view_app();
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(app.navigator().agent.input.value(), "");
    assert!(!app.should_quit());

    // @step When the app handles a KeyCode::Char('q') event with KeyModifiers::NONE
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then the AgentView input buffer contains the literal 'q' character
    assert_eq!(
        app.navigator().agent.input.value(),
        "q",
        "RPC-073 bug 2: `q` was trapped by app-shortcut instead of appended to AgentView input",
    );

    // @step Then App::should_quit remains false
    assert!(
        !app.should_quit(),
        "RPC-073 bug 2: app quit while typing `q` into AgentView input",
    );
}

// =============================================================================
// Scenario: Pressing ? while the BoardView is focused still opens the
// HelpDialog
// =============================================================================
#[test]
fn question_mark_in_board_view_still_opens_help_dialog() {
    // @step Given an App is constructed with active_view = Board
    let mut app = board_view_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.compositor().len(), 0);

    // @step When the app handles a KeyCode::Char('?') event with KeyModifiers::NONE
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));

    // @step Then the Compositor contains a HelpDialog layer at the top of the stack
    assert_eq!(
        app.compositor().topmost_id(),
        Some("help-dialog".to_string()),
        "HelpDialog should still open from BoardView after RPC-073 fix",
    );
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Critical));
}

// =============================================================================
// Scenario: Pressing q while the BoardView is focused is IGNORED (RPC-102).
//
// Before RPC-102, the Stage-4 app-shortcut bound `q` to quit. RPC-102
// inverted the binding: `q` is no longer a quit shortcut on the BoardView
// (TS parity — `src/tui/components/BoardView.tsx` uses `key.escape`).
// The BoardView ESC binding is exercised by
// `tests/boardview_esc_exit_confirmation_rpc102.rs`.
// =============================================================================
#[test]
fn q_in_board_view_is_ignored_after_rpc102() {
    // @step Given an App is constructed with active_view = Board and no critical dialog topmost
    let mut app = board_view_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.should_quit());
    assert!(!matches!(
        app.compositor().topmost_priority(),
        Some(Priority::Critical)
    ));

    // @step When the app handles a KeyCode::Char('q') event with KeyModifiers::NONE
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then App::should_quit remains false (RPC-102: 'q' is no longer a quit binding outside DisconnectDialog)
    assert!(
        !app.should_quit(),
        "RPC-102: BoardView `q` shortcut must NOT quit — TS parity uses ESC for exit confirmation",
    );
    // @step And no compositor layer is pushed
    assert_eq!(
        app.compositor().len(),
        0,
        "RPC-102: BoardView `q` must not push any compositor layer",
    );
}

// =============================================================================
// Scenario: Pressing Ctrl+D while the AgentView input is focused still
// quits the app
// =============================================================================
#[test]
fn ctrl_d_in_agent_view_input_still_quits() {
    // @step Given an App is constructed with active_view = Agent and a focused MultiLineInput
    let mut app = agent_view_app();
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert!(!app.should_quit());

    // @step When the app handles a KeyCode::Char('d') event with KeyModifiers::CONTROL
    let key = KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    };
    let _ = app.handle_event(&crossterm::event::Event::Key(key));

    // @step Then App::should_quit is set to true
    assert!(
        app.should_quit(),
        "Ctrl+D in AgentView input must still quit the app (fallback shortcut)",
    );
}

// =============================================================================
// Scenario: When the critical DisconnectDialog is topmost, q is intercepted
// by the DisconnectDialog handler and the Compositor never sees the event
// =============================================================================
#[test]
fn q_with_disconnect_dialog_topmost_is_intercepted_by_dialog() {
    // @step Given an App has a DisconnectDialog pushed onto the Compositor and it is the topmost critical-priority layer
    let mut app = agent_view_app();
    app.compositor_mut()
        .push(Box::new(DisconnectDialog::new()));
    assert_eq!(
        app.compositor().topmost_priority(),
        Some(Priority::Critical),
        "DisconnectDialog must be topmost critical layer",
    );
    let pre_input = app.navigator().agent.input.value();

    // @step When the app handles a KeyCode::Char('q') event
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then App::should_quit becomes true and the DisconnectDialog is removed from the Compositor
    //
    // RPC-011 CR-1: DisconnectDialog handles `q` itself and clears
    // itself from the Compositor via the disconnect-cleanup path.
    assert!(
        app.should_quit(),
        "DisconnectDialog must handle `q` itself even after RPC-073 dispatch-order inversion",
    );
    assert!(
        !app.compositor().contains("disconnect-dialog"),
        "DisconnectDialog removes itself when `q` is pressed",
    );
    // And the AgentView input must NOT have received `q` — the
    // DisconnectDialog ran the disconnect-event handler at Stage 1
    // (before any other dispatch).
    assert_eq!(
        app.navigator().agent.input.value(),
        pre_input,
        "AgentView input must NOT receive `q` when DisconnectDialog is topmost",
    );
}

// Silence unused-import warnings for symbols only used by some scenarios.
#[allow(dead_code)]
fn _silence_unused() {
    let _ = Action::SessionCreated;
}
