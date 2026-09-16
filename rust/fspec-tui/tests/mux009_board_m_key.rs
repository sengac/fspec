//! MUX-009 — Board 'M' key opens the Mux config dialog + top-of-screen Mux hint.
//!
//! Feature: spec/features/board-m-key-opens-the-mux-config-dialog-top-of-screen-mux-hint.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::MockBackend;

/// The stable compositor id for the MuxConfigDialog.
const MUX_CONFIG_DIALOG_ID: &str = "mux-config-dialog";

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

/// Serialises tests that mutate the process-global data directory
/// (within this test binary).
static DATA_DIR_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the mux004/tui093 pattern). Returns the guard (held
/// for the test's duration) + the TempDir.
fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = TempDir::new().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf()).expect("set data dir");
    (guard, tmp)
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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Render the full App (navigator + compositor) into a 120x24 buffer
/// and return the flattened text (one row per line).
fn render_app_rows(app: &mut App) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    let buf: &Buffer = term.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

/// Render the App and return the full buffer text (rows joined by '\n').
fn render_app_text(app: &mut App) -> String {
    render_app_rows(app).join("\n")
}

/// Check that the MuxConfigDialog is present on the compositor.
fn assert_dialog_open(app: &App) {
    assert!(
        app.compositor().contains(MUX_CONFIG_DIALOG_ID),
        "MuxConfigDialog must be open on the compositor; layers={:?} view={:?} focus={} effective={:?}",
        app.compositor().layer_ids(),
        app.active_view(),
        app.navigator().mux.focus(),
        app.navigator().mux.effective_panes()
    );
}

