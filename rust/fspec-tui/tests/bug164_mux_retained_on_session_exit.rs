//! BUG-164 — closing a session in mux mode must retain the mux.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.
//!
//! The exit-confirmation dialog (Detach / Close Session / Cancel)
//! routes both the Detach and Close Session choices through
//! `Action::BackToBoard`. When the mux grid is active, BackToBoard is
//! a "focus the board pane within the grid" semantic — it must never
//! flip the whole view out of Mux (the pre-fix behavior painted the
//! single Board view and "turned the mux off").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

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

fn right() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
}

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

/// Open N sessions and enable mux (default preset: Board | Agent).
async fn app_with_sessions_and_mux(app: &mut App, n_sessions: usize) {
    for i in 1..=n_sessions {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await;
    // MUX-004: bare /mux now opens the config dialog; the explicit
    // "/mux on" keeps the enable-with-default-preset path.
    submit(app, "/mux on");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing a session in mux mode retains the mux and focuses the
// board pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing a session in mux mode retains the mux and focuses the board pane
#[tokio::test]
async fn closing_a_session_in_mux_mode_retains_the_mux_and_focuses_the_board_pane() {
    // @step Given mux mode is active with Board and Agent panes and two agent sessions are open
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_mux(&mut app, 2).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 2);
    assert!(app.navigator().mux.config().enabled);
    // @step And the Agent pane is focused
    // Focusing the agent pane routes the store's current session to the
    // pane's window slot (BUG-163 sync_mux_focus_to_session, applied on
    // the next event): with one window slot and two sessions the window
    // sits at start, so the pane hosts s-1.
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When the exit dialog is answered with Close Session
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "the exit confirmation dialog must open on ESC from the agent pane"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "the pane-hosted session s-1 must be the store's current session"
    );
    let _ = app.handle_event(&right()); // Detach -> Close Session
    let _ = app.handle_event(&enter());
    drain_pending(&mut app).await;
    // @step Then the destroyed session is removed from the open-session list
    assert_eq!(
        mock.destroy_session_calls(),
        1,
        "Close Session must destroy the pane-hosted session (s-1)"
    );
    assert_eq!(
        mock.last_destroyed_session(),
        Some(sid("s-1")),
        "the pane-hosted session s-1 must be the one destroyed"
    );
    let remaining: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(
        remaining,
        vec![sid("s-2")],
        "only the closed session is removed; the other stays open"
    );
    // @step And the TUI is still in mux mode with the same panes and layout
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "BackToBoard must NOT flip the whole view out of the mux grid"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config must stay enabled after a session close"
    );
    let panes = app.navigator().mux.effective_panes();
    assert!(
        panes.contains(&MuxPaneKind::Board),
        "the Board pane must survive the session close (panes: {panes:?})"
    );
    // @step And the Board pane is focused within the grid
    let focused = app.navigator().mux.effective_panes()[app.navigator().mux.focus()];
    assert_eq!(
        focused,
        MuxPaneKind::Board,
        "BackToBoard must focus the Board pane inside the grid"
    );
    // @step And no single-view flip to Board occurs
    assert_ne!(
        app.active_view(),
        ViewMode::Board,
        "the single Board view must NOT be painted while mux is enabled"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: detaching from a session in mux mode retains the mux and
// focuses the board pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: detaching from a session in mux mode retains the mux and focuses the board pane
#[tokio::test]
async fn detaching_from_a_session_in_mux_mode_retains_the_mux_and_focuses_the_board_pane() {
    // @step Given mux mode is active with Board and Agent panes and one agent session is open
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_mux(&mut app, 1).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the Agent pane is focused
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When the exit dialog is answered with Detach
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "the exit confirmation dialog must open on ESC from the agent pane"
    );
    let _ = app.handle_event(&enter()); // Detach is pre-selected
    drain_pending(&mut app).await;
    // @step Then the session remains open in the store
    assert_eq!(mock.destroy_session_calls(), 0);
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "Detach must NOT remove the session from open_sessions"
    );
    // @step And the TUI is still in mux mode with the same panes and layout
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "BackToBoard must NOT flip the whole view out of the mux grid"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config must stay enabled after a detach"
    );
    // @step And the Board pane is focused within the grid
    let focused = app.navigator().mux.effective_panes()[app.navigator().mux.focus()];
    assert_eq!(
        focused,
        MuxPaneKind::Board,
        "BackToBoard must focus the Board pane inside the grid"
    );
}
