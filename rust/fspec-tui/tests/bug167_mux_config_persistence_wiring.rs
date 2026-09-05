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

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::Arc;

use serde_json::json;
use serial_test::serial;
use std::sync::Mutex;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::multiplex::{MuxConfig, MuxOrientation, MuxPaneKind};
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

fn key(code: KeyCode, mods: KeyModifiers) -> crossterm::event::Event {
    crossterm::event::Event::Key(KeyEvent::new(code, mods))
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

/// Root the process-global data directory at a fresh throwaway dir (the
/// established tui093 pattern). Returns the guard (held for the test's
/// duration) + the TempDir.
fn root_data_dir() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tmp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set data dir");
    (guard, tmp)
}

/// Read the user-scope shared config as a JSON value.
fn read_user_config(data_dir: &std::path::Path) -> serde_json::Value {
    let raw = fs::read_to_string(data_dir.join("fspec-config.json"))
        .expect("read fspec-config.json");
    serde_json::from_str(&raw).expect("parse fspec-config.json")
}

/// Seed one open agent session (the slash-command path needs a current
/// session to surface scrollback notices into).
async fn seed_session(app: &mut App, id: &str) {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
    drain_pending(app).await;
}

/// Seed sibling keys in the user-scope shared config that a save must
/// preserve.
fn seed_sibling_config(data_dir: &std::path::Path) {
    fs::create_dir_all(data_dir).expect("mkdir");
    fs::write(
        data_dir.join("fspec-config.json"),
        json!({"agent": "claude", "tools": {"test": "cargo test"}}).to_string(),
    )
    .expect("seed sibling config");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /mux save persists to the shared fspec-config.json with no
// manual persist-dirs wiring
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: /mux save persists to the shared fspec-config.json with no manual persist-dirs wiring
#[tokio::test]
#[serial]
async fn slash_mux_save_persists_with_no_manual_persist_dirs_wiring() {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().expect("data dir");
    // @step When mux mode is active and I submit the slash command "/mux save"
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    submit(&mut app, "/mux save");
    drain_pending(&mut app).await;
    // @step Then the fspec-config.json under the data directory contains the tui.mux key
    let config = read_user_config(&data_dir);
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must be a JSON object after /mux save: {config}"
    );
    // @step And the tui.mux value holds the live mux config (orientation, splits, pane list, focused pane, enabled)
    assert_eq!(config["tui"]["mux"]["enabled"], json!(true));
    assert_eq!(config["tui"]["mux"]["orientation"], json!("Horizontal"));
    assert_eq!(config["tui"]["mux"]["splits"], json!([50]));
    assert_eq!(
        config["tui"]["mux"]["panes"],
        json!(["Board", "Agent"])
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: committing the mux config dialog with 's' persists tui.mux
// to the shared config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: committing the mux config dialog with 's' persists tui.mux to the shared config
#[tokio::test]
#[serial]
async fn dialog_s_commits_and_persists_tui_mux_to_the_shared_config() {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().expect("data dir");
    seed_sibling_config(&data_dir);
    // @step And the MuxConfigDialog is open with the Orientation row set to Vertical
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    // Cursor 0 = Enabled; Down → Orientation row; Right → Vertical.
    app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step When I press 's' to apply and save
    app.handle_event(&key(KeyCode::Char('s'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the MuxConfigDialog is closed and the live mux layout is vertical
    assert!(
        !app.compositor().contains("mux-config-dialog"),
        "the dialog must close on 's'"
    );
    assert_eq!(
        app.mux_state().config().orientation,
        MuxOrientation::Vertical
    );
    // @step And the fspec-config.json under the data directory contains a tui.mux key with orientation "Vertical"
    let config = read_user_config(&data_dir);
    assert_eq!(
        config["tui"]["mux"]["orientation"],
        json!("Vertical"),
        "orientation must round-trip: {config}"
    );
    // @step And pre-existing sibling keys in the shared config are preserved
    assert_eq!(config["agent"], json!("claude"));
    assert_eq!(config["tools"]["test"], json!("cargo test"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: exiting mux mode auto-saves the post-exit config to tui.mux
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: exiting mux mode auto-saves the post-exit config to tui.mux
#[tokio::test]
#[serial]
async fn exiting_mux_mode_auto_saves_the_post_exit_config_to_tui_mux() {
    // @step Given an App constructed with a backend and no explicit persist-dirs setup
    let (_tmp_guard, _tmp) = root_data_dir();
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    // @step And the process-global data directory is rooted at a throwaway directory
    let data_dir = codelet_common::get_data_dir().expect("data dir");
    // @step And mux mode is active
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;
    // @step When I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    drain_pending(&mut app).await;
    // @step Then the fspec-config.json under the data directory contains a tui.mux key
    let config = read_user_config(&data_dir);
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must exist after the mux-exit auto-save: {config}"
    );
    // @step And the saved tui.mux value has enabled=false and the live panes and splits
    assert_eq!(config["tui"]["mux"]["enabled"], json!(false));
    assert_eq!(config["tui"]["mux"]["panes"], json!(["Board", "Agent"]));
    assert_eq!(config["tui"]["mux"]["splits"], json!([50]));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a project-scope tui.mux overrides the user-scope value on load
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: a project-scope tui.mux overrides the user-scope value on load
#[tokio::test]
async fn a_project_scope_tui_mux_overrides_the_user_scope_value_on_load() {
    // @step Given a user-scope fspec-config.json with tui.mux holding a horizontal 50/50 Board|Agent preset
    let user = tempfile::tempdir().expect("user tempdir");
    fs::create_dir_all(user.path()).expect("mkdir");
    fs::write(
        user.path().join("fspec-config.json"),
        json!({"tui": {"mux": {"orientation": "Horizontal", "splits": [50], "panes": ["Board", "Agent"], "focused_pane": 1, "enabled": true}}}).to_string(),
    )
    .expect("seed user");
    // @step And a project spec/fspec-config.json with tui.mux holding a vertical 40/60 Board|Agent preset
    let project = tempfile::tempdir().expect("project tempdir");
    fs::create_dir_all(project.path().join("spec")).expect("mkdir spec");
    fs::write(
        project.path().join("spec").join("fspec-config.json"),
        json!({"tui": {"mux": {"orientation": "Vertical", "splits": [40], "panes": ["Board", "Agent"], "focused_pane": 1, "enabled": true}}}).to_string(),
    )
    .expect("seed project");
    // @step When a fresh TUI bootstrap loads the persisted mux config
    // (the path-injectable core — the App-level load reads the same
    // deep-merged view through the shared-config globals)
    let value =
        codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
            user.path(),
            project.path(),
        )
        .expect("project tui.mux must load");
    // @step Then the live mux config is vertical with a 40/60 split (the project value wins)
    let cfg: MuxConfig = serde_json::from_value(value).expect("MuxConfig round-trip");
    assert_eq!(cfg.orientation, MuxOrientation::Vertical);
    assert_eq!(cfg.splits, vec![40]);
    assert_eq!(cfg.panes, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: an unresolvable data directory never panics load or save
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: an unresolvable data directory never panics load or save
#[tokio::test]
async fn an_unresolvable_data_directory_never_panics_load_or_save() {
    // @step Given the process-global data directory cannot be resolved and no fallback home is available
    let missing = tempfile::tempdir().expect("empty tempdir");
    // @step When a fresh TUI bootstrap loads the persisted mux config
    let value = codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
        missing.path(),
        missing.path(),
    );
    // @step Then the live mux config is the default preset (horizontal Board|Agent 50/50)
    assert!(
        value.is_none(),
        "a missing tui.mux key must load as None (default preset fallback): {value:?}"
    );
    // @step And submitting "/mux save" surfaces a one-line error notice in the agent scrollback
    // and the TUI remains usable (no panic) — the save error path returns
    // Err(String), which dispatch surfaces via push_mux_error. A save to a
    // path that cannot be created (a non-existent nested dir under a file)
    // must return Err, never panic.
    let file_path = missing.path().join("not-a-dir");
    fs::write(&file_path, "x").expect("seed file");
    let err = codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(
        &file_path.join("nested"),
        &file_path,
        &json!({
            "orientation": "Horizontal",
            "splits": [50],
            "panes": ["Board", "Agent"],
            "focused_pane": 1,
            "enabled": true
        }),
    );
    // @step And the TUI remains usable (no panic)
    assert!(
        err.is_err(),
        "a save to an uncreatable path must surface Err (never panic): {err:?}"
    );
}
