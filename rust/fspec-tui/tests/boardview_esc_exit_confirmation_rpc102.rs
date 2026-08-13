//! RPC-102 — BoardView ESC exit confirmation (TS parity).
//!
//! Feature: spec/features/boardview-esc-key-exit-confirmation.feature
//!
//! This test file is written BEFORE the implementation. Until RPC-102
//! lands, `BoardExitConfirmationDialog`, `BOARD_EXIT_CONFIRMATION_DIALOG_ID`,
//! and the BoardView ESC contract do not yet exist in the crate — the file
//! will not compile, which is the canonical Rust "red phase".
//!
//! Each scenario in the feature file is exercised by exactly one `#[test]`
//! (or `#[tokio::test]`) below, and every Gherkin step has a matching
//! `// @step ...` comment placed immediately before the code that exercises
//! it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::components::board_exit_confirmation_dialog::{
    BoardExitConfirmationDialog, BOARD_EXIT_CONFIRMATION_DIALOG_ID,
};
use codelet_fspec_tui::components::disconnect_dialog::DisconnectDialog;
use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{synth_key, Action, App, Component, FspecBackend, Priority};
use codelet_rpc_types::SessionStatus;
use crossterm::event::KeyCode;

mod common;
use common::{harness::AppTestHarness, test_app, MockBackend};

// ───────────────────────── helpers ──────────────────────────────────────

/// Build a fresh App focused on the BoardView with no overlays.
fn board_view_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let (app, _term) = test_app(backend);
    app
}

/// Build a fresh App focused on the AgentView with an idle session.
fn agent_view_app_idle() -> App {
    let harness = AppTestHarness::new();
    let mut app = harness.app;
    // Mark the focused session idle so ESC-cascade L4 (interrupt) doesn't
    // fire and instead falls through to L6 (clear input).
    if let Some(sid) = app.agent_view_store().current_session().cloned() {
        app.agent_view_store_mut()
            .set_session_status(sid, SessionStatus::Idle);
    }
    app
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Esc on the BoardView opens an exit confirmation dialog
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_esc_on_the_boardview_opens_an_exit_confirmation_dialog() {
    // @step Given I am viewing the BoardView with no overlay
    let mut app = board_view_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.compositor().len(), 0);
    assert!(!app.should_quit());

    // @step When I press the Esc key
    let _ = app.handle_event(&synth_key(KeyCode::Esc));

    // @step Then an "Exit fspec?" confirmation dialog appears over the board
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "BoardView ESC must push BoardExitConfirmationDialog onto compositor"
    );
    assert_eq!(
        app.compositor().topmost_id(),
        Some(BOARD_EXIT_CONFIRMATION_DIALOG_ID.to_string()),
        "BoardExitConfirmationDialog must be the topmost compositor layer"
    );

    // @step And the application is still running
    assert!(
        !app.should_quit(),
        "BoardView ESC must NOT set should_quit — TS parity requires confirmation first"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'q' on the BoardView is ignored
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_q_on_the_boardview_is_ignored() {
    // @step Given I am viewing the BoardView with no overlay
    let mut app = board_view_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.compositor().len(), 0);
    assert!(!app.should_quit());

    // @step When I press the 'q' key
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then no dialog appears
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "'q' on BoardView must NOT push BoardExitConfirmationDialog"
    );
    assert_eq!(
        app.compositor().len(),
        0,
        "'q' on BoardView must not push any compositor layer"
    );

    // @step And the application is still running
    assert!(
        !app.should_quit(),
        "'q' on BoardView must NOT set should_quit — TS parity: 'q' is not a quit binding outside DisconnectDialog"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Confirming the exit dialog closes the application
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirming_the_exit_dialog_closes_the_application() {
    // @step Given the BoardView exit confirmation dialog is showing
    let mut app = board_view_app();
    let _ = app.handle_event(&synth_key(KeyCode::Esc));
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "precondition: dialog must be open"
    );

    // @step And the "Exit" option is selected
    // BoardExitConfirmationDialog uses a two-option layout [Exit, Cancel]
    // with Exit pre-selected (TS parity: visual confirmMode, riskLevel=medium).
    // Default selection is Exit — no navigation required.

    // @step When I press Enter
    let _ = app.handle_event(&synth_key(KeyCode::Enter));

    // Drain any actions emitted by the dialog onto the action bus and
    // dispatch them through the App so the QuitApp action flips
    // should_quit.
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }

    // @step Then the dialog closes
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "Enter on Exit must remove BoardExitConfirmationDialog from compositor"
    );

    // @step And the application exits
    assert!(app.should_quit(), "Enter on Exit must set should_quit=true");
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Cancelling the exit dialog with Esc returns to the board
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_the_exit_dialog_with_esc_returns_to_the_board() {
    // @step Given the BoardView exit confirmation dialog is showing
    let mut app = board_view_app();
    let _ = app.handle_event(&synth_key(KeyCode::Esc));
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "precondition: dialog must be open"
    );

    // @step When I press the Esc key
    let _ = app.handle_event(&synth_key(KeyCode::Esc));

    // Drain any deferred Compositor callbacks.
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }

    // @step Then the dialog disappears
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "ESC inside dialog must remove BoardExitConfirmationDialog"
    );

    // @step And I am returned to the BoardView
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Cancel must keep the user on the BoardView"
    );

    // @step And the application is still running
    assert!(
        !app.should_quit(),
        "Cancelling exit dialog must NOT set should_quit"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: DisconnectDialog still honors 'q' to quit (RPC-011 CR-1 regression guard)
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_dialog_still_honors_q_to_quit() {
    // @step Given the backend connection has dropped
    let mut app = board_view_app();

    // @step And the DisconnectDialog is showing
    app.compositor_mut().push(Box::new(DisconnectDialog::new()));
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "precondition: DisconnectDialog must be topmost"
    );
    assert_eq!(
        app.compositor().topmost_priority(),
        Some(Priority::Critical)
    );
    assert!(!app.should_quit());

    // @step When I press the 'q' key
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then the application exits
    assert!(
        app.should_quit(),
        "DisconnectDialog 'q' must still quit — RPC-011 CR-1 preserved"
    );
    assert!(
        !app.compositor().contains("disconnect-dialog"),
        "DisconnectDialog removes itself when 'q' is pressed"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: DisconnectDialog still honors 'r' to reconnect
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_dialog_still_honors_r_to_reconnect() {
    // @step Given the backend connection has dropped
    let mut app = board_view_app();

    // @step And the DisconnectDialog is showing
    app.compositor_mut().push(Box::new(DisconnectDialog::new()));
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string())
    );
    assert!(!app.should_quit());

    // Drain any actions on the bus so we can isolate the ManualReconnect
    // emission below.
    while let Some(_action) = app.try_recv_action() {}

    // @step When I press the 'r' key
    let _ = app.handle_event(&synth_key(KeyCode::Char('r')));

    // @step Then a manual reconnection attempt is initiated
    let mut saw_manual_reconnect = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::ManualReconnect) {
            saw_manual_reconnect = true;
        }
        app.dispatch(action);
    }
    assert!(
        saw_manual_reconnect,
        "Pressing 'r' in DisconnectDialog must emit Action::ManualReconnect — RPC-011 CR-1 preserved"
    );

    // @step And the application is still running
    assert!(
        !app.should_quit(),
        "Pressing 'r' must NOT quit — only 'q' quits inside DisconnectDialog"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: AgentView Esc-cascade still clears non-empty input
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agentview_esc_cascade_still_clears_non_empty_input() {
    // @step Given I am viewing the AgentView with a session attached
    let mut app = agent_view_app_idle();
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert!(app.agent_view_store().current_session().is_some());

    // @step And the input box contains the text "draft message"
    for ch in "draft message".chars() {
        let _ = app.handle_event(&synth_key(KeyCode::Char(ch)));
    }
    assert_eq!(
        app.navigator().agent.input.value(),
        "draft message",
        "precondition: input must contain draft text"
    );

    // @step When I press the Esc key
    let _ = app.handle_event(&synth_key(KeyCode::Esc));

    // Drain any actions emitted by the Esc-cascade so AgentEscPressed
    // routes through App::dispatch and L6 (clear input) executes.
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }

    // @step Then the input box is cleared
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "AgentView Esc-cascade L6 must clear non-empty input — regression guard"
    );

    // @step And no exit confirmation dialog appears
    assert!(
        !app.compositor()
            .contains(EXIT_CONFIRMATION_DIALOG_ID),
        "AgentView Esc on non-empty input must NOT push ExitConfirmationDialog (L6 fires before L7)"
    );
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "AgentView Esc must NOT push BoardExitConfirmationDialog — wrong view"
    );

    // @step And the application is still running
    assert!(!app.should_quit(), "AgentView Esc-cascade L6 must NOT quit");
}

