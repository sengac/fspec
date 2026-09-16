//! BUG-183 — Esc on the Files/Checkpoints mux pane closes that pane.
//!
//! Feature: spec/features/mux-pane-esc-close.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.
//!
//! Root cause: `forward_to_pane` in `views/multiplex/keys.rs` maps every
//! non-Ignored outcome (including `Close`) to a plain
//! `EventResult::consumed()`, so Esc on a Files/Checkpoints mux pane was
//! a dead key. The fix translates the views' `Close` outcome onto the
//! action bus (the same `CloseChangedFilesView` /
//! `CloseCheckpointsView` the single-view Navigator path emits) and the
//! Navigator's `apply_action` arm then removes the pane kind from the
//! LIVE mux grid only — the saved `tui.mux` config stays untouched, so
//! `/mux off` then `/mux on` restores the full layout (same transience
//! as the agent slot dropping on session close).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory (the
/// mux-exit R6 auto-save writes the shared `fspec-config.json`).
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the established mux004 pattern). Returns the guard
/// (held for the test's duration) + the TempDir.
fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = TempDir::new().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf()).expect("set data dir");
    (guard, tmp)
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

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

/// Poll until a lazy view's load tracker is idle.
async fn wait_for_view_idle(app: &mut App, changed_files: bool) {
    for _ in 0..100 {
        drain_pending(app).await;
        let idle = if changed_files {
            !app.navigator().changed_files.load.is_loading()
        } else {
            !app.navigator().checkpoints.load.is_loading()
        };
        if idle {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!(
        "timeout: the {} view did not finish loading",
        if changed_files {
            "ChangedFiles"
        } else {
            "Checkpoints"
        }
    );
}

/// Enter mux mode with a specific pane list and wait for the rendered
/// lazy views (ChangedFiles / Checkpoints) to finish their initial
/// loads. One agent session is open so the Agent pane has something to
/// render.
async fn setup_mux_with_panes(app: &mut App, panes: Vec<MuxPaneKind>) {
    // Create a session so the Agent pane has something to render (and
    // the slash-command submit path has a current session).
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(app).await;
    app.navigator_mut().mux.set_pane_list(panes, None);
    submit(app, "/mux on");
    drain_pending(app).await;
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "mux mode must be active after /mux on"
    );
    // Only wait for the lazy views that are actually RENDERED (an
    // un-rendered pane's view keeps its pre-mux load state).
    let rendered = app.navigator().mux.effective_panes().to_vec();
    if rendered.contains(&MuxPaneKind::ChangedFiles) {
        wait_for_view_idle(app, true).await;
    }
    if rendered.contains(&MuxPaneKind::Checkpoints) {
        wait_for_view_idle(app, false).await;
    }
}

/// Seed a session, enter mux with the given pane list, then close the
/// session so the grid runs with ZERO open sessions (the agent slots
/// are dropped). The slash-command submit path requires a current
/// session, so the session is seeded BEFORE `/mux on` and closed after
/// — the established pattern (BUG-165's `enter_mux_then_close_the_
/// session`).
async fn enter_mux_with_no_sessions(app: &mut App, panes: Vec<MuxPaneKind>) {
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(app).await;
    app.navigator_mut().mux.set_pane_list(panes, None);
    submit(app, "/mux on");
    drain_pending(app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    drain_pending(app).await;
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
}

/// Focus the mux pane of the given kind.
fn focus_pane(app: &mut App, kind: MuxPaneKind) {
    let panes = app.navigator().mux.effective_panes();
    let idx = panes
        .iter()
        .position(|k| *k == kind)
        .unwrap_or_else(|| panic!("pane kind {kind:?} not in effective panes {panes:?}"));
    app.navigator_mut().mux.set_focus(idx);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc on the focused Files mux pane closes the pane and keeps
// the saved layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc on the focused Files mux pane closes the pane and keeps the saved layout
#[tokio::test]
#[serial]
async fn esc_on_the_focused_files_mux_pane_closes_the_pane_and_keeps_the_saved_layout() {
    // The /mux off tail below triggers the mux-exit R6 auto-save (a
    // real write) — root the data dir for isolation.
    let (_data_guard, _data) = root_data_dir();
    // @step Given mux mode is active with Board, Agent, Files and Checkpoints panes
    let (mut app, _mock) = fresh_app();
    setup_mux_with_panes(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    )
    .await;

    // @step And the Files pane is focused and its list has loaded
    focus_pane(&mut app, MuxPaneKind::ChangedFiles);
    assert!(
        !app.navigator().changed_files.load.is_loading(),
        "the Files view must be loaded before Esc is honoured"
    );

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the grid shows Board, Agent and Checkpoints (the Files pane is gone)
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes,
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints
        ],
        "the Files pane must be removed from the grid, got {panes:?}"
    );

    // @step And the surviving panes rescale to absorb the Files pane's share of the width
    let rects = app.navigator().mux.pane_rects();
    assert_eq!(rects.len(), 3, "three pane rects expected");
    for (i, rect) in rects.iter().enumerate() {
        assert!(
            rect.width > 0,
            "pane {i} must have non-zero width, got {}",
            rect.width
        );
    }
    // Absorbed: the surviving panes + their dividers still tile the full
    // 120-wide body (the closed pane's share is not lost — the Checkpoints
    // pane's implicit last share grows).
    assert_eq!(
        rects.iter().map(|r| r.x + r.width).max().unwrap_or(0),
        120,
        "the surviving panes must absorb the closed pane's share"
    );

    // @step And the saved mux layout still lists the Files pane so a later /mux off then /mux on restores it
    assert!(
        app.mux_state()
            .config()
            .panes
            .contains(&MuxPaneKind::ChangedFiles),
        "the saved tui.mux pane list must be untouched by the live close"
    );
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints
        ],
        "/mux off then /mux on must restore the closed pane from the saved layout"
    );

    // @step And the TUI is still in mux mode with a surviving pane focused
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the mux must stay active after closing a pane"
    );
    let focus_idx = app.navigator().mux.focus();
    let focus_kind = app.navigator().mux.effective_panes()[focus_idx];
    assert_ne!(
        focus_kind,
        MuxPaneKind::ChangedFiles,
        "focus must not remain on the closed pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc on the focused Checkpoints mux pane closes the pane and
// keeps the saved layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc on the focused Checkpoints mux pane closes the pane and keeps the saved layout
#[tokio::test]
#[serial]
async fn esc_on_the_focused_checkpoints_mux_pane_closes_the_pane_and_keeps_the_saved_layout() {
    let (_data_guard, _data) = root_data_dir();
    // @step Given mux mode is active with Board, Agent and Checkpoints panes and one agent session is open
    let (mut app, _mock) = fresh_app();
    setup_mux_with_panes(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints,
        ],
    )
    .await;
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "one agent session must be open"
    );

    // @step And the Checkpoints pane is focused and its list has loaded
    focus_pane(&mut app, MuxPaneKind::Checkpoints);
    assert!(
        !app.navigator().checkpoints.load.is_loading(),
        "the Checkpoints view must be loaded before Esc is honoured"
    );

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the grid shows Board and Agent (the Checkpoints pane is gone)
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes,
        &[MuxPaneKind::Board, MuxPaneKind::Agent],
        "the Checkpoints pane must be removed from the grid, got {panes:?}"
    );

    // @step And the saved mux layout still lists the Checkpoints pane so a later /mux off then /mux on restores it
    assert!(
        app.mux_state()
            .config()
            .panes
            .contains(&MuxPaneKind::Checkpoints),
        "the saved tui.mux pane list must be untouched by the live close"
    );
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints
        ],
        "/mux off then /mux on must restore the closed pane from the saved layout"
    );

    // @step And the TUI is still in mux mode with a surviving pane focused
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the mux must stay active after closing the Checkpoints pane"
    );
    let focus_kind = app.navigator().mux.effective_panes()[app.navigator().mux.focus()];
    assert_ne!(
        focus_kind,
        MuxPaneKind::Checkpoints,
        "focus must not remain on the closed pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc on a Files pane mid-initial-load does not close the pane
// (R2 / TUI-107)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc on a Files pane mid-initial-load does not close the pane
#[tokio::test]
async fn esc_on_a_files_pane_mid_initial_load_does_not_close_the_pane() {
    // @step Given mux mode is active with Board and Files panes
    let (mut app, _mock) = fresh_app();
    // The slash-command submit path needs a current session.
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    app.navigator_mut()
        .mux
        .set_pane_list(vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles], None);
    submit(&mut app, "/mux on");
    // NOTE: no drain here — `/mux on` spawned the initial
    // `changed_files()` load (pending task) and the view is mid-load.
    assert_eq!(app.active_view(), ViewMode::Mux);

    // @step And the Files pane is focused and still in its initial load (the loading dialog is showing)
    focus_pane(&mut app, MuxPaneKind::ChangedFiles);
    assert!(
        app.navigator().changed_files.load.is_loading(),
        "the Files pane must still be in its initial load"
    );

    // @step When I press the Esc key
    let result = app.handle_event(&esc());
    // The view returned Ignored (TUI-107): the key fell through to the
    // App-level shortcuts exactly as today and nothing was consumed.
    assert!(
        !result.is_consumed(),
        "Esc mid-load must stay Ignored (TUI-107), not consumed"
    );

    // @step Then the Files pane is still in the grid (the pane does not close mid-load)
    assert!(
        app.navigator()
            .mux
            .effective_panes()
            .contains(&MuxPaneKind::ChangedFiles),
        "a reflex Esc mid-load must not close the pane"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the mux must stay active while the load is in flight"
    );

    // @step And the initial load completes un-impeded
    drain_pending(&mut app).await;
    assert!(
        !app.navigator().changed_files.load.is_loading(),
        "the initial load must complete after the reflex Esc"
    );
    assert!(
        app.navigator()
            .mux
            .effective_panes()
            .contains(&MuxPaneKind::ChangedFiles),
        "the pane must still be in the grid after the load completes"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing the last rendered pane exits mux to the single Board
// view (R3 degenerate case)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the last rendered pane exits mux to the single Board view
#[tokio::test]
#[serial]
async fn closing_the_last_rendered_pane_exits_mux_to_the_single_board_view() {
    // The exit triggers the mux-exit R6 auto-save (a real write) — root
    // the data dir for isolation.
    let (_data_guard, _data) = root_data_dir();
    // @step Given mux mode is active with Agent and Checkpoints panes and no agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_with_no_sessions(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Checkpoints]).await;
    assert_eq!(app.active_view(), ViewMode::Mux);

    // @step And only the Checkpoints pane renders (the agent slot is dropped) and its list has loaded
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Checkpoints],
        "with zero sessions the agent slot must be dropped"
    );
    focus_pane(&mut app, MuxPaneKind::Checkpoints);
    wait_for_view_idle(&mut app, false).await;

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the TUI leaves mux mode and shows the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "closing the last rendered pane must exit to the single Board view"
    );
    assert!(
        !app.navigator().mux.config().enabled,
        "the mux must be disabled on exit"
    );

    // @step And the saved mux layout is unchanged
    assert_eq!(
        app.mux_state().config().panes,
        vec![MuxPaneKind::Agent, MuxPaneKind::Checkpoints],
        "the saved tui.mux pane list must survive the exit"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Example 2: the transient close survives a session close and /mux
// off → /mux on restores the closed pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the Checkpoints pane after the last session closed restores via the saved layout
#[tokio::test]
#[serial]
async fn closing_the_checkpoints_pane_after_the_last_session_closed_restores_via_the_saved_layout()
{
    let (_data_guard, _data) = root_data_dir();
    // @step Given the mux grid shows Board, Agent and Checkpoints with one agent session open
    let (mut app, _mock) = fresh_app();
    setup_mux_with_panes(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints,
        ],
    )
    .await;
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints
        ],
    );

    // @step When I close the agent session (the agent slot is dropped from the live grid)
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board, MuxPaneKind::Checkpoints],
        "closing the session must drop the agent slot (same transience as the lazy-pane close)"
    );

    // @step And I focus the Checkpoints pane and press the Esc key
    focus_pane(&mut app, MuxPaneKind::Checkpoints);
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the Checkpoints pane is gone (the agent slot stays dropped — no session open)
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board],
        "the Checkpoints pane must be gone and the agent slot stays dropped, got {:?}",
        app.navigator().mux.effective_panes()
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the mux must stay active while a Board pane survives"
    );

    // @step And /mux off then /mux on brings the Checkpoints pane back from the saved layout
    // "Later" a session is open again (the slash-command path needs a
    // current session to submit) — the SAVED layout (which still lists
    // the Agent slot and the Checkpoints pane) is what restores.
    app.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    drain_pending(&mut app).await;
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints
        ],
        "the saved layout must bring back the agent slot AND the closed Checkpoints pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// R3 normal close: a lone full-width Board pane STAYS in Mux (user
