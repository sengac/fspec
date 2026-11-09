//! MUX-010 saved-config scenarios: an old-shape tui.mux is an unreadable
//! value (CLEAN-014 deleted its migration) and the new shape round-trips.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use codelet_test_helpers::{OrFail, TestError, TestResult};
use serde_json::{json, Value};
use tempfile::TempDir;

use super::support::{app_with_session, submit};

/// The process-global data directory is shared by every test of this
/// binary: hold the guard while a test roots it at its own directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn root_data_dir() -> Result<(tokio::sync::MutexGuard<'static, ()>, TempDir), TestError> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = TempDir::new().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok((guard, tmp))
}

fn config_path(data: &TempDir) -> std::path::PathBuf {
    data.path().join("fspec-config.json")
}

fn seed(data: &TempDir, mux: Value) -> TestResult {
    let config = json!({ "tui": { "mux": mux, "lastUsedModel": "anthropic/claude" } });
    std::fs::write(config_path(data), config.to_string()).or_fail("seed fspec-config.json")?;
    Ok(())
}

/// The saved `tui.mux` value.
pub fn saved_mux(data: &TempDir) -> Result<Value, TestError> {
    let raw = std::fs::read_to_string(config_path(data)).or_fail("read fspec-config.json")?;
    let value: Value = serde_json::from_str(&raw).or_fail("parse fspec-config.json")?;
    Ok(value["tui"]["mux"].clone())
}

/// Scenario: An old saved tui.mux shows an error and the default layout
#[tokio::test(start_paused = true)]
async fn an_old_saved_tui_mux_shows_an_error_and_the_default_layout() -> TestResult {
    let (_guard, data) = root_data_dir().await?;
    // @step Given fspec-config.json holds tui.mux {"orientation": "Vertical", "splits": [40], "panes": ["Board", "Agent"], "focused_pane": 1, "enabled": true}
    let old = json!({"orientation": "Vertical", "splits": [40], "panes": ["Board", "Agent"],
                     "focused_pane": 1, "enabled": true});
    seed(&data, old.clone())?;
    // @step When the TUI loads the mux config
    let (mut app, _mock) = app_with_session().await?;
    app.load_mux_config();
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then the Session error dialog lists an error naming tui.mux and the unknown field "orientation"
    let errors = app.session_operation_errors();
    assert_eq!(errors.len(), 1, "exactly one error: {errors:?}");
    let error = errors.first().or_fail("the error")?;
    assert!(
        error.contains("tui.mux") && error.contains("unknown field `orientation`"),
        "{error}"
    );
    // @step And the loaded mux layout is the default "board, agent"
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        "board, agent"
    );
    // @step And fspec-config.json still holds the old tui.mux unchanged
    assert_eq!(saved_mux(&data)?, old);
    Ok(())
}

/// Scenario: A saved tui.mux in the new shape round-trips and an unreadable one is left on disk
#[tokio::test(start_paused = true)]
async fn a_new_shape_round_trips_and_an_unreadable_one_is_left_alone() -> TestResult {
    let (_guard, data) = root_data_dir().await?;
    // @step Given fspec-config.json holds tui.mux {"layout": "agent, [board, files]", "focused_pane": 0, "enabled": false}
    let saved = json!({"layout": "agent, [board, files]", "focused_pane": 0, "enabled": false});
    seed(&data, saved.clone())?;
    // @step When the TUI loads the mux config and the user runs /mux save
    let (mut app, _mock) = app_with_session().await?;
    app.load_mux_config();
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        "agent, [board, files]"
    );
    submit(&mut app, "/mux save").await?;
    // @step Then the saved tui.mux is byte-for-byte the same value
    assert_eq!(saved_mux(&data)?, saved);
    // @step And a saved value whose layout string does not parse loads the default layout and is left on disk unchanged
    let broken = json!({"layout": "agent, [board", "focused_pane": 0, "enabled": false});
    seed(&data, broken.clone())?;
    let (mut fresh, _mock) = app_with_session().await?;
    fresh.load_mux_config();
    assert_eq!(
        fresh.navigator().mux.config().layout.to_string(),
        "board, agent"
    );
    assert_eq!(saved_mux(&data)?, broken);
    Ok(())
}
