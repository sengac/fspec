//! MUX-011: a saved tui.mux that cannot be read shows an exact error (the
//! key, the column and the problem) as an app-level notice, and the
//! default layout is used for this session; the bad text stays on disk.
//!
//! Feature: spec/features/an-unreadable-saved-mux-layout-shows-an-exact-error.feature
//!
//! Harness: App + MockBackend with the process data directory rooted in a
//! temp dir holding a hand-edited user fspec-config.json (the bug167
//! pattern); App::load_mux_config is the startup load. Deterministic.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::sync::Arc;

use serde_json::{json, Value};
use serial_test::serial;

use codelet_fspec_tui::{App, FspecBackend};

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const DEFAULT_LAYOUT: &str = "board, agent";

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

/// Root the process data directory at a temp dir whose user
/// fspec-config.json is `config`.
async fn root_data_dir_with(
    config: &Value,
) -> Result<
    (tokio::sync::MutexGuard<'static, ()>, tempfile::TempDir),
    codelet_test_helpers::TestError,
> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    fs::write(tmp.path().join("fspec-config.json"), config.to_string()).or_fail("seed config")?;
    Ok((guard, tmp))
}

/// The startup load: a fresh App loads the saved mux config.
async fn load_at_startup() -> Result<App, codelet_test_helpers::TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.load_mux_config();
    drain_pending(&mut app).await?;
    Ok(app)
}

fn mux_with_layout(layout: &str) -> Value {
    json!({"tui": {"mux": {"layout": layout, "focused_pane": 0, "enabled": false}}})
}

fn only_error(app: &App) -> Result<String, codelet_test_helpers::TestError> {
    let errors = app.session_operation_errors();
    assert_eq!(errors.len(), 1, "exactly one error: {errors:?}");
    Ok(errors.first().or_fail("the error")?.clone())
}

#[tokio::test]
#[serial]
async fn an_unparseable_saved_layout_shows_the_key_the_column_and_the_problem() -> TestResult {
    // @step Given the user fspec-config.json holds tui.mux with the layout "board, [agent"
    let (_guard, tmp) = root_data_dir_with(&mux_with_layout("board, [agent")).await?;

    // @step When the TUI loads the saved mux config at startup
    let app = load_at_startup().await?;

    // @step Then the Session error dialog lists "tui.mux.layout in fspec-config.json: unbalanced bracket: '[' at column 8 is never closed; using the default layout this session"
    assert_eq!(
        only_error(&app)?,
        "tui.mux.layout in fspec-config.json: unbalanced bracket: '[' at column 8 is never closed; using the default layout this session"
    );

    // @step And the live mux layout is "board, agent"
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        DEFAULT_LAYOUT
    );

    // @step And the user fspec-config.json still holds the layout "board, [agent"
    let raw = fs::read_to_string(tmp.path().join("fspec-config.json")).or_fail("read config")?;
    let saved: Value = serde_json::from_str(&raw).or_fail("parse config")?;
    assert_eq!(saved["tui"]["mux"]["layout"], json!("board, [agent"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn an_unknown_pane_in_the_saved_layout_names_its_column() -> TestResult {
    // @step Given the user fspec-config.json holds tui.mux with the layout "board, banana"
    let (_guard, _tmp) = root_data_dir_with(&mux_with_layout("board, banana")).await?;

    // @step When the TUI loads the saved mux config at startup
    let app = load_at_startup().await?;

    // @step Then the Session error dialog lists an error naming tui.mux.layout, column 8 and the unknown pane 'banana'
    let error = only_error(&app)?;
    assert!(
        error.starts_with("tui.mux.layout in fspec-config.json: "),
        "{error}"
    );
    assert!(
        error.contains("unknown pane 'banana' at column 8"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_saved_tui_mux_that_is_not_an_object_shows_an_error() -> TestResult {
    // @step Given the user fspec-config.json holds tui.mux as the string "board"
    let (_guard, _tmp) = root_data_dir_with(&json!({"tui": {"mux": "board"}})).await?;

    // @step When the TUI loads the saved mux config at startup
    let app = load_at_startup().await?;

    // @step Then the Session error dialog lists an error naming tui.mux
    let error = only_error(&app)?;
    assert!(
        error.starts_with("tui.mux in fspec-config.json: "),
        "{error}"
    );
    assert!(
        error.ends_with("; using the default layout this session"),
        "{error}"
    );

    // @step And the live mux layout is "board, agent"
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        DEFAULT_LAYOUT
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_saved_tui_mux_with_an_unknown_field_shows_an_error() -> TestResult {
    // @step Given the user fspec-config.json holds tui.mux with an extra field "orientation2"
    let config = json!({"tui": {"mux": {
        "layout": "board, agent", "focused_pane": 0, "enabled": false, "orientation2": "h"
    }}});
    let (_guard, _tmp) = root_data_dir_with(&config).await?;

    // @step When the TUI loads the saved mux config at startup
    let app = load_at_startup().await?;

    // @step Then the Session error dialog lists an error naming tui.mux and the unknown field "orientation2"
    let error = only_error(&app)?;
    assert!(
        error.starts_with("tui.mux in fspec-config.json: "),
        "{error}"
    );
    assert!(error.contains("orientation2"), "{error}");
    Ok(())
}

#[tokio::test]
#[serial]
async fn no_saved_tui_mux_uses_the_default_layout_without_an_error() -> TestResult {
    // @step Given the user fspec-config.json has no tui.mux key
    let (_guard, _tmp) = root_data_dir_with(&json!({"agent": "claude"})).await?;

    // @step When the TUI loads the saved mux config at startup
    let app = load_at_startup().await?;

    // @step Then no Session error dialog opens
    assert!(
        app.session_operation_errors().is_empty(),
        "{:?}",
        app.session_operation_errors()
    );

    // @step And the live mux layout is "board, agent"
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        DEFAULT_LAYOUT
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn an_unreadable_fspec_config_json_shows_its_error_and_the_default_layout() -> TestResult {
    // @step Given the user fspec-config.json holds the text "{ not json"
    let (_guard, tmp) = root_data_dir_with(&json!({})).await?;
    let path = tmp.path().join("fspec-config.json");
    fs::write(&path, "{ not json").or_fail("write the broken config")?;

    // @step When the TUI starts and loads the mux config
    let app = load_at_startup().await?;

    // @step Then the Session error dialog lists one error naming fspec-config.json as invalid JSON and saying the default layout is used this session
    let error = only_error(&app)?;
    assert!(
        error.starts_with(&format!("Invalid JSON in {}:", path.display()))
            && error.ends_with("; using the default layout this session"),
        "{error}"
    );

    // @step And the mux layout is "board, agent"
    assert_eq!(
        app.navigator().mux.config().layout.to_string(),
        DEFAULT_LAYOUT
    );

    // @step And fspec-config.json still holds "{ not json"
    assert_eq!(
        fs::read_to_string(&path).or_fail("read config")?,
        "{ not json"
    );
    Ok(())
}