// ──────────────────────────────────────────────────────────────────────────
// Static component invariants (priority, id, default selection)
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn dialog_priority_is_critical_and_id_matches_const() {
    // @step Given a freshly constructed BoardExitConfirmationDialog
    let dialog = BoardExitConfirmationDialog::new();
    // @step Then its Component::priority is Critical
    assert_eq!(
        dialog.priority(),
        Priority::Critical,
        "BoardExitConfirmationDialog must use Priority::Critical so it overlays the BoardView"
    );
    // @step And its Component::id equals BOARD_EXIT_CONFIRMATION_DIALOG_ID
    assert_eq!(dialog.id(), BOARD_EXIT_CONFIRMATION_DIALOG_ID);
}

// ──────────────────────────────────────────────────────────────────────────
// Idempotency guard — second ESC on the BoardView while the dialog is open
// must not push a second dialog instance.
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_esc_twice_on_boardview_only_opens_one_dialog() {
    // @step Given I am viewing the BoardView with no overlay
    let mut app = board_view_app();
    assert_eq!(app.compositor().len(), 0);

    // @step When I press ESC once
    let _ = app.handle_event(&synth_key(KeyCode::Esc));
    // @step Then the BoardExitConfirmationDialog is on the compositor
    assert!(app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID));
    let len_after_first = app.compositor().len();

    // @step When I press ESC a second time (the dialog itself handles it
    //       as Cancel and removes itself)
    let _ = app.handle_event(&synth_key(KeyCode::Esc));
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }

    // @step Then the dialog is gone (its own ESC handler == Cancel)
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "Second ESC must close the dialog (ESC == Cancel inside the dialog)"
    );

    // The compositor must NOT have grown past one instance during the
    // initial push — guard against double-push.
    assert_eq!(
        len_after_first, 1,
        "First ESC must push exactly one BoardExitConfirmationDialog"
    );
}
