//! Feature: spec/features/default-thinking-level-shared-config-persistence.feature
//!
//! TUI-092 — the default thinking level is persisted to the shared CONFIG-008
//! `fspec-config.json` under the nested key `tui.defaultThinkingLevel` (int
//! `0..=3`). These integration tests exercise the path-injectable
//! `*_with_dirs` cores against OS temp dirs (no fs mocking, no `$HOME`
//! mutation) and assert the on-disk JSON layout, sibling-key preservation,
//! project-scope override, graceful degradation, and the global wrapper
//! round-trip.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Mutex;

use codelet_rpc_types::ThinkingLevel;
use codelet_sessions::default_thinking_level_persistence::{
    load_default_thinking_level, load_default_thinking_level_with_dirs,
    save_default_thinking_level, save_default_thinking_level_with_dirs,
};
use serde_json::Value;

/// Serialises tests touching the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Read `<data_dir>/fspec-config.json` as a `serde_json::Value`.
fn read_user_config(data_dir: &std::path::Path) -> Value {
    let raw = std::fs::read_to_string(data_dir.join("fspec-config.json"))
        .expect("read fspec-config.json");
    serde_json::from_str(&raw).expect("parse fspec-config.json")
}

/// Scenario: Saving High writes the nested tui.defaultThinkingLevel key
#[test]
fn saving_high_writes_the_nested_default_thinking_level_key() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the default thinking level High is saved to the user scope
    save_default_thinking_level_with_dirs(dir, dir, ThinkingLevel::High).expect("save High");
    // @step Then the file fspec-config.json under the data directory contains tui.defaultThinkingLevel equal to 3
    let config = read_user_config(dir);
    assert_eq!(config["tui"]["defaultThinkingLevel"].as_u64(), Some(3));
}

/// Scenario: Saved level round-trips through the shared config
#[test]
fn saved_level_round_trips_through_the_shared_config() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the default thinking level Medium is saved to the user scope
    save_default_thinking_level_with_dirs(dir, dir, ThinkingLevel::Medium).expect("save Medium");
    // @step And the default thinking level is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is Medium
    assert_eq!(loaded, ThinkingLevel::Medium);
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
    // @step When the default thinking level High is saved to the user scope
    save_default_thinking_level_with_dirs(dir, dir, ThinkingLevel::High).expect("save High");
    // @step Then the shared config still contains tui.lastUsedModel equal to "anthropic/claude-opus-4-5"
    let config = read_user_config(dir);
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-5")
    );
    // @step And the shared config contains tui.defaultThinkingLevel equal to 3
    assert_eq!(config["tui"]["defaultThinkingLevel"].as_u64(), Some(3));
}

/// Scenario: Project scope overrides the user value on load
#[test]
fn project_scope_overrides_the_user_value_on_load() {
    // @step Given a user shared config with tui.defaultThinkingLevel set to 1
    let data_tmp = tempfile::tempdir().expect("data tempdir");
    let data_dir = data_tmp.path();
    std::fs::write(
        data_dir.join("fspec-config.json"),
        r#"{"tui":{"defaultThinkingLevel":1}}"#,
    )
    .expect("seed user");
    // @step And a project shared config with tui.defaultThinkingLevel set to 3
    let cwd_tmp = tempfile::tempdir().expect("cwd tempdir");
    let cwd = cwd_tmp.path();
    std::fs::create_dir_all(cwd.join("spec")).expect("spec dir");
    std::fs::write(
        cwd.join("spec").join("fspec-config.json"),
        r#"{"tui":{"defaultThinkingLevel":3}}"#,
    )
    .expect("seed project");
    // @step When the default thinking level is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(data_dir, cwd);
    // @step Then the loaded default thinking level is High
    assert_eq!(loaded, ThinkingLevel::High);
}

/// Scenario: An out-of-range value loads as Off
#[test]
fn an_out_of_range_value_loads_as_off() {
    // @step Given a user shared config with tui.defaultThinkingLevel set to 7
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    std::fs::write(
        dir.join("fspec-config.json"),
        r#"{"tui":{"defaultThinkingLevel":7}}"#,
    )
    .expect("seed");
    // @step When the default thinking level is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is Off
    assert_eq!(loaded, ThinkingLevel::Off);
}

/// Scenario: A missing config file loads as Off
#[test]
fn a_missing_config_file_loads_as_off() {
    // @step Given a data directory with no persisted shared config
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    // @step When the default thinking level is loaded from the shared config
    let loaded = load_default_thinking_level_with_dirs(dir, dir);
    // @step Then the loaded default thinking level is Off
    assert_eq!(loaded, ThinkingLevel::Off);
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
    // @step When the default thinking level High is saved via the global wrapper
    save_default_thinking_level(ThinkingLevel::High).expect("save via global wrapper");
    // @step And the default thinking level is loaded via the global wrapper
    let loaded = load_default_thinking_level();
    // @step Then the loaded default thinking level is High
    assert_eq!(loaded, ThinkingLevel::High);
}
