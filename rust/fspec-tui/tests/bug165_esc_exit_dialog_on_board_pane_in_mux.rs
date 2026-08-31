//! BUG-165 — Esc on the board pane in mux mode must show the exit dialog.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.
//!
//! Root cause: in mux mode the App-level fallback (`handle_app_shortcut`)
//! only pushes the BoardExitConfirmationDialog when
//! `active_view == ViewMode::Board`. With no open agent sessions the
//! agent slot is dropped from the effective panes (MUX-002 rule 3), the
//! Board pane stays focused, Esc is forwarded to the BoardView handler
//! (which ignores Esc), falls through to stage 4 — and there the
//! `ViewMode::Mux` guard swallows it. Esc was a dead key.
//!
//! Expected: Esc while the Board pane is focused in mux mode pushes the
//! same BoardExitConfirmationDialog as the single Board view (R9 —
//! dialogs overlay the mux). Esc on a focused AGENT pane keeps the
//! existing agent exit-confirmation cascade (Detach / Close Session /
//! Cancel) — regression guard.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::board_exit_confirmation_dialog::BOARD_EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
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

fn submit(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

fn esc() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
}

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

/// Enter mux with one open session, then close that session (Close
/// Session). BUG-164's fix focuses the board pane within the grid and
/// MUX-002's window re-derivation drops the now-unfilled agent slot —
/// leaving the Board pane as the only (focused) pane. This is the
/// realistic path to "mux mode, no agents open".
async fn enter_mux_then_close_the_session(app: &mut App) {
    app.dispatch(Action::SessionCreated(codelet_rpc_types::SessionId::new(
        "s-1",
    )));
    drain_pending(app).await;
    // MUX-004: bare /mux now opens the config dialog; "/mux on" keeps
    // the explicit enable path.
    submit(app, "/mux on");
    drain_pending(app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    drain_pending(app).await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing Esc on the board pane in mux mode with no open
// agents shows the exit dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing Esc on the board pane in mux mode with no open agents shows the exit dialog
#[tokio::test]
async fn pressing_esc_on_the_board_pane_in_mux_mode_with_no_open_agents_shows_the_exit_dialog() {
    // @step Given mux mode is active with the default Board and Agent panes and no agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_then_close_the_session(&mut app).await;
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert!(app.navigator().mux.config().enabled);
    // MUX-002 rule 3: with zero sessions the agent slot is dropped from
    // the effective panes — only the Board pane remains, and fresh
    // entry focuses it (index 0).
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes,
        &[MuxPaneKind::Board],
        "with no open agents the agent slot must be dropped from the effective panes"
    );
    assert_eq!(
        panes[app.navigator().mux.focus()],
        MuxPaneKind::Board,
        "the Board pane must be focused when no agents are open"
    );
    assert_eq!(app.compositor().len(), 0, "no dialog may be open yet");

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the BoardExitConfirmationDialog is shown over the full screen
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "ESC on the focused board pane in mux mode must push the BoardExitConfirmationDialog"
    );
    assert_eq!(
        app.compositor().topmost_id(),
        Some(BOARD_EXIT_CONFIRMATION_DIALOG_ID.to_string()),
        "the board exit confirmation dialog must be the topmost compositor layer"
    );
    assert!(
        !app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "ESC on the board pane must NOT open the agent exit confirmation dialog"
    );
    assert!(
        !app.should_quit(),
        "ESC must NOT quit directly — the confirmation dialog must appear first"
    );

    // @step And the Board pane is still focused and the mux grid is retained
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "ESC on the board pane must NOT flip the view out of the mux grid"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config must stay enabled"
    );
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes[app.navigator().mux.focus()],
        MuxPaneKind::Board,
        "the Board pane must stay focused inside the grid"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: confirming the board exit dialog in mux mode quits
// (the dialog commit path must work while the mux grid is active)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: confirming the board exit dialog in mux mode quits the application
#[tokio::test]
async fn confirming_the_board_exit_dialog_in_mux_mode_quits() {
    // @step Given mux mode is active with the default Board and Agent panes and no agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_then_close_the_session(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    // @step Then the BoardExitConfirmationDialog is shown over the full screen
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "the board exit confirmation dialog must be open before commit"
    );
    // @step When I confirm the Exit option (pre-selected — Enter commits)
    let _ = app.handle_event(&enter());
    drain_pending(&mut app).await;
    // @step Then the application exits
    assert!(
        app.should_quit(),
        "Enter on the pre-selected 'Exit' option must set should_quit=true"
    );
    // @step And the BoardExitConfirmationDialog is removed from the compositor
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "committing must remove the dialog from the compositor"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing Esc on the agent pane in mux mode with an open agent
// still shows the agent exit dialog (regression guard)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing Esc on the agent pane in mux mode with an open agent still shows the agent exit dialog
#[tokio::test]
async fn pressing_esc_on_the_agent_pane_in_mux_mode_with_an_open_agent_still_shows_the_agent_exit_dialog(
) {
    // @step Given mux mode is active with the default Board and Agent panes and one agent session is open
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(codelet_rpc_types::SessionId::new(
        "s-1",
    )));
    drain_pending(&mut app).await;
    // MUX-004: bare /mux now opens the config dialog; "/mux on" keeps
    // the explicit enable path.
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // Focus the Agent pane (last rendered pane).
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes[app.navigator().mux.focus()],
        MuxPaneKind::Agent,
        "the Agent pane must be focused"
    );
    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    // @step Then the agent exit confirmation dialog (Detach / Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "ESC on the focused agent pane must open the agent exit confirmation dialog"
    );
    assert!(
        !app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "ESC on the agent pane must NOT open the board exit confirmation dialog"
    );
    // @step And the Agent pane is focused
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes[app.navigator().mux.focus()],
        MuxPaneKind::Agent,
        "the Agent pane must stay focused inside the grid"
    );
}
