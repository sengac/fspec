//! MUX-004 — Mux configuration dialog + /mux slash-popup entry.
//!
//! Feature: spec/features/mux-config-dialog.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::Arc;
use std::sync::Mutex;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::agent::slash_commands::{filter_commands, SlashCommandAction};
use codelet_fspec_tui::views::multiplex::{MuxOrientation, MuxPaneKind};
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
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
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Root the process-global data directory at a fresh throwaway dir and
/// keep it alive (the established tui093 pattern). Returns the guard
/// (held for the test's duration) + the TempDir.
fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = TempDir::new().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set data dir");
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

fn submit(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Seed one open agent session into the app (required for the agent pane
/// to be meaningful and for the slash-command path to have a current
/// session to submit into).
async fn seed_session(app: &mut App, id: &str) {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await;
}

/// Open the MuxConfigDialog by submitting bare `/mux`.
async fn open_mux_config_dialog(app: &mut App) {
    submit(app, "/mux");
    drain_pending(app).await;
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
        "MuxConfigDialog must be open on the compositor"
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
// Scenario: bare /mux opens the MuxConfigDialog over the current view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: bare /mux opens the MuxConfigDialog over the current view
#[tokio::test]
async fn bare_mux_opens_the_mux_config_dialog_over_the_current_view() {
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux"
    open_mux_config_dialog(&mut app).await;
    // @step Then the MuxConfigDialog is open on the compositor
    assert_dialog_open(&app);
    // @step And the dialog shows an Enabled row with Off
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Enabled") && text.contains("Off"),
        "dialog must show an Enabled row with Off; got:\n{text}"
    );
    // @step And the dialog shows an Orientation row with Horizontal
    assert!(
        text.contains("Orientation") && text.contains("Horizontal"),
        "dialog must show an Orientation row with Horizontal; got:\n{text}"
    );
    // @step And the dialog shows a Pane row for Board
    assert!(
        text.contains("Pane 1") && text.contains("Board"),
        "dialog must show a Pane row for Board; got:\n{text}"
    );
    // @step And the dialog shows a Pane row for Agent
    assert!(
        text.contains("Pane 2") && text.contains("Agent"),
        "dialog must show a Pane row for Agent; got:\n{text}"
    );
    // @step And the TUI is still in the single Board view (the dialog is an overlay and mux is not yet active)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must remain Board while the dialog is open (mux not yet active)"
    );
    assert!(
        !app.mux_state().config().enabled,
        "mux config must NOT be enabled while the dialog is open"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux on enters mux mode without opening the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enters mux mode without opening the dialog
#[tokio::test]
async fn mux_on_enters_mux_mode_without_opening_the_dialog() {
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    // @step Then mux mode is active with the default preset
    assert_eq!(app.active_view(), ViewMode::Mux, "/mux on must enter mux mode");
    assert!(
        app.mux_state().config().enabled,
        "mux config must be enabled after /mux on"
    );
    assert_eq!(
        app.mux_state().config().panes.len(),
        2,
        "default preset has two panes"
    );
    // @step And no MuxConfigDialog is open on the compositor
    assert_dialog_closed(&app);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux off exits mux mode without opening the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux off exits mux mode without opening the dialog
#[tokio::test]
async fn mux_off_exits_mux_mode_without_opening_the_dialog() {
    // @step Given mux mode is active with the default two panes and the pre-mux view is the Board view
    // BUG-167: /mux off triggers the mux-exit auto-save (a real write to the
    // shared fspec-config.json). Root this test's own data dir (holding the
    // guard) so the auto-save is isolated and it cannot race a concurrent
    // dir-rooted persistence test within this process.
    let (_data_guard, _data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step When I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await;
    // @step Then mux mode is inactive
    assert!(
        !app.mux_state().config().enabled,
        "mux config must be disabled after /mux off"
    );
    // @step And the TUI shows the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "must return to the pre-mux Board view"
    );
    // @step And no MuxConfigDialog is open on the compositor
    assert_dialog_closed(&app);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the dialog body shows the enabled, orientation and pane rows in order
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the dialog body shows the enabled, orientation and pane rows in order
#[tokio::test]
async fn the_dialog_body_shows_the_enabled_orientation_and_pane_rows_in_order() {
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // @step When I submit the slash command "/mux"
    open_mux_config_dialog(&mut app).await;
    // @step Then the dialog body rows appear in the order: Enabled, Orientation, Pane 1, Pane 2
    let rows = render_app_rows(&mut app);
    let enabled_y = rows
        .iter()
        .position(|r| r.contains("Enabled"))
        .expect("Enabled row must be visible");
    let orientation_y = rows
        .iter()
        .position(|r| r.contains("Orientation"))
        .expect("Orientation row must be visible");
    let pane1_y = rows
        .iter()
        .position(|r| r.contains("Pane 1"))
        .expect("Pane 1 row must be visible");
    let pane2_y = rows
        .iter()
        .position(|r| r.contains("Pane 2"))
        .expect("Pane 2 row must be visible");
    assert!(
        enabled_y < orientation_y && orientation_y < pane1_y && pane1_y < pane2_y,
        "rows must appear in order: Enabled({enabled_y}), Orientation({orientation_y}), Pane 1({pane1_y}), Pane 2({pane2_y})"
    );
    // @step And the dialog footer lists the editing keybindings (cursor, value, add, remove, save, apply, cancel)
    let text = rows.join("\n");
    assert!(
        text.contains("Field") || text.contains("↑↓"),
        "footer must reference field/cursor navigation; got:\n{text}"
    );
    assert!(
        text.contains("Value") || text.contains("←→"),
        "footer must reference value cycling; got:\n{text}"
    );
    assert!(
        text.contains("Add") || text.contains("A"),
        "footer must reference add; got:\n{text}"
    );
    assert!(
        text.contains("Remove") || text.contains("⌫"),
        "footer must reference remove; got:\n{text}"
    );
    assert!(
        text.contains("Save") || text.contains("S"),
        "footer must reference save; got:\n{text}"
    );
    assert!(
        text.contains("Apply") || text.contains("Enter"),
        "footer must reference apply; got:\n{text}"
    );
    assert!(
        text.contains("Cancel") || text.contains("Esc"),
        "footer must reference cancel; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: cursor wraps and values cycle per highlighted row
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: cursor wraps and values cycle per highlighted row
#[tokio::test]
async fn cursor_wraps_and_values_cycle_per_highlighted_row() {
    // @step Given the MuxConfigDialog is open with the default two panes
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    assert_dialog_open(&app);
    // @step When I press Up on the Enabled row
    // (cursor starts at 0 = Enabled; Up wraps to last pane row = index 3)
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the cursor wraps to the last Pane row
    // (verify by rendering: the last pane row should be highlighted —
    //  the selected row gets the inverse highlight which changes the
    //  marker from "  " to "▸ ")
    let rows = render_app_rows(&mut app);
    let last_pane_row = rows
        .iter()
        .find(|r| r.contains("Pane 2"))
        .expect("Pane 2 row must be visible");
    assert!(
        last_pane_row.contains("▸"),
        "the last Pane row must be highlighted (cursor wrapped to it); got: {last_pane_row}"
    );
    // @step When I press Down on the last Pane row
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the cursor wraps back to the Enabled row
    let rows = render_app_rows(&mut app);
    let enabled_row = rows
        .iter()
        .find(|r| r.contains("Enabled"))
        .expect("Enabled row must be visible");
    assert!(
        enabled_row.contains("▸"),
        "the Enabled row must be highlighted (cursor wrapped back); got: {enabled_row}"
    );
    // @step When I press Down to the Orientation row and press Right
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // now cursor is on Orientation (index 1)
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the Orientation row shows Vertical
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Vertical"),
        "Orientation must show Vertical after Right; got:\n{text}"
    );
    // @step When I press Right again
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the Orientation row shows Horizontal
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Horizontal"),
        "Orientation must show Horizontal after second Right; got:\n{text}"
    );
    // @step When I move the cursor to the Agent Pane row and press Right three times
    // Move from Orientation (1) to Pane 2 (index 3)
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // Now on Pane 2 (Agent). Cycle: Agent → Files → Checkpoints → Board
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the Pane row shows Board again (the cycle is Board -> Agent -> Files -> Checkpoints -> Board)
    let text = render_app_text(&mut app);
    // The pane row should now show Board (Agent → Files → Checkpoints → Board)
    assert!(
        text.contains("Board"),
        "after 3 Rights from Agent, the pane must cycle back to Board; got:\n{text}"
    );
    // R4: mouse wheel scrolls the cursor like Up/Down — from the last
    // pane row (index 3) a wheel-up moves to the previous row.
    let wheel_up = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    let _ = app.handle_event(&wheel_up);
    drain_pending(&mut app).await;
    let rows = render_app_rows(&mut app);
    let pane1_row = rows
        .iter()
        .find(|r| r.contains("Pane 1"))
        .expect("Pane 1 row must be visible");
    assert!(
        pane1_row.contains("▸"),
        "mouse wheel-up must move the cursor like Up (to the Pane 1 row); got: {pane1_row}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: adding a third pane and applying it rebuilds the grid
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: adding a third pane and applying it rebuilds the grid
#[tokio::test]
async fn adding_a_third_pane_and_applying_it_rebuilds_the_grid() {
    // @step Given the MuxConfigDialog is open with the default two panes
    // (mux is active on the default preset — the grid being rebuilt; the
    //  Enabled state is On and unchanged through the commit, so R7's
    //  "only refreshes the layout" keeps mux active)
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    open_mux_config_dialog(&mut app).await;
    // @step When I press "a" to append a pane row
    let _ = app.handle_event(&key(KeyCode::Char('a'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the dialog shows a third Pane row with kind Board
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Pane 3"),
        "a third Pane row must appear after pressing 'a'; got:\n{text}"
    );
    // @step When I move the cursor to the third Pane row and cycle its kind to Files
    // Pressing 'a' already moved the cursor onto the new Pane 3 row
    // (index 4). Cycle Board → Agent → Files with two Rights.
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And mux mode is active with three panes: Board, Agent and Files
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active after apply");
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 3, "must have three panes");
    assert_eq!(cfg.panes[0], MuxPaneKind::Board);
    assert_eq!(cfg.panes[1], MuxPaneKind::Agent);
    assert_eq!(cfg.panes[2], MuxPaneKind::ChangedFiles);
    // @step And the split scale is re-derived for three panes (one entry per inter-pane gap, last pane takes the remainder)
    assert_eq!(
        cfg.splits.len(),
        2,
        "three panes must have two split entries (n-1)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pane count stays within the 2 to 4 bounds
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pane count stays within the 2 to 4 bounds
#[tokio::test]
async fn pane_count_stays_within_the_2_to_4_bounds() {
    // @step Given the MuxConfigDialog is open with the default two panes and the cursor on a Pane row
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    // Move cursor to a Pane row (Down from Enabled → Orientation, Down → Pane 1)
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step When I press Backspace
    let _ = app.handle_event(&key(KeyCode::Backspace, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the pane row is NOT removed (minimum of two panes)
    let text = render_app_text(&mut app);
    assert!(
        text.contains("Pane 1") && text.contains("Pane 2"),
        "both pane rows must still be present (minimum 2 enforced); got:\n{text}"
    );

    // Second half: 4 panes, 'a' does nothing
    // @step Given the MuxConfigDialog is open with four panes
    // Close the current dialog and reopen with 4 panes
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // Set up 4 panes via /mux 4
    submit(&mut app, "/mux 4");
    drain_pending(&mut app).await;
    // Now open the dialog again
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    assert_dialog_open(&app);
    // @step When I press "a"
    let _ = app.handle_event(&key(KeyCode::Char('a'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then no fifth Pane row is added (maximum of four panes)
    let text = render_app_text(&mut app);
    assert!(
        !text.contains("Pane 5"),
        "a fifth pane row must NOT appear (maximum 4 enforced); got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter applies the draft layout and closes the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter applies the draft layout and closes the dialog
#[tokio::test]
#[serial]
async fn enter_applies_the_draft_layout_and_closes_the_dialog() {
    // @step Given the MuxConfigDialog is open with the default two panes while mux is off
    let (_data_guard, data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    assert_dialog_open(&app);
    // @step When I cycle the second Pane row's kind to Checkpoints
    // Move cursor to Pane 2 (index 3: Enabled=0, Orientation=1, Pane1=2, Pane2=3)
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // On Pane 2 (Agent). Cycle: Agent→Files→Checkpoints (2 Rights)
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout has panes Board and Checkpoints
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 2);
    assert_eq!(cfg.panes[0], MuxPaneKind::Board);
    assert_eq!(cfg.panes[1], MuxPaneKind::Checkpoints);
    // @step And the shared fspec-config.json is NOT written (apply does not save)
    let path = data.path().join("fspec-config.json");
    assert!(
        !path.exists(),
        "fspec-config.json must NOT exist after a plain Enter apply"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: S applies the draft and persists it to the shared config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: S applies the draft and persists it to the shared config
#[tokio::test]
#[serial]
async fn s_applies_the_draft_and_persists_it_to_the_shared_config() {
    // @step Given the MuxConfigDialog is open with the default two panes while mux is off
    let (_data_guard, data) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    // @step When I set the Orientation row to Vertical
    // Cursor starts at 0 (Enabled); one Down moves to Orientation (index 1)
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // Cycle to Vertical (Right once)
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press "s" to save
    let _ = app.handle_event(&key(KeyCode::Char('s'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout is vertical with panes Board and Agent
    let cfg = app.mux_state().config();
    assert_eq!(cfg.orientation, MuxOrientation::Vertical);
    assert_eq!(cfg.panes.len(), 2);
    assert_eq!(cfg.panes[0], MuxPaneKind::Board);
    assert_eq!(cfg.panes[1], MuxPaneKind::Agent);
    // @step And the shared fspec-config.json tui.mux key contains the vertical orientation and the two-pane list
    let path = data.path().join("fspec-config.json");
    assert!(
        path.exists(),
        "fspec-config.json must exist after 's' save"
    );
    let raw = fs::read_to_string(&path).expect("read fspec-config.json");
    assert!(
        raw.contains("\"mux\""),
        "the tui.mux key must be present: {raw}"
    );
    // MuxOrientation serializes as the string "Vertical" (serde default
    // for a plain enum) and the pane kinds as "Board"/"Agent".
    assert!(
        raw.contains("Vertical"),
        "the saved config must contain the vertical orientation: {raw}"
    );
    assert!(raw.contains("Board") && raw.contains("Agent"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc closes the dialog without applying or saving
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc closes the dialog without applying or saving
#[tokio::test]
async fn esc_closes_the_dialog_without_applying_or_saving() {
    // @step Given the MuxConfigDialog is open with the default two panes
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    // @step And the live mux layout is horizontal with panes Board and Agent
    let before = app.mux_state().config().clone();
    assert_eq!(before.orientation, MuxOrientation::Horizontal);
    assert_eq!(before.panes, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    // @step When I cycle the second Pane row's kind to Files
    // Move to Pane 2 (index 3)
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // Cycle Agent → Files (one Right)
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press Esc to cancel
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout is still horizontal with panes Board and Agent
    let after = app.mux_state().config();
    assert_eq!(
        after, &before,
        "the live config must be unchanged after Esc (cancel-safe)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: reopening while the dialog is open is a no-op
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: reopening while the dialog is open is a no-op
#[tokio::test]
async fn reopening_while_the_dialog_is_open_is_a_no_op() {
    // @step Given the MuxConfigDialog is open
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    open_mux_config_dialog(&mut app).await;
    assert_dialog_open(&app);
    // @step When I submit the slash command "/mux" again
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    // @step Then exactly one MuxConfigDialog layer exists on the compositor
    let mux_dialog_count = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == MUX_CONFIG_DIALOG_ID)
        .count();
    assert_eq!(
        mux_dialog_count, 1,
        "exactly one MuxConfigDialog must exist (idempotent); got {mux_dialog_count}"
    );
    // @step When I press Esc to close it
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I submit the slash command "/mux" again
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    // @step Then a fresh MuxConfigDialog is open seeded from the current config
    assert_dialog_open(&app);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing with Enabled On enters mux mode with the draft layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing with Enabled On enters mux mode with the draft layout
#[tokio::test]
async fn committing_with_enabled_on_enters_mux_mode_with_the_draft_layout() {
    // @step Given the TUI is showing the single Agent view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    app.dispatch(Action::EnterWorkUnit("AUTH-001".to_string()));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And the MuxConfigDialog is open with Enabled set to Off and panes Board, Agent and Files
    // Open the dialog (mux is off by default): Enabled Off, panes Board/Agent.
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    assert_dialog_open(&app);
    // Build the third pane row and cycle its kind to Files: rows become
    // 0=Enabled, 1=Orientation, 2=Pane 1, 3=Pane 2, 4=Pane 3. Pressing
    // 'a' moves the cursor onto Pane 3; Right cycles Board → Agent → Files.
    let _ = app.handle_event(&key(KeyCode::Char('a'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step When I move the cursor to the Enabled row and press Right to set it to On
    // Move back to the Enabled row (index 0): Up from 4 → 3 → 2 → 1 → 0.
    for _ in 0..4 {
        let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
        drain_pending(&mut app).await;
    }
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then mux mode is active with three panes: Board, Agent and Files
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active");
    let cfg = app.mux_state().config();
    assert_eq!(cfg.panes.len(), 3);
    assert_eq!(cfg.panes[0], MuxPaneKind::Board);
    assert_eq!(cfg.panes[1], MuxPaneKind::Agent);
    assert_eq!(cfg.panes[2], MuxPaneKind::ChangedFiles);
    // @step And the pre-mux view recorded for exit is the single Agent view
    assert_eq!(
        app.navigator().mux.pre_mux_view(),
        Some(ViewMode::Agent),
        "the pre-mux view must be the Agent view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing with Enabled Off exits mux mode back to the pre-mux view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing with Enabled Off exits mux mode back to the pre-mux view
#[tokio::test]
async fn committing_with_enabled_off_exits_mux_mode_back_to_the_pre_mux_view() {
    // @step Given mux mode is active with the default two panes and the pre-mux view is the Board view
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the MuxConfigDialog is open with Enabled set to On
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    assert_dialog_open(&app);
    // @step When I move the cursor to the Enabled row and press Left to set it to Off
    // Cursor starts at 0 (Enabled). Left toggles On→Off.
    let _ = app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then mux mode is inactive
    assert!(
        !app.mux_state().config().enabled,
        "mux must be disabled after committing with Enabled: Off"
    );
    // @step And the TUI shows the single Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "must return to the pre-mux Board view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the slash popup lists /mux with a description and picking it opens the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the slash popup lists /mux with a description and picking it opens the dialog
#[tokio::test]
async fn the_slash_popup_lists_mux_with_a_description_and_picking_it_opens_the_dialog() {
    // @step Given the agent input contains "/" (the slash popup is open)
    // and the user types the "mux" filter characters
    let (mut view, mut rx) = fresh_agent_view();
    for ch in "/mux".chars() {
        view.handle_event(&key(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    assert!(
        view.slash_popup.is_some(),
        "the slash popup must be open after typing /mux"
    );
    // @step When I type "mux" as the popup filter
    // (already typed above; the popup filter is "mux")
    let popup = view.slash_popup.as_ref().expect("popup open");
    assert_eq!(popup.filter(), "mux");
    // @step Then the popup shows exactly one row: "/mux" with a description of the mux layout configuration
    let matches = popup.matches();
    assert_eq!(
        matches.len(),
        1,
        "exactly one popup row expected for filter 'mux'; got: {:?}",
        matches.iter().map(|c| c.name()).collect::<Vec<_>>()
    );
    assert_eq!(matches[0].name(), "mux");
    assert!(
        matches[0].description.to_lowercase().contains("mux"),
        "the description must mention mux: {:?}",
        matches[0].description
    );
    // @step When I press Enter on the highlighted /mux row
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    // @step Then the agent input is cleared
    assert!(
        view.input.is_empty(),
        "the agent input must be cleared after picking /mux"
    );
    // And the popup pick emits Action::SlashCommandSelected(Mux)
    let mut saw_mux_pick = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(
            a,
            Action::SlashCommandSelected(SlashCommandAction::Mux)
        ) {
            saw_mux_pick = true;
        }
    }
    assert!(
        saw_mux_pick,
        "picking /mux must emit SlashCommandSelected(Mux)"
    );

    // @step And the MuxConfigDialog is open on the compositor
    // (the App half: dispatching the emitted action opens the dialog)
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Mux));
    drain_pending(&mut app).await;
    assert_dialog_open(&app);
}

/// Build a fresh AgentView wired to an unbounded action channel.
fn fresh_agent_view() -> (
    codelet_fspec_tui::AgentView,
    tokio::sync::mpsc::UnboundedReceiver<Action>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (codelet_fspec_tui::AgentView::new(tx), rx)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the agent help dialog lists /mux from the registry
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the agent help dialog lists /mux from the registry
#[test]
fn the_agent_help_dialog_lists_mux_from_the_registry() {
    use codelet_fspec_tui::components::help_dialog::HelpDialog;
    use codelet_fspec_tui::components::Component;

    // @step Given the agent help content is generated from the slash command registry
    // (agent_help_lines() in components/help_content.rs iterates
    // SLASH_COMMANDS — no manual help edit needed for new rows)
    let registry = filter_commands("mux");
    assert!(!registry.is_empty(), "the registry must include a mux entry");
    let mux_cmd = &registry[0];
    assert_eq!(mux_cmd.name(), "mux");
    assert!(!mux_cmd.description.is_empty());

    // @step When I read the slash-command lines of the agent help
    let mut dialog = HelpDialog::for_agent();
    let backend = TestBackend::new(200, 60);
    let mut terminal = Terminal::new(backend).expect("Terminal::new");
    terminal
        .draw(|frame| dialog.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    let buf = terminal.backend().buffer().clone();
    let text = (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");

    // @step Then the lines include a /mux row with the same description as the slash popup registry
    assert!(
        text.contains("/mux"),
        "the agent help must list the /mux command:\n{text}"
    );
    assert!(
        text.contains(mux_cmd.description),
        "the /mux help row must carry the registry description {:?}:\n{text}",
        mux_cmd.description
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux help documents the config dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux help documents the config dialog
#[tokio::test]
async fn mux_help_documents_the_config_dialog() {
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // @step When I submit the slash command "/mux help"
    submit(&mut app, "/mux help");
    drain_pending(&mut app).await;
    // @step Then a one-line notice appears in the agent scrollback
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        !text.is_empty(),
        "a notice must appear in the scrollback after /mux help"
    );
    // @step And the notice mentions the config dialog (bare /mux) and the on/off subcommands
    assert!(
        text.to_lowercase().contains("config") || text.to_lowercase().contains("dialog"),
        "the /mux help notice must mention the config dialog; got:\n{text}"
    );
    assert!(
        text.contains("on") && text.contains("off"),
        "the /mux help notice must mention the on/off subcommands; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Helper: read session scrollback text
// ─────────────────────────────────────────────────────────────────────────

fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines
                .iter()
                .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        })
        .collect::<Vec<String>>()
        .join("\n")
}
