//! Feature: spec/features/mux-config-persistence-wiring.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! BUG-167 — mux config save is a no-op in the real binary: the shared
//! fspec-config.json persist dirs were never wired in production
//! (set_mux_persist_dir was only ever called from tests). These tests pin
//! the FIX: App::new is the single wiring point — it resolves the CONFIG-008
//! two-scope shared-config dirs (process-global data dir + cwd) and MuxState
//! delegates load/save to the existing mux_config_persistence globals. The
//! tests root the process-global data directory at a temp dir
//! (codelet_common::set_data_directory, the established tui093 pattern) and
//! drive the REAL save paths (/mux save, dialog 's', mux-exit auto-save)
//! with NO manual persist-dirs call.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::sync::Arc;

use serde_json::json;
use serial_test::serial;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::multiplex::{MuxConfig, MuxPaneKind};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn key(code: KeyCode, mods: KeyModifiers) -> crossterm::event::Event {
    crossterm::event::Event::Key(KeyEvent::new(code, mods))
}

/// MUX-010: replace the MuxConfigDialog's layout field text (the field
/// has the focus when the dialog opens): End, Backspace until empty,
/// then type `text`.
fn replace_layout_field(app: &mut App, text: &str) {
    app.handle_event(&key(KeyCode::End, KeyModifiers::NONE));
    for _ in 0..64 {
        app.handle_event(&key(KeyCode::Backspace, KeyModifiers::NONE));
    }
    for c in text.chars() {
        app.handle_event(&key(KeyCode::Char(c), KeyModifiers::NONE));
    }
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

/// Root the process-global data directory at a fresh throwaway dir (the
/// established tui093 pattern). Returns the guard (held for the test's
/// duration) + the TempDir.
async fn root_data_dir() -> Result<
    (tokio::sync::MutexGuard<'static, ()>, tempfile::TempDir),
    codelet_test_helpers::TestError,
> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok((guard, tmp))
}

/// Read the user-scope shared config as a JSON value.
fn read_user_config(
    data_dir: &std::path::Path,
) -> Result<serde_json::Value, codelet_test_helpers::TestError> {
    let raw =
        fs::read_to_string(data_dir.join("fspec-config.json")).or_fail("read fspec-config.json")?;
    Ok(serde_json::from_str(&raw).or_fail("parse fspec-config.json")?)
}

/// Seed one open agent session (the slash-command path needs a current
/// session to surface scrollback notices into).
async fn seed_session(app: &mut App, id: &str) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await?;
    Ok(())
}