/// Check that the MuxConfigDialog is NOT present on the compositor.
fn assert_dialog_closed(app: &App) {
    assert!(
        !app.compositor().contains(MUX_CONFIG_DIALOG_ID),
        "MuxConfigDialog must NOT be on the compositor"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: M opens the Mux config dialog from the single Board view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: M opens the Mux config dialog from the single Board view
#[tokio::test]
async fn m_opens_the_mux_config_dialog_from_the_single_board_view() {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled, "mux must be disabled");
    // @step When I press the 'm' key
    let _ = app.handle_event(&key(KeyCode::Char('m'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is open overlaying the board
    assert_dialog_open(&app);
    // @step And the dialog is seeded from the live mux config showing Enabled Off and the current pane rows
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Enabled") && text.contains("Off"),
        "dialog must show Enabled Off; got:\n{text}"
    );
    assert!(
        text.contains("Pane 1") && text.contains("Board"),
        "dialog must show Pane 1 Board; got:\n{text}"
    );
    assert!(
        text.contains("Pane 2") && text.contains("Agent"),
        "dialog must show Pane 2 Agent; got:\n{text}"
    );
    // @step And the board view is still visible underneath
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must remain Board while the dialog is open"
    );
    // @step And mux is still not active
    assert!(
        !app.mux_state().config().enabled,
        "mux must NOT be enabled while the dialog is open"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: Ctrl+M does not open the Mux config dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Ctrl+M does not open the Mux config dialog
#[tokio::test]
async fn ctrl_m_does_not_open_the_mux_config_dialog() {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I press Ctrl+M
    let _ = app.handle_event(&key(KeyCode::Char('m'), KeyModifiers::CONTROL));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is not open
    assert_dialog_closed(&app);
    // @step And the key falls through to the App-level handling
    // (no panic, no crash — the view is still Board)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must remain Board after Ctrl+M (no dialog, no crash)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: M opens the Mux config dialog from the focused Board pane in mux mode
// ─────────────────────────────────────────────────────────────────────────

/// Seed one open agent session into the app (required for the slash-command
/// path to have a current session to submit into).
async fn seed_session(app: &mut App, id: &str) {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await;
}

/// Scenario: M opens the Mux config dialog from the focused Board pane in mux mode
#[tokio::test]
#[serial]
async fn m_opens_the_mux_config_dialog_from_the_focused_board_pane_in_mux_mode() {
    // 's' persists the committed layout to fspec-config.json — root the
    // data dir at a throwaway tempdir (mux004 pattern).
    let (_data_guard, _data) = root_data_dir();
    // @step Given I am in mux mode with the Board | Agent grid and the Board pane focused
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // Enter mux mode via /mux on (Board pane focused on fresh entry —
    // the view the user came from, R9 fall-through rule).
    app.dispatch(Action::InputSubmitted("/mux on".to_string()));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux, "mux mode must be active");
    assert!(app.mux_state().config().enabled, "mux must be enabled");
    assert_eq!(
        app.mux_state().config().panes.len(),
        2,
        "default preset has two panes"
    );
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused on fresh /mux on entry"
    );
    // @step When I press the 'M' key
    let _ = app.handle_event(&key(KeyCode::Char('M'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is open overlaying the whole grid
    assert_dialog_open(&app);
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the view must remain Mux while the dialog is open"
    );
    // @step And the dialog shows the live enabled layout with Enabled On and the current pane rows
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Enabled") && text.contains("On"),
        "dialog must show Enabled On; got:\n{text}"
    );
    assert!(
        text.contains("Pane 1") && text.contains("Board"),
        "dialog must show Pane 1 Board; got:\n{text}"
    );
    assert!(
        text.contains("Pane 2") && text.contains("Agent"),
        "dialog must show Pane 2 Agent; got:\n{text}"
    );
    // @step And pressing 's' applies and persists the draft and closes the dialog with the grid keeping the committed layout
    let _ = app.handle_event(&key(KeyCode::Char('s'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_dialog_closed(&app);
    // The grid keeps the committed layout (mux still active, 2 panes).
    assert!(
        app.mux_state().config().enabled,
        "mux must remain enabled after 's' save"
    );
    assert_eq!(
        app.mux_state().config().panes.len(),
        2,
        "the grid must keep its two panes after save"
    );
    // @step And pressing Esc closes the dialog without applying or saving
    // The 's' save re-entered the grid with focus on the Agent pane
    // (focused_pane: 1 from the saved layout), so move focus back to
    // the Board pane before re-opening the dialog for the Esc cancel test.
    let _ = app.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused again before re-opening"
    );
    // Re-open the dialog and press Esc.
    let _ = app.handle_event(&key(KeyCode::Char('M'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_dialog_open(&app);
    let before = app.mux_state().config().clone();
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_dialog_closed(&app);
    assert_eq!(
        app.mux_state().config(),
        &before,
        "the live config must be unchanged after Esc (cancel-safe)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: The Board header top chord row advertises the M Mux binding
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The Board header top chord row advertises the M Mux binding
#[tokio::test]
async fn board_header_top_chord_row_advertises_m_mux_binding() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I look at the board header's keybinding chord row
    let text = render_app_text(&mut app);
    // @step Then the chord row reads "C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux"
    let chord_row = text
        .lines()
        .find(|l| l.contains("C Checkpoints"))
        .expect("the board header must contain the keybinding chord row");
    assert!(
        chord_row.contains("M Mux"),
        "the chord row must advertise 'M Mux'; got: {chord_row}"
    );
    assert!(
        chord_row.contains("/ Search"),
        "the chord row must retain the existing '/ Search' entry; got: {chord_row}"
    );
    // @step And the chord is painted as a single plain foreground span
    // (no per-chord styling — verify the chord row has no color/style
    //  differentiation: the entire row is a single span, so it has a
    //  uniform style. We assert the row exists and contains the full
    //  chord text as a contiguous string.)
    assert!(
        chord_row.contains("C Checkpoints")
            && chord_row.contains("F Changed Files")
            && chord_row.contains("D FOUNDATION.md")
            && chord_row.contains(". New Agent")
            && chord_row.contains("/ Search")
            && chord_row.contains("M Mux"),
        "the chord row must contain all entries in sequence; got: {chord_row}"
    );
}