// decision — do NOT auto-exit in that case)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the last Files pane of a Board+Files grid keeps a full-width Board pane in Mux
#[tokio::test]
async fn closing_the_last_files_pane_of_a_board_and_files_grid_keeps_a_full_width_board_pane() {
    // @step Given mux mode is active with Board and Files panes and no agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_with_no_sessions(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    )
    .await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);

    // @step And the Files pane is focused and its list has loaded
    focus_pane(&mut app, MuxPaneKind::ChangedFiles);
    wait_for_view_idle(&mut app, true).await;

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the grid shows the full-width Board pane in Mux (no auto-exit)
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board],
        "only the Board pane may remain"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "a lone Board pane must NOT auto-exit the mux (user decision)"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux must stay enabled with a lone Board pane"
    );

    // @step And the saved mux layout still lists the Files pane
    assert!(
        app.mux_state()
            .config()
            .panes
            .contains(&MuxPaneKind::ChangedFiles),
        "the saved tui.mux config must be untouched by the live close"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: focus clamps to the last surviving pane when the focused pane
// is removed from the end (R1 focus clamp)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: focus clamps to a surviving pane when the last pane is closed
#[tokio::test]
async fn focus_clamps_to_a_surviving_pane_when_the_last_pane_is_closed() {
    let (mut app, _mock) = fresh_app();
    setup_mux_with_panes(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
        ],
    )
    .await;

    // Focus the Files pane (index 2, the last pane).
    focus_pane(&mut app, MuxPaneKind::ChangedFiles);
    assert_eq!(app.navigator().mux.focus(), 2, "Files pane is at index 2");

    // Close the Files pane.
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // The grid now has 2 panes; focus must clamp to index 1 (Agent).
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(panes.len(), 2);
    assert!(
        !panes.contains(&MuxPaneKind::ChangedFiles),
        "Files pane must be gone"
    );
    let focus_kind = panes[app.navigator().mux.focus()];
    assert_eq!(
        focus_kind,
        MuxPaneKind::Agent,
        "focus must land on the Agent pane (index 1) after clamping"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// R4 regression guards: the pre-existing Esc semantics on the other
// panes / modes are untouched by the new pane-close path.
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc on the focused Board mux pane still shows the exit dialog
#[tokio::test]
async fn esc_on_the_focused_board_mux_pane_still_shows_the_exit_dialog() {
    // @step Given mux mode is active with Board and Files panes and no agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_with_no_sessions(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    )
    .await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
    // @step And the Board pane is focused
    // No sessions → agent slot is dropped; Board is the first pane.
    focus_pane(&mut app, MuxPaneKind::Board);

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the BoardExitConfirmationDialog is shown over the full screen
    assert!(
        app.compositor()
            .contains(codelet_fspec_tui::components::board_exit_confirmation_dialog::BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "Esc on the Board mux pane must push the BoardExitConfirmationDialog"
    );

    // @step And the TUI is still in mux mode with the same panes
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the mux must stay active after the exit dialog opens"
    );
    assert!(
        app.navigator()
            .mux
            .effective_panes()
            .contains(&MuxPaneKind::Board),
        "the Board pane must remain in the grid"
    );
}

/// Scenario: Esc on the focused Agent mux pane still shows the agent exit dialog
#[tokio::test]
async fn esc_on_the_focused_agent_mux_pane_still_shows_the_agent_exit_dialog() {
    // @step Given mux mode is active with Board and Agent panes and one agent session is open
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    app.navigator_mut()
        .mux
        .set_pane_list(vec![MuxPaneKind::Board, MuxPaneKind::Agent], None);
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the Agent pane is focused
    focus_pane(&mut app, MuxPaneKind::Agent);

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the agent exit confirmation dialog (Detach / Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(
            codelet_fspec_tui::components::exit_confirmation_dialog::EXIT_CONFIRMATION_DIALOG_ID
        ),
        "Esc on the Agent mux pane must open the agent exit confirmation dialog"
    );

    // @step And the Agent pane is still in the grid
    assert!(
        app.navigator()
            .mux
            .effective_panes()
            .contains(&MuxPaneKind::Agent),
        "the Agent pane must remain in the grid"
    );
}

/// Scenario: Esc in the single Changed Files view still closes to the Board
#[tokio::test]
async fn esc_in_the_single_changed_files_view_still_closes_to_the_board() {
    // @step Given the TUI is showing the single Changed Files view with mux inactive
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenChangedFilesView);
    drain_pending(&mut app).await;
    assert_eq!(
        app.active_view(),
        ViewMode::ChangedFiles,
        "the single ChangedFiles view must be active"
    );
    wait_for_view_idle(&mut app, true).await;

    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the TUI is back on the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Esc in the single ChangedFiles view must return to Board"
    );

    // @step And no mux footer row is painted
    assert!(
        !app.navigator().mux.config().enabled,
        "mux must be inactive for the single-view flip"
    );
}
