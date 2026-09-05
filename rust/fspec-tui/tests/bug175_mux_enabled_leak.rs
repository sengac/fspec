//! BUG-175 — the persisted `tui.mux.enabled` flag is a saved layout
//! preference, not a runtime mode. When it survives a restart with
//! `enabled=true`, the BackToBoard / EnterWorkUnit routing arms gate on
//! the persisted flag instead of the live `ViewMode::Mux`, so closing a
//! session from single-view mode strands the user on a blank,
//! unresponsive full-screen AgentView (Esc dead).
//!
//! Feature: spec/features/mux-close-session-landing.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use codelet_fspec_tui::components::board_exit_confirmation_dialog::BOARD_EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serial_test::serial;

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

fn esc() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
}

fn right() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
}

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

/// Open N sessions so the agent store has live sessions.
async fn app_with_sessions(app: &mut App, n_sessions: usize) {
    for i in 1..=n_sessions {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await;
}

/// Simulate the BUG-175 corruption: the persisted `tui.mux.enabled` flag
/// is `true` while the live view is NOT the mux grid. This is the state a
/// restarted TUI used to hold (bootstrap loaded the flag verbatim) and
/// the state a corrupted config can still produce.
fn simulate_leaked_persisted_flag(app: &mut App) {
    app.navigator_mut().mux.config_mut().enabled = true;
}

/// Root the process-global data directory at a fresh throwaway dir
/// (established tui093 / BUG-166 / BUG-167 pattern) so a test can seed
/// the user-scope `fspec-config.json` the way a real `~/.fspec` does.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set data dir");
    (guard, tmp)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: bootstrap force-disables a persisted mux grid and keeps the
// saved layout for /mux on
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: bootstrap force-disables a persisted mux grid and keeps the saved layout for /mux on
#[tokio::test]
#[serial]
async fn bootstrap_force_disables_a_persisted_mux_grid_and_keeps_the_saved_layout_for_mux_on() {
    // @step Given a fresh TUI bootstrap with a saved tui.mux config of Board and Agent at 50/50 with enabled true
    let (_data_guard, _data) = root_data_dir();
    let user_config = _data.path().join("fspec-config.json");
    std::fs::write(
        &user_config,
        r#"{"tui":{"mux":{"orientation":"Vertical","splits":[40],"panes":["Board","Agent"],"focused_pane":1,"enabled":true}}}"#,
    )
    .expect("seed user-scope fspec-config.json");
    let (mut app, _mock) = fresh_app();
    // The /mux slash command only routes while a session is open
    // (handle_input_submitted drops input with no current session), so
    // the fresh bootstrap seeds one session first.
    app_with_sessions(&mut app, 1).await;
    app.load_mux_config();
    use codelet_fspec_tui::views::multiplex::MuxOrientation;
    // @step When the TUI starts
    // (bootstrap loads the config — the observable state is what
    // `load_mux_config` leaves on the live layout + view)
    // @step Then the TUI is on the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "a fresh start must land on the single Board view, never in the grid"
    );
    // @step And the mux layout is disabled
    assert!(
        !app.navigator().mux.config().enabled,
        "the persisted enabled=true must NOT leak into the live layout (BUG-175)"
    );
    assert!(
        app.mux_state().config().enabled
            == app.navigator().mux.config().enabled,
        "the persistence mirror and the live layout must agree on the enabled flag"
    );
    // The saved LAYOUT is a preference — it must survive the force-off.
    assert_eq!(
        app.navigator().mux.config().orientation,
        MuxOrientation::Vertical,
        "the saved orientation must survive bootstrap"
    );
    assert_eq!(
        app.navigator().mux.config().splits,
        vec![40],
        "the saved split scale must survive bootstrap"
    );
    // @step And when I submit the slash command "/mux on" the grid is the saved 50/50 Board | Agent layout
    app.dispatch(Action::InputSubmitted("/mux on".to_string()));
    drain_pending(&mut app).await;
    assert!(
        app.navigator().mux.config().enabled,
        "/mux on must re-enable the mux"
    );
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(
        app.mux_state().config().orientation,
        MuxOrientation::Vertical,
        "/mux on must restore the saved orientation"
    );
    assert_eq!(
        app.mux_state().config().splits,
        vec![40],
        "/mux on must restore the saved split scale"
    );
    assert_eq!(
        app.mux_state().config().panes.len(),
        2,
        "/mux on must restore the saved pane list"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: back-to-board lands on the single Board view when the
// persisted mux flag is on but the grid is not active
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: back-to-board lands on the single Board view when the persisted mux flag is on but the grid is not active
#[tokio::test]
async fn back_to_board_lands_on_the_board_when_the_persisted_flag_is_on_but_the_grid_is_not_active() {
    // @step Given the TUI is on the single Agent view with one agent session open
    let (mut app, mock) = fresh_app();
    app_with_sessions(&mut app, 1).await;
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "one live session must be focused on the Agent view"
    );
    // @step And the persisted tui.mux enabled flag is on
    simulate_leaked_persisted_flag(&mut app);
    assert!(
        app.navigator().mux.config().enabled,
        "the leaked flag must be in place for this scenario"
    );
    assert_ne!(
        app.active_view(),
        ViewMode::Mux,
        "the scenario requires the grid to be NOT active"
    );
    // @step When the agent session is closed
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID),
        "the exit confirmation dialog must open on ESC from the agent view"
    );
    let _ = app.handle_event(&right()); // Detach -> Close Session
    let _ = app.handle_event(&enter());
    drain_pending(&mut app).await;
    assert_eq!(mock.destroy_session_calls(), 1, "the session must be destroyed");
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "the closed session must be gone from the open-session list"
    );
    // @step Then the TUI is on the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "closing the last session MUST land on the Board — the pre-fix routing gated on the persisted flag and stranded the view on a blank Agent"
    );
    // @step And Esc on the board opens the exit-fspec confirmation dialog
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "Esc must be alive on the board — the pre-fix state made it a dead key (the sessionless-agent BackToBoard no-op loop)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a board work unit enters the single Agent view when
// the persisted mux flag is on but the grid is not active
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on a board work unit enters the single Agent view when the persisted mux flag is on but the grid is not active
#[tokio::test]
async fn enter_on_a_board_work_unit_enters_the_single_agent_view_when_the_persisted_flag_is_on() {
    // @step Given the TUI is on the single Board view with work unit AUTH-001 selected
    let (mut app, _mock) = fresh_app();
    use codelet_rpc_types::WorkUnitInfo;
    app.dispatch(Action::WorkUnitsLoaded(vec![WorkUnitInfo {
        id: "AUTH-001".to_string(),
        title: "User Login".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }]));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the scenario requires the single Board view"
    );
    // @step And the persisted tui.mux enabled flag is on
    simulate_leaked_persisted_flag(&mut app);
    // @step When I press Enter
    let _ = app.handle_event(&enter());
    // The routing decision itself: with the grid not active, Enter must
    // take the single-view path (flip + attach), NOT the mux path
    // (MuxEnterWorkUnit — which, in a non-mux view, is a dead-end: the
    // Navigator arm ignores it and nothing flips the view). Drain the
    // bus observing each action AS IT LANDS so the routing choice is
    // observable before dispatch folds it.
    let mut saw_mux_enter = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::MuxEnterWorkUnit(_)) {
            saw_mux_enter = true;
        }
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
    assert!(
        !saw_mux_enter,
        "Enter from the single Board view must NOT route through MuxEnterWorkUnit (BUG-175: routing gates on the live ViewMode, not the persisted flag)"
    );
    // @step Then the TUI is on the single Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Enter on a board work unit in single-view mode must flip to the Agent view even with a leaked mux flag"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux default enters the grid with the enabled flag in
// lockstep with the live view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux default enters the grid with the enabled flag in lockstep with the live view
#[tokio::test]
async fn mux_default_enters_the_grid_with_the_enabled_flag_in_lockstep_with_the_live_view() {
    // @step Given the TUI is on the single Board view with one agent session open
    let (mut app, _mock) = fresh_app();
    app_with_sessions(&mut app, 1).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux default"
    app.dispatch(Action::InputSubmitted("/mux default".to_string()));
    drain_pending(&mut app).await;
    // @step Then the TUI is in mux mode
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "/mux default must enter the grid"
    );
    // @step And the mux enabled flag is on
    assert!(
        app.navigator().mux.config().enabled,
        "the invariant: enabled ⇔ ViewMode::Mux. Pre-fix, /mux default entered the grid with enabled=false — every flag-gated path (Shift+Left/Right intercept, key classification, the R6 auto-save) silently mis-fired inside the grid"
    );
}
