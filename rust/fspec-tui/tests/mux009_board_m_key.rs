//! MUX-009 — Board 'M' key opens the Mux config dialog + top-of-screen Mux hint.
//!
//! Feature: spec/features/board-m-key-opens-the-mux-config-dialog-top-of-screen-mux-hint.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
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
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the mux004/tui093 pattern). Returns the guard (held
/// for the test's duration) + the TempDir.
async fn root_data_dir(
) -> Result<(tokio::sync::MutexGuard<'static, ()>, TempDir), codelet_test_helpers::TestError> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = TempDir::new().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok((guard, tmp))
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Render the full App (navigator + compositor) into a 120x24 buffer
/// and return the flattened text (one row per line).
fn render_app_rows(app: &mut App) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf: &Buffer = term.backend().buffer();
    Ok((0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect())
}

/// Render the App and return the full buffer text (rows joined by '\n').
fn render_app_text(app: &mut App) -> Result<String, codelet_test_helpers::TestError> {
    Ok(render_app_rows(app)?.join("\n"))
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
async fn m_opens_the_mux_config_dialog_from_the_single_board_view() -> TestResult {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled, "mux must be disabled");
    // @step When I press the 'm' key
    let _ = app.handle_event(&key(KeyCode::Char('m'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is open overlaying the board
    assert_dialog_open(&app);
    // @step And the dialog is seeded from the live mux config showing Enabled Off and the current layout string
    let text = render_app_text(&mut app)?;
    assert!(
        text.contains("Enabled  Off"),
        "dialog must show Enabled Off; got:\n{text}"
    );
    assert!(
        text.contains("Layout  board, agent"),
        "dialog must show the live layout string; got:\n{text}"
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: Ctrl+M does not open the Mux config dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Ctrl+M does not open the Mux config dialog
#[tokio::test]
async fn ctrl_m_does_not_open_the_mux_config_dialog() -> TestResult {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I press Ctrl+M
    let _ = app.handle_event(&key(KeyCode::Char('m'), KeyModifiers::CONTROL));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is not open
    assert_dialog_closed(&app);
    // @step And the key falls through to the App-level handling
    // (no panic, no crash — the view is still Board)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must remain Board after Ctrl+M (no dialog, no crash)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: M opens the Mux config dialog from the focused Board pane in mux mode
// ─────────────────────────────────────────────────────────────────────────

/// Seed one open agent session into the app (required for the slash-command
/// path to have a current session to submit into).
async fn seed_session(app: &mut App, id: &str) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await?;
    Ok(())
}

/// Scenario: M opens the Mux config dialog from the focused Board pane in mux mode
#[tokio::test]
#[serial]
async fn m_opens_the_mux_config_dialog_from_the_focused_board_pane_in_mux_mode() -> TestResult {
    // 's' persists the committed layout to fspec-config.json — root the
    // data dir at a throwaway tempdir (mux004 pattern).
    let (_data_guard, _data) = root_data_dir().await?;
    // @step Given I am in mux mode with the Board | Agent grid and the Board pane focused
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // Enter mux mode via /mux on (Board pane focused on fresh entry —
    // the view the user came from, R9 fall-through rule).
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/mux on".to_string()));
    }
    drain_pending(&mut app).await?;
    assert_eq!(app.active_view(), ViewMode::Mux, "mux mode must be active");
    assert!(app.mux_state().config().enabled, "mux must be enabled");
    assert_eq!(
        app.mux_state().config().layout.to_string(),
        "board, agent",
        "default preset has two panes"
    );
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused on fresh /mux on entry"
    );
    // @step When I press the 'M' key
    let _ = app.handle_event(&key(KeyCode::Char('M'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is open overlaying the whole grid
    assert_dialog_open(&app);
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the view must remain Mux while the dialog is open"
    );
    // @step And the dialog shows the live enabled layout with Enabled On and the current layout string
    let text = render_app_text(&mut app)?;
    assert!(
        text.contains("Enabled  On"),
        "dialog must show Enabled On; got:\n{text}"
    );
    assert!(
        text.contains("Layout  board, agent"),
        "dialog must show the live layout string; got:\n{text}"
    );
    // @step And activating Save applies and persists the draft and closes the dialog with the grid keeping the committed layout
    // MUX-010: the dialog has no 's' shortcut any more ('s' types into
    // the layout field) — the Save button applies and persists. Shift+Tab
    // from the field wraps to Save; Enter activates it.
    let _ = app.handle_event(&key(KeyCode::BackTab, KeyModifiers::SHIFT));
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    let saved = std::fs::read_to_string(
        codelet_common::get_data_dir()
            .or_fail("data dir")?
            .join("fspec-config.json"),
    )
    .or_fail("Save must write fspec-config.json")?;
    let saved: serde_json::Value = serde_json::from_str(&saved).or_fail("parse")?;
    assert_eq!(
        saved["tui"]["mux"]["layout"],
        serde_json::json!("board, agent"),
        "Save must persist the committed layout: {saved}"
    );
    assert_dialog_closed(&app);
    // The grid keeps the committed layout (mux still active, 2 panes).
    assert!(
        app.mux_state().config().enabled,
        "mux must remain enabled after 's' save"
    );
    assert_eq!(
        app.mux_state().config().layout.to_string(),
        "board, agent",
        "the grid must keep its two panes after save"
    );
    // @step And pressing Esc closes the dialog without applying or saving
    // The Save re-entered the grid with focus on the Agent pane
    // (focused_pane: 1 from the saved layout), so move focus back to
    // the Board pane before re-opening the dialog for the Esc cancel test.
    let _ = app.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused again before re-opening"
    );
    // Re-open the dialog and press Esc.
    let _ = app.handle_event(&key(KeyCode::Char('M'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert_dialog_open(&app);
    let before = app.mux_state().config().clone();
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert_dialog_closed(&app);
    assert_eq!(
        app.mux_state().config(),
        &before,
        "the live config must be unchanged after Esc (cancel-safe)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: The Board header top chord row advertises the M Mux binding
// (BOARD-023 R10 supersession: the full chord is collapsed into the short
// 'u Actions' hint — the 'M Mux' entry now lives in the actions dialog)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The Board header top chord row advertises the M Mux binding
/// (BOARD-023 R10 superseded the full chord with the short 'u Actions'
/// hint; MENU-002 R1 supersedes that hint with the live 2-zone menu bar —
/// the 'M Mux' entry now lives in the Actions dropdown)
#[tokio::test]
async fn board_header_top_chord_row_advertises_m_mux_binding() -> TestResult {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I look at the board header's keybinding chord row
    let text = render_app_text(&mut app)?;
    // @step Then the header row 3 shows the 2-zone menu bar's Zone A items (Kanban Tools Settings Help)
    // (MENU-002 supersedes both the old chord and the 'u Actions' hint;
    // MENU-008 names the Zone A items. A fresh store has no session chips.)
    let hint_row = text
        .lines()
        .find(|l| l.contains("Kanban") && l.contains("Help"))
        .or_fail("the board header must contain the 2-zone bar's Zone A row")?;
    for item in ["Kanban", "Tools", "Settings", "Help"] {
        assert!(
            hint_row.contains(item),
            "Zone A must paint the {item:?} item; got: {hint_row}"
        );
    }
    // @step And the row contains no chord separators and no C Checkpoints chord segment
    assert!(
        !hint_row.contains('◆'),
        "the bar row must not paint chord separators; got: {hint_row}"
    );
    assert!(
        !text.contains("C Checkpoints"),
        "the old six-action chord must no longer render in the header; got:\n{text}"
    );
    Ok(())
}
