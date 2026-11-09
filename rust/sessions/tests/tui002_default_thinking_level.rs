//! Feature: spec/features/default-thinking-level-store-and-session-inheritance.feature
//!
//! TUI-002 — persist the saved thinking level. TUI-092 — storage delegates to
//! the shared CONFIG-008 `fspec-config.json`; since MODEL-012 the level is
//! saved per model under `tui.thinkingLevels`. The path-injectable cores take
//! `data_dir + cwd` (`*_with_dirs`), so these round-trip scenarios pass a
//! throwaway temp dir for both. The `set_thinking_level_default` scenario
//! drives a real `SessionManager` against a throwaway data dir seeded with
//! the offline models.dev fixture, serialised on the process-global data
//! directory via `DATA_DIR_GUARD`. Which level a new session starts at is
//! MODEL-012's feature (thinking-level-remembered-per-model).

use std::path::PathBuf;
use std::sync::Mutex;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::ThinkingLevel;
use codelet_sessions::default_thinking_level_persistence::{
    load_default_thinking_level_with_dirs, save_default_thinking_level_with_dirs,
};
use codelet_sessions::SessionManager;
use codelet_test_helpers::{OrFailErr, TestError, TestResult};

/// Trimmed offline models.dev catalog (anthropic/openai/google).
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// The model every saved level here belongs to.
const OPUS: &str = "anthropic/claude-opus-4-5";

/// Serialises tests that depend on the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Root a fresh data dir whose model cache is pre-seeded from the offline
/// fixture. Returns the live TempDir (kept alive by the caller) and its path.
fn seed_data_dir() -> Result<(tempfile::TempDir, PathBuf), TestError> {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
    let data_dir = tempfile::tempdir()?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE)?;
    let path = data_dir.path().to_path_buf();
    Ok((data_dir, path))
}

/// Scenario: Saving a model's thinking level round-trips from disk
#[test]
fn saving_a_models_thinking_level_round_trips_from_disk() -> TestResult {
    // @step Given a data directory with no persisted default thinking level
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();
    assert_eq!(
        load_default_thinking_level_with_dirs(dir, dir, OPUS),
        Ok(None)
    );
    // @step When the thinking level High is saved for anthropic/claude-opus-4-5 to that data directory
    save_default_thinking_level_with_dirs(
        dir,
        dir,
        OPUS,
        ThinkingLevel::Effort("high".to_string()),
    )?;
    // @step And the saved thinking level for anthropic/claude-opus-4-5 is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(dir, dir, OPUS);
    // @step Then the loaded thinking level is High
    assert_eq!(loaded, Ok(Some(ThinkingLevel::Effort("high".to_string()))));
    Ok(())
}

/// Scenario: A missing config file has no saved level
#[test]
fn a_missing_config_file_has_no_saved_level() -> TestResult {
    // @step Given a data directory with no persisted default thinking level
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();
    // @step When the saved thinking level for anthropic/claude-opus-4-5 is loaded from that data directory
    let loaded = load_default_thinking_level_with_dirs(dir, dir, OPUS);
    // @step Then there is no saved thinking level, not Off
    assert_eq!(loaded, Ok(None));
    Ok(())
}

/// Scenario: set_thinking_level_default for an unknown session saves nothing
#[test]
fn set_thinking_level_default_for_an_unknown_session_saves_nothing() -> TestResult {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a session manager rooted at a data directory with no persisted default
    let (_tmp, dir) = seed_data_dir()?;
    codelet_common::set_data_directory(dir.clone())?;
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    codelet_test_helpers::shared_temp_subdirectory()
        .and_then(|workspace| manager.set_default_project(workspace))?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step When set_thinking_level_default is called with High for an unknown session
    let unknown = codelet_rpc_types::SessionId::new("00000000-0000-0000-0000-000000000000");
    let refusal = handle
        .set_thinking_level_default(&unknown, ThinkingLevel::Effort("high".to_string()))
        .or_fail_err("an unknown session is refused")?;
    // @step Then the call fails naming the unknown session
    assert!(refusal.contains(&unknown.value), "{refusal}");
    // @step And no thinking level is saved in that data directory
    assert!(
        !dir.join("fspec-config.json").exists(),
        "nothing is written for an unknown session"
    );
    Ok(())
}
