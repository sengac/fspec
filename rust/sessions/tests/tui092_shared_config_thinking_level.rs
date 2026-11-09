//! Feature: spec/features/default-thinking-level-shared-config-persistence.feature
//!
//! TUI-092 — the saved thinking level is persisted to the shared CONFIG-008
//! `fspec-config.json`; since MODEL-012 it is per model under
//! `tui.thinkingLevels`. These integration tests exercise the path-injectable
//! `*_with_dirs` cores against OS temp dirs (no fs mocking, no `$HOME`
//! mutation) and assert the on-disk JSON layout, sibling-key preservation,
//! project-scope override, a missing file (no saved level, never Off), and
//! the global wrapper round-trip.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Mutex;

use codelet_rpc_types::ThinkingLevel;
use codelet_sessions::default_thinking_level_persistence::{
    load_default_thinking_level, load_default_thinking_level_with_dirs,
    save_default_thinking_level, save_default_thinking_level_with_dirs,
};
use serde_json::Value;

/// The model every saved level here belongs to.
const OPUS: &str = "anthropic/claude-opus-4-5";

/// Serialises tests touching the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Read `<data_dir>/fspec-config.json` as a `serde_json::Value`.
fn read_user_config(data_dir: &std::path::Path) -> Value {
    let raw = std::fs::read_to_string(data_dir.join("fspec-config.json"))
        .expect("read fspec-config.json");
    serde_json::from_str(&raw).expect("parse fspec-config.json")
}

/// Scenario: Saving High writes the model's entry under tui.thinkingLevels
#[test]
fn saving_high_writes_the_models_entry_under_thinking_levels() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the thinking level High is saved for anthropic/claude-opus-4-5 to the user scope
    save_default_thinking_level_with_dirs(
        dir,
        dir,
        OPUS,
        ThinkingLevel::Effort("high".to_string()),
    )
    .expect("save High");
    // @step Then the file fspec-config.json under the data directory contains tui.thinkingLevels with effort:high for anthropic/claude-opus-4-5
    let config = read_user_config(dir);
    assert_eq!(
        config["tui"]["thinkingLevels"][OPUS].as_str(),
        Some("effort:high")
    );
}

/// Scenario: Saved level round-trips through the shared config
#[test]
fn saved_level_round_trips_through_the_shared_config() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the thinking level Medium is saved for anthropic/claude-opus-4-5 to the user scope
    save_default_thinking_level_with_dirs(
        dir,
        dir,
        OPUS,
        ThinkingLevel::Effort("medium".to_string()),
    )
    .expect("save Medium");
    // @step And the saved thinking level for anthropic/claude-opus-4-5 is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(dir, dir, OPUS);
    // @step Then the loaded thinking level is Medium
    assert_eq!(
        loaded,
        Ok(Some(ThinkingLevel::Effort("medium".to_string())))
    );
}

/// Scenario: Saving preserves pre-existing sibling keys
#[test]
fn saving_preserves_pre_existing_sibling_keys() {
    // @step Given a user shared config that already contains tui.lastUsedModel set to "anthropic/claude-opus-4-5"
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    std::fs::write(
        dir.join("fspec-config.json"),
        r#"{"tui":{"lastUsedModel":"anthropic/claude-opus-4-5"}}"#,
    )
    .expect("seed sibling");
    // @step When the thinking level High is saved for anthropic/claude-opus-4-5 to the user scope
    save_default_thinking_level_with_dirs(
        dir,
        dir,
        OPUS,
        ThinkingLevel::Effort("high".to_string()),
    )
    .expect("save High");
    // @step Then the shared config still contains tui.lastUsedModel equal to "anthropic/claude-opus-4-5"
    let config = read_user_config(dir);
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-5")
    );
    // @step And the shared config contains tui.thinkingLevels with effort:high for anthropic/claude-opus-4-5
    assert_eq!(
        config["tui"]["thinkingLevels"][OPUS].as_str(),
        Some("effort:high")
    );
}

/// Scenario: Project scope overrides the user value on load
#[test]
fn project_scope_overrides_the_user_value_on_load() {
    // @step Given a user shared config with tui.thinkingLevels holding effort:low for anthropic/claude-opus-4-5
    let data_tmp = tempfile::tempdir().expect("data tempdir");
    let data_dir = data_tmp.path();
    std::fs::write(
        data_dir.join("fspec-config.json"),
        r#"{"tui":{"thinkingLevels":{"anthropic/claude-opus-4-5":"effort:low"}}}"#,
    )
    .expect("seed user");
    // @step And a project shared config with tui.thinkingLevels holding effort:high for anthropic/claude-opus-4-5
    let cwd_tmp = tempfile::tempdir().expect("cwd tempdir");
    let cwd = cwd_tmp.path();
    std::fs::create_dir_all(cwd.join("spec")).expect("spec dir");
    std::fs::write(
        cwd.join("spec").join("fspec-config.json"),
        r#"{"tui":{"thinkingLevels":{"anthropic/claude-opus-4-5":"effort:high"}}}"#,
    )
    .expect("seed project");
    // @step When the saved thinking level for anthropic/claude-opus-4-5 is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(data_dir, cwd, OPUS);
    // @step Then the loaded thinking level is High
    assert_eq!(loaded, Ok(Some(ThinkingLevel::Effort("high".to_string()))));
}

/// Scenario: A missing config file has no saved level
#[test]
fn a_missing_config_file_has_no_saved_level() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the saved thinking level for anthropic/claude-opus-4-5 is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(dir, dir, OPUS);
    // @step Then there is no saved thinking level, not Off
    assert_eq!(loaded, Ok(None));
}

/// Scenario: The global wrappers round-trip via the shared config
#[test]
fn the_global_wrappers_round_trip_via_the_shared_config() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given the global data directory is rooted at a throwaway directory
    let tmp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(tmp.path().to_path_buf()).expect("set data dir");
    // @step When the thinking level High is saved for anthropic/claude-opus-4-5 via the global wrapper
    save_default_thinking_level(OPUS, ThinkingLevel::Effort("high".to_string()))
        .expect("save via global wrapper");
    // @step And the saved thinking level for anthropic/claude-opus-4-5 is loaded via the global wrapper
    let loaded = load_default_thinking_level(OPUS);
    // @step Then the loaded thinking level is High
    assert_eq!(loaded, Ok(Some(ThinkingLevel::Effort("high".to_string()))));
}
