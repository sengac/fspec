//! Feature: spec/features/default-thinking-level-persistence.feature
//!
//! TUI-002 — persist the default thinking level and re-apply it to every
//! new/resumed session so an idle Rust ratatui session shows the yellow
//! `[T:High]` badge (parity with the TS Ink reference).
//!
//! TUI-092 — storage now delegates to the shared CONFIG-008 `fspec-config.json`
//! under the nested key `tui.defaultThinkingLevel`. The path-injectable cores
//! take `data_dir + cwd` (`*_with_dirs`), so these round-trip scenarios pass a
//! throwaway temp dir for both. The `set_thinking_level_default` +
//! session-creation scenarios drive a real `SessionManager` against a throwaway
//! data dir seeded with the offline models.dev fixture, serialised on the
//! process-global data directory via `DATA_DIR_GUARD`.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::ThinkingLevel;
use codelet_sessions::default_thinking_level_persistence::{
    load_default_thinking_level_with_dirs, save_default_thinking_level_with_dirs,
};
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google).
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that depend on the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Root a fresh data dir whose model cache is pre-seeded from the offline
/// fixture. Returns the live TempDir (kept alive by the caller) and its path.
fn seed_data_dir() -> (tempfile::TempDir, PathBuf) {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write fixture");
    let path = data_dir.path().to_path_buf();
    (data_dir, path)
}

/// Scenario: Saving the default thinking level round-trips from disk
#[test]
fn saving_the_default_thinking_level_round_trips_from_disk() {
    // @step Given a data directory with no persisted default thinking level
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    assert_eq!(
        load_default_thinking_level_with_dirs(dir, dir),
        ThinkingLevel::Off
    );
    // @step When the default thinking level High is saved to that data directory
    save_default_thinking_level_with_dirs(dir, dir, ThinkingLevel::High).expect("save");
    // @step And the default thinking level is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is High
    assert_eq!(loaded, ThinkingLevel::High);
}

/// Scenario: A missing config file loads as Off
#[test]
fn a_missing_config_file_loads_as_off() {
    // @step Given a data directory with no persisted default thinking level
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the default thinking level is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is Off
    assert_eq!(loaded, ThinkingLevel::Off);
}

/// Scenario: An out-of-range persisted value loads as Off
#[test]
fn an_out_of_range_persisted_value_loads_as_off() {
    // @step Given a data directory whose default thinking level config holds the value 7
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    std::fs::write(
        dir.join("fspec-config.json"),
        r#"{"tui":{"defaultThinkingLevel":7}}"#,
    )
    .expect("seed");
    // @step When the default thinking level is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is Off
    assert_eq!(loaded, ThinkingLevel::Off);
}

/// Scenario: set_thinking_level_default persists the level even with no live session
#[test]
fn set_thinking_level_default_persists_the_level_even_with_no_live_session() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a session manager rooted at a data directory with no persisted default
    let (_tmp, dir) = seed_data_dir();
    codelet_common::set_data_directory(dir.clone()).expect("set data dir");
    assert_eq!(
        load_default_thinking_level_with_dirs(&dir, &dir),
        ThinkingLevel::Off
    );
    let manager = Arc::new(SessionManager::new());
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step When set_thinking_level_default is called with High for an unknown session
    let unknown = codelet_rpc_types::SessionId::new("00000000-0000-0000-0000-000000000000");
    let _ = handle.set_thinking_level_default(&unknown, ThinkingLevel::High);
    // @step And the default thinking level is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(&dir, &dir);
    // @step Then the loaded default thinking level is High
    assert_eq!(loaded, ThinkingLevel::High);
}

/// Scenario: A new session inherits a persisted High default
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_session_inherits_a_persisted_high_default() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a session manager rooted at a data directory whose persisted default thinking level is High
    let (_tmp, dir) = seed_data_dir();
    save_default_thinking_level_with_dirs(&dir, &dir, ThinkingLevel::High).expect("persist High");
    codelet_common::set_data_directory(dir).expect("set data dir");
    let manager = Arc::new(SessionManager::new());
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_default_model("anthropic/claude-opus-4-5");
    // @step When a new session is created
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "session must be created");
    // @step Then the new session's thinking level is High
    assert_eq!(handle.get_thinking_level(&sid), ThinkingLevel::High);
}

/// Scenario: A new session inherits a persisted Off default
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_session_inherits_a_persisted_off_default() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a session manager rooted at a data directory whose persisted default thinking level is Off
    let (_tmp, dir) = seed_data_dir();
    save_default_thinking_level_with_dirs(&dir, &dir, ThinkingLevel::Off).expect("persist Off");
    codelet_common::set_data_directory(dir).expect("set data dir");
    let manager = Arc::new(SessionManager::new());
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_default_model("anthropic/claude-opus-4-5");
    // @step When a new session is created
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "session must be created");
    // @step Then the new session's thinking level is Off
    assert_eq!(handle.get_thinking_level(&sid), ThinkingLevel::Off);
}
