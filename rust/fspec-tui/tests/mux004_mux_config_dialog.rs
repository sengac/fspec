//! MUX-004 — Mux configuration dialog + /mux slash-popup entry.
//!
//! Feature: spec/features/mux-config-dialog.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::agent::slash_commands::{filter_commands, SlashCommandAction};
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
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
/// keep it alive (the established tui093 pattern). Returns the guard
/// (held for the test's duration) + the TempDir.
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

fn submit(app: &mut App, text: &str) {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, text.to_string()));
    }
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Seed one open agent session into the app (required for the agent pane
/// to be meaningful and for the slash-command path to have a current
/// session to submit into).
async fn seed_session(app: &mut App, id: &str) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await?;
    Ok(())
}

/// Open the MuxConfigDialog by submitting bare `/mux`.
async fn open_mux_config_dialog(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    submit(app, "/mux");
    drain_pending(app).await?;
    Ok(())
}

/// MUX-010: replace the MuxConfigDialog's layout field text (the field
/// has the focus when the dialog opens): End, Backspace until empty,
/// then type `text`.
fn replace_layout_field(app: &mut App, text: &str) {
    let _ = app.handle_event(&key(KeyCode::End, KeyModifiers::NONE));
    for _ in 0..64 {
        let _ = app.handle_event(&key(KeyCode::Backspace, KeyModifiers::NONE));
    }
    for c in text.chars() {
        let _ = app.handle_event(&key(KeyCode::Char(c), KeyModifiers::NONE));
    }
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
async fn bare_mux_opens_the_mux_config_dialog_over_the_current_view() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux"
    open_mux_config_dialog(&mut app).await?;
    // @step Then the MuxConfigDialog is open on the compositor
    assert_dialog_open(&app);
    // @step And the dialog shows an Enabled row with Off
    let rows = render_app_rows(&mut app)?;
    let text = rows.join("\n");
    assert!(
        text.contains("Enabled  Off"),
        "dialog must show an Enabled row with Off; got:\n{text}"
    );
    // @step And the dialog shows the layout field with "board, agent"
    let field_y = rows
        .iter()
        .position(|r| r.contains("Layout  board, agent"))
        .or_fail("the layout field must show \"board, agent\"")?;
    // @step And the dialog preview shows a box labelled board
    let preview: Vec<&String> = rows
        .iter()
        .skip(field_y + 1)
        .take_while(|r| !r.contains("Enabled"))
        .collect();
    assert!(
        preview.iter().any(|r| r.contains("board")),
        "the preview must show a box labelled board; got:\n{text}"
    );
    // @step And the dialog preview shows a box labelled agent
    assert!(
        preview.iter().any(|r| r.contains("agent")),
        "the preview must show a box labelled agent; got:\n{text}"
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux on enters mux mode without opening the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux on enters mux mode without opening the dialog
#[tokio::test]
async fn mux_on_enters_mux_mode_without_opening_the_dialog() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await?;
    // @step Then mux mode is active with the default preset
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "/mux on must enter mux mode"
    );
    assert!(
        app.mux_state().config().enabled,
        "mux config must be enabled after /mux on"
    );
    assert_eq!(
        app.mux_state().config().layout.to_string(),
        "board, agent",
        "the default preset is the board, agent layout"
    );
    // @step And no MuxConfigDialog is open on the compositor
    assert_dialog_closed(&app);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux off exits mux mode without opening the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux off exits mux mode without opening the dialog
#[tokio::test]
async fn mux_off_exits_mux_mode_without_opening_the_dialog() -> TestResult {
    // @step Given mux mode is active with the default two panes and the pre-mux view is the Board view
    // BUG-167: /mux off triggers the mux-exit auto-save (a real write to the
    // shared fspec-config.json). Root this test's own data dir (holding the
    // guard) so the auto-save is isolated and it cannot race a concurrent
    // dir-rooted persistence test within this process.
    let (_data_guard, _data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await?;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step When I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter applies the draft layout and closes the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter applies the draft layout and closes the dialog
#[tokio::test]
#[serial]
async fn enter_applies_the_draft_layout_and_closes_the_dialog() -> TestResult {
    // @step Given the MuxConfigDialog is open with the default two panes while mux is off
    let (_data_guard, data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    open_mux_config_dialog(&mut app).await?;
    assert_dialog_open(&app);
    assert!(!app.mux_state().config().enabled);
    // @step When I replace the layout field text with "board, checkpoints"
    replace_layout_field(&mut app, "board, checkpoints");
    drain_pending(&mut app).await?;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout has panes Board and Checkpoints
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "board, checkpoints");
    assert_eq!(
        cfg.layout.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Checkpoints]
    );
    // @step And the shared fspec-config.json is NOT written (apply does not save)
    // BUG-448: what apply must not do is save the mux layout; a task left
    // over from an earlier test may write unrelated keys into the shared
    // data dir, so the check is on `tui.mux`, not the file's existence.
    let path = data.path().join("fspec-config.json");
    if path.exists() {
        let saved: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).or_fail("read fspec-config.json")?,
        )
        .or_fail("fspec-config.json is JSON")?;
        assert!(
            saved["tui"]["mux"].is_null(),
            "a plain Enter apply must not save the mux layout: {saved}"
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Save applies the draft and persists it to the shared config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Save applies the draft and persists it to the shared config
#[tokio::test]
#[serial]
async fn save_applies_the_draft_and_persists_it_to_the_shared_config() -> TestResult {
    // @step Given the MuxConfigDialog is open with the default two panes while mux is off
    let (_data_guard, data) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    open_mux_config_dialog(&mut app).await?;
    assert_dialog_open(&app);
    // @step When I replace the layout field text with "[board, agent]"
    replace_layout_field(&mut app, "[board, agent]");
    drain_pending(&mut app).await?;
    // @step And I activate the Save button
    // (Shift+Tab from the field wraps to Save; Enter activates it)
    let _ = app.handle_event(&key(KeyCode::BackTab, KeyModifiers::SHIFT));
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout is "[board, agent]": Board over Agent in one column
    let cfg = app.navigator().mux.config();
    assert_eq!(cfg.layout.to_string(), "[board, agent]");
    assert_eq!(cfg.layout.items().len(), 1, "one column");
    assert_eq!(
        cfg.layout.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent],
        "Board over Agent"
    );
    // @step And the shared fspec-config.json tui.mux key contains the layout string "[board, agent]"
    let path = data.path().join("fspec-config.json");
    assert!(path.exists(), "fspec-config.json must exist after Save");
    let raw = fs::read_to_string(&path).or_fail("read fspec-config.json")?;
    let saved: serde_json::Value =
        serde_json::from_str(&raw).or_fail("fspec-config.json is JSON")?;
    assert_eq!(
        saved["tui"]["mux"]["layout"],
        serde_json::json!("[board, agent]"),
        "the saved config must contain the layout string: {raw}"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc closes the dialog without applying or saving
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc closes the dialog without applying or saving
#[tokio::test]
async fn esc_closes_the_dialog_without_applying_or_saving() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the MuxConfigDialog is open with the default two panes
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    open_mux_config_dialog(&mut app).await?;
    assert_dialog_open(&app);
    // @step And the live mux layout is "board, agent"
    let before = app.mux_state().config().clone();
    assert_eq!(before.layout.to_string(), "board, agent");
    // @step When I replace the layout field text with "board, files"
    replace_layout_field(&mut app, "board, files");
    drain_pending(&mut app).await?;
    // @step And I press Esc to cancel
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is closed
    assert_dialog_closed(&app);
    // @step And the live mux layout is still "board, agent"
    let after = app.mux_state().config();
    assert_eq!(
        after, &before,
        "the live config must be unchanged after Esc (cancel-safe)"
    );
    assert_eq!(after.layout.to_string(), "board, agent");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: reopening while the dialog is open is a no-op
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: reopening while the dialog is open is a no-op
#[tokio::test]
async fn reopening_while_the_dialog_is_open_is_a_no_op() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the MuxConfigDialog is open
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    open_mux_config_dialog(&mut app).await?;
    assert_dialog_open(&app);
    // @step When I submit the slash command "/mux" again
    submit(&mut app, "/mux");
    drain_pending(&mut app).await?;
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
    drain_pending(&mut app).await?;
    // @step And I submit the slash command "/mux" again
    submit(&mut app, "/mux");
    drain_pending(&mut app).await?;
    // @step Then a fresh MuxConfigDialog is open seeded from the current config
    assert_dialog_open(&app);
    let text = render_app_text(&mut app)?;
    assert!(
        text.contains("Layout  board, agent"),
        "the fresh dialog is seeded from the current layout; got:\n{text}"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing with Enabled On enters mux mode with the draft layout
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing with Enabled On enters mux mode with the draft layout
#[tokio::test]
async fn committing_with_enabled_on_enters_mux_mode_with_the_draft_layout() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the TUI is showing the single Agent view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // BUG-299: show the single Agent view directly; entering an unloaded card
    // would open the BUG-280/281 creation dialog instead.
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And the MuxConfigDialog is open with Enabled set to Off and the layout field "board, agent, files"
    // Open the dialog (mux is off by default): Enabled Off, board, agent.
    submit(&mut app, "/mux");
    drain_pending(&mut app).await?;
    assert_dialog_open(&app);
    replace_layout_field(&mut app, "board, agent, files");
    drain_pending(&mut app).await?;
    // @step When I move the cursor to the Enabled row and press Right to set it to On
    // The field has the focus; Tab moves to the Enabled toggle.
    let _ = app.handle_event(&key(KeyCode::Tab, KeyModifiers::NONE));
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert!(
        render_app_text(&mut app)?.contains("Enabled  On"),
        "Right on the Enabled row must set it to On"
    );
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then mux mode is active with three panes: Board, Agent and Files
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active");
    let cfg = app.mux_state().config();
    assert_eq!(cfg.layout.to_string(), "board, agent, files");
    assert_eq!(
        cfg.layout.leaves(),
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles
        ]
    );
    // @step And the pre-mux view recorded for exit is the single Agent view
    assert_eq!(
        app.navigator().mux.pre_mux_view(),
        Some(ViewMode::Agent),
        "the pre-mux view must be the Agent view"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing with Enabled Off exits mux mode back to the pre-mux view
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing with Enabled Off exits mux mode back to the pre-mux view
#[tokio::test]
async fn committing_with_enabled_off_exits_mux_mode_back_to_the_pre_mux_view() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given mux mode is active with the default two panes and the pre-mux view is the Board view
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await?;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the MuxConfigDialog is open with Enabled set to On
    submit(&mut app, "/mux");
    drain_pending(&mut app).await?;
    assert_dialog_open(&app);
    assert!(render_app_text(&mut app)?.contains("Enabled  On"));
    // @step When I move the cursor to the Enabled row and press Left to set it to Off
    // The field has the focus; Tab moves to the Enabled toggle.
    let _ = app.handle_event(&key(KeyCode::Tab, KeyModifiers::NONE));
    let _ = app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step And I press Enter to apply
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the slash popup lists /mux with a description and picking it opens the dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the slash popup lists /mux with a description and picking it opens the dialog
#[tokio::test]
async fn the_slash_popup_lists_mux_with_a_description_and_picking_it_opens_the_dialog() -> TestResult
{
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
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
    let popup = view.slash_popup.as_ref().or_fail("popup open")?;
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
        if matches!(a, Action::SlashCommandSelected(_, SlashCommandAction::Mux)) {
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
    seed_session(&mut app, "s-1").await?;
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Mux,
    ));
    drain_pending(&mut app).await?;
    assert_dialog_open(&app);
    Ok(())
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
fn the_agent_help_dialog_lists_mux_from_the_registry() -> TestResult {
    use codelet_fspec_tui::components::help_dialog::HelpDialog;
    use codelet_fspec_tui::components::Component;

    // @step Given the agent help content is generated from the slash command registry
    // (agent_help_lines() in components/help_content.rs iterates
    // SLASH_COMMANDS — no manual help edit needed for new rows)
    let registry = filter_commands("mux");
    assert!(
        !registry.is_empty(),
        "the registry must include a mux entry"
    );
    let mux_cmd = &registry[0];
    assert_eq!(mux_cmd.name(), "mux");
    assert!(!mux_cmd.description.is_empty());

    // @step When I read the slash-command lines of the agent help
    let mut dialog = HelpDialog::for_agent();
    let backend = TestBackend::new(200, 60);
    let mut terminal = Terminal::new(backend).or_fail("Terminal::new")?;
    terminal
        .draw(|frame| dialog.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    let buf = terminal.backend().buffer().clone();
    let text = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux help documents the config dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux help documents the config dialog
#[tokio::test]
async fn mux_help_documents_the_config_dialog() -> TestResult {
    // BUG-448: every test roots its own data dir (mux state is persisted).
    let (_guard, _data) = root_data_dir().await?;
    // @step Given the TUI is showing the single Board view with one open agent session
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // @step When I submit the slash command "/mux help"
    submit(&mut app, "/mux help");
    drain_pending(&mut app).await?;
    // @step Then a one-line notice appears in the agent scrollback
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        !text.is_empty(),
        "a notice must appear in the scrollback after /mux help"
    );
    // @step And the notice mentions the config dialog (bare /mux), the layout string with an example and the on/off subcommands
    assert!(
        text.contains("/mux opens the config dialog"),
        "the /mux help notice must mention the config dialog; got:\n{text}"
    );
    assert!(
        text.contains("/mux <layout>")
            && text.contains("e.g. /mux agent, [board, board], board, files"),
        "the /mux help notice must show the layout string with an example; got:\n{text}"
    );
    assert!(
        text.contains("/mux on|off"),
        "the /mux help notice must mention the on/off subcommands; got:\n{text}"
    );
    Ok(())
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
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}