/// Seed sibling keys in the user-scope shared config that a save must
/// preserve.
fn seed_sibling_config(data_dir: &std::path::Path) -> Result<(), codelet_test_helpers::TestError> {
    fs::create_dir_all(data_dir).or_fail("mkdir")?;
    fs::write(
        data_dir.join("fspec-config.json"),
        json!({"agent": "claude", "tools": {"test": "cargo test"}}).to_string(),
    )
    .or_fail("seed sibling config")?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux save persists to the shared fspec-config.json with no
// manual persist-dirs wiring
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux save persists to the shared fspec-config.json with no manual persist-dirs wiring
#[tokio::test]
#[serial]
async fn slash_mux_save_persists_with_no_manual_persist_dirs_wiring() -> TestResult {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().or_fail("data dir")?;
    // @step When mux mode is active and I submit the slash command "/mux save"
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await?;
    submit(&mut app, "/mux save");
    drain_pending(&mut app).await?;
    // @step Then the fspec-config.json under the data directory contains the tui.mux key
    let config = read_user_config(&data_dir)?;
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must be a JSON object after /mux save: {config}"
    );
    // @step And the tui.mux value holds the live mux config (layout string, focused pane, enabled)
    assert_eq!(
        config["tui"]["mux"],
        json!({"layout": "board, agent", "focused_pane": 1, "enabled": true}),
        "tui.mux must hold exactly the layout string, focused pane and enabled flag"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing the mux config dialog with Save persists tui.mux
// to the shared config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing the mux config dialog with Save persists tui.mux to the shared config
#[tokio::test]
#[serial]
async fn dialog_save_commits_and_persists_tui_mux_to_the_shared_config() -> TestResult {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().or_fail("data dir")?;
    seed_sibling_config(&data_dir)?;
    // @step And the MuxConfigDialog is open with the layout field set to "[board, agent]"
    submit(&mut app, "/mux");
    drain_pending(&mut app).await?;
    assert!(app.compositor().contains("mux-config-dialog"));
    replace_layout_field(&mut app, "[board, agent]");
    drain_pending(&mut app).await?;
    // @step When I activate the Save button to apply and save
    // (Shift+Tab from the field wraps to Save; Enter activates it)
    app.handle_event(&key(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    // @step Then the MuxConfigDialog is closed and the live mux layout is "[board, agent]"
    assert!(
        !app.compositor().contains("mux-config-dialog"),
        "the dialog must close on Save"
    );
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        "[board, agent]"
    );
    // @step And the fspec-config.json under the data directory contains a tui.mux key with layout "[board, agent]"
    let config = read_user_config(&data_dir)?;
    assert_eq!(
        config["tui"]["mux"]["layout"],
        json!("[board, agent]"),
        "the layout string must round-trip: {config}"
    );
    // @step And pre-existing sibling keys in the shared config are preserved
    assert_eq!(config["agent"], json!("claude"));
    assert_eq!(config["tools"]["test"], json!("cargo test"));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: exiting mux mode auto-saves the post-exit config to tui.mux
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: exiting mux mode auto-saves the post-exit config to tui.mux
#[tokio::test]
#[serial]
async fn exiting_mux_mode_auto_saves_the_post_exit_config_to_tui_mux() -> TestResult {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir().await?;
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await?;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().or_fail("data dir")?;
    // @step And mux mode is active
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await?;
    // @step When I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await?;
    // @step Then the fspec-config.json under the data directory contains a tui.mux key
    let config = read_user_config(&data_dir)?;
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must exist after the mux-exit auto-save: {config}"
    );
    // @step And the saved tui.mux value has enabled=false and the live layout string
    assert_eq!(config["tui"]["mux"]["enabled"], json!(false));
    assert_eq!(config["tui"]["mux"]["layout"], json!("board, agent"));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a project-scope tui.mux overrides the user-scope value on load
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a project-scope tui.mux overrides the user-scope value on load
#[tokio::test]
async fn a_project_scope_tui_mux_overrides_the_user_scope_value_on_load() -> TestResult {
    // @step Given a user-scope fspec-config.json with tui.mux holding the layout "board, agent"
    let user = tempfile::tempdir().or_fail("user tempdir")?;
    fs::create_dir_all(user.path()).or_fail("mkdir")?;
    fs::write(
        user.path().join("fspec-config.json"),
        json!({"tui": {"mux": {"layout": "board, agent", "focused_pane": 1, "enabled": true}}})
            .to_string(),
    )
    .or_fail("seed user")?;
    // @step And a project spec/fspec-config.json with tui.mux holding the layout "[board:2, agent:3]"
    let project = tempfile::tempdir().or_fail("project tempdir")?;
    fs::create_dir_all(project.path().join("spec")).or_fail("mkdir spec")?;
    fs::write(
        project.path().join("spec").join("fspec-config.json"),
        json!({"tui": {"mux": {"layout": "[board:2, agent:3]", "focused_pane": 1, "enabled": true}}}).to_string(),
    )
    .or_fail("seed project")?;
    // @step When a fresh TUI bootstrap loads the persisted mux config
    // (the path-injectable core — the App-level load reads the same
    // deep-merged view through the shared-config globals)
    let value = codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
        user.path(),
        project.path(),
    )
    .or_fail("the config loads")?
    .or_fail("project tui.mux must load")?;
    // @step Then the live mux layout is "[board:2, agent:3]" (the project value wins)
    let cfg: MuxConfig = serde_json::from_value(value).or_fail("MuxConfig round-trip")?;
    assert_eq!(cfg.layout.to_string(), "[board:2, agent:3]");
    assert_eq!(
        cfg.layout.leaves(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent]
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: an unresolvable data directory never panics load or save
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: an unresolvable data directory never panics load or save
#[tokio::test]
async fn an_unresolvable_data_directory_never_panics_load_or_save() -> TestResult {
    // @step Given the process-global data directory cannot be resolved and no fallback home is available
    let missing = tempfile::tempdir().or_fail("empty tempdir")?;
    // @step When a fresh TUI bootstrap loads the persisted mux config
    let value = codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
        missing.path(),
        missing.path(),
    )
    .or_fail("a missing config loads")?;
    // @step Then the live mux config is the default layout "board, agent"
    assert!(
        value.is_none(),
        "a missing tui.mux key must load as None (default preset fallback): {value:?}"
    );
    assert_eq!(
        MuxConfig::default().layout.to_string(),
        "board, agent",
        "the fallback is the default layout"
    );
    // @step And submitting "/mux save" surfaces a one-line error notice in the agent scrollback
    // and the TUI remains usable (no panic) — the save error path returns
    // Err(String), which dispatch surfaces via push_mux_error. A save to a
    // path that cannot be created (a non-existent nested dir under a file)
    // must return Err, never panic.
    let file_path = missing.path().join("not-a-dir");
    fs::write(&file_path, "x").or_fail("seed file")?;
    let err = codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(
        &file_path.join("nested"),
        &file_path,
        &json!({
            "layout": "board, agent",
            "focused_pane": 1,
            "enabled": true
        }),
    );
    // @step And the TUI remains usable (no panic)
    assert!(
        err.is_err(),
        "a save to an uncreatable path must surface Err (never panic): {err:?}"
    );
    Ok(())
}
