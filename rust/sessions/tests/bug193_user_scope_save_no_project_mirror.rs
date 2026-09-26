//! Feature: spec/features/user-scope-config-saves-never-mirror-project-keys.feature
//!
//! BUG-193 — user-scope config writers (mux save, default-thinking-level
//! save) must never mirror project-scope keys (`tools`, `agent`) into the
//! user-scope `fspec-config.json`. These tests drive the path-injectable
//! save cores against OS temp dirs (user dir + project cwd) and assert:
//!
//!   * a save writes the target key,
//!   * user-scope siblings survive,
//!   * project-scope keys do NOT leak into the user file,
//!   * project-scope tui.mux STILL overrides on load (load behavior kept).
//!
//! Red-phase proof: before the fix these save cores read the
//! deep-merged view (`load_config_with_dirs`) and wrote it wholesale to
//! user scope, so the "does NOT contain tools/agent" assertions fail.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use codelet_rpc_types::ThinkingLevel;
use codelet_sessions::default_thinking_level_persistence::save_default_thinking_level_with_dirs;
use codelet_sessions::mux_config_persistence::{
    load_mux_config_with_dirs, save_mux_config_with_dirs,
};
use serde_json::{json, Value};

const MUX_JSON: &str = r#"{"orientation":"Horizontal","splits":[46],"panes":["Board","Agent"],"focused_pane":1,"enabled":true}"#;

/// Seed a user-scope config file (the `~/.fspec` equivalent).
fn seed_user(data_dir: &std::path::Path, content: &str) {
    fs::create_dir_all(data_dir).expect("mkdir user dir");
    fs::write(data_dir.join("fspec-config.json"), content).expect("seed user config");
}

/// Seed a project-scope config file (`<cwd>/spec/fspec-config.json`).
fn seed_project(cwd: &std::path::Path, content: &str) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec");
    fs::write(cwd.join("spec").join("fspec-config.json"), content).expect("seed project config");
}

/// Read the user-scope config back as a `Value`.
fn read_user(data_dir: &std::path::Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(data_dir.join("fspec-config.json")).expect("read user config"),
    )
    .expect("parse user config")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Saving the mux config does not mirror project-scope keys into
// the user config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Saving the mux config does not mirror project-scope keys into the user config
#[test]
fn mux_save_does_not_mirror_project_scope_keys_into_the_user_config() {
    // @step Given the user-scope fspec-config.json holds providers and rlcd sections
    let user = tempfile::tempdir().expect("user dir");
    seed_user(
        user.path(),
        r#"{"providers":{"openai":{"profiles":{}}},"rlcd":{"url":"http://127.0.0.1:8000"}}"#,
    );
    // @step And the project spec/fspec-config.json holds agent and tools.test keys
    let cwd = tempfile::tempdir().expect("project dir");
    seed_project(
        cwd.path(),
        r#"{"agent":"claude","tools":{"test":{"command":"JAVA_HOME=./vendor/jdk-17 mvn test"}}}"#,
    );
    // @step When the mux config is saved to the user scope
    save_mux_config_with_dirs(
        user.path(),
        cwd.path(),
        &serde_json::from_str(MUX_JSON).unwrap(),
    )
    .expect("mux save");
    // @step Then the user-scope fspec-config.json contains the saved tui.mux value
    let config = read_user(user.path());
    assert_eq!(
        config["tui"]["mux"]["orientation"],
        json!("Horizontal"),
        "tui.mux must be persisted: {config}"
    );
    // @step And the user-scope fspec-config.json still contains the providers and rlcd sections
    assert!(
        config["providers"].is_object(),
        "providers must survive: {config}"
    );
    assert_eq!(config["rlcd"]["url"], json!("http://127.0.0.1:8000"));
    // @step And the user-scope fspec-config.json does NOT contain the tools key
    assert!(
        config.get("tools").is_none(),
        "project-scope 'tools' must NOT be mirrored into user scope: {config}"
    );
    // @step And the user-scope fspec-config.json does NOT contain the agent key
    assert!(
        config.get("agent").is_none(),
        "project-scope 'agent' must NOT be mirrored into user scope: {config}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Saving the default thinking level does not mirror
// project-scope keys into the user config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Saving the default thinking level does not mirror project-scope keys into the user config
#[test]
fn thinking_level_save_does_not_mirror_project_scope_keys_into_the_user_config() {
    // @step Given the project spec/fspec-config.json holds agent and tools keys
    let cwd = tempfile::tempdir().expect("project dir");
    seed_project(
        cwd.path(),
        r#"{"agent":"cursor","tools":{"qualityCheck":{"commands":["cargo clippy"]}}}"#,
    );
    let user = tempfile::tempdir().expect("user dir");
    // @step When the default thinking level High is saved to the user scope
    save_default_thinking_level_with_dirs(user.path(), cwd.path(), ThinkingLevel::High)
        .expect("save thinking level");
    // @step Then the user-scope fspec-config.json contains tui.defaultThinkingLevel equal to 3
    let config = read_user(user.path());
    assert_eq!(
        config["tui"]["defaultThinkingLevel"].as_u64(),
        Some(3),
        "defaultThinkingLevel must be persisted: {config}"
    );
    // @step And the user-scope fspec-config.json does NOT contain the tools key
    assert!(
        config.get("tools").is_none(),
        "project-scope 'tools' must NOT be mirrored into user scope: {config}"
    );
    // @step And the user-scope fspec-config.json does NOT contain the agent key
    assert!(
        config.get("agent").is_none(),
        "project-scope 'agent' must NOT be mirrored into user scope: {config}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A user-scope mux save preserves user-scope siblings
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: A user-scope mux save preserves user-scope siblings
#[test]
fn a_user_scope_mux_save_preserves_user_scope_siblings() {
    // @step Given the user-scope fspec-config.json holds tui.lastUsedModel and tui.defaultThinkingLevel
    let user = tempfile::tempdir().expect("user dir");
    seed_user(
        user.path(),
        r#"{"tui":{"lastUsedModel":"anthropic/claude-opus-4-5","defaultThinkingLevel":1}}"#,
    );
    let cwd = tempfile::tempdir().expect("project dir");
    // @step When the mux config is saved to the user scope
    save_mux_config_with_dirs(
        user.path(),
        cwd.path(),
        &serde_json::from_str(MUX_JSON).unwrap(),
    )
    .expect("mux save");
    // @step Then the user-scope fspec-config.json contains the saved tui.mux value
    let config = read_user(user.path());
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must be persisted: {config}"
    );
    // @step And the user-scope fspec-config.json still contains tui.lastUsedModel unchanged
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-5"),
        "sibling tui.lastUsedModel must survive: {config}"
    );
    // @step And the user-scope fspec-config.json still contains tui.defaultThinkingLevel unchanged
    assert_eq!(
        config["tui"]["defaultThinkingLevel"].as_u64(),
        Some(1),
        "sibling tui.defaultThinkingLevel must survive: {config}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Saving to a user scope without any user config file still
// succeeds
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Saving to a user scope without any user config file still succeeds
#[test]
fn saving_without_any_user_config_file_still_succeeds() {
    // @step Given the user-scope directory has no fspec-config.json file
    let user = tempfile::tempdir().expect("user dir");
    // @step And the project spec/fspec-config.json holds agent and tools keys
    let cwd = tempfile::tempdir().expect("project dir");
    seed_project(
        cwd.path(),
        r#"{"agent":"claude","tools":{"test":{"command":"npm test"}}}"#,
    );
    // @step When the mux config is saved to the user scope
    save_mux_config_with_dirs(
        user.path(),
        cwd.path(),
        &serde_json::from_str(MUX_JSON).unwrap(),
    )
    .expect("mux save");
    // @step Then the user-scope fspec-config.json is created with only the tui.mux value
    let config = read_user(user.path());
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must be persisted: {config}"
    );
    assert_eq!(
        config.as_object().unwrap().len(),
        1,
        "the fresh user config must contain ONLY 'tui': {config}"
    );
    // @step And the user-scope fspec-config.json does NOT contain the tools or agent keys
    assert!(config.get("tools").is_none());
    assert!(config.get("agent").is_none());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A malformed user config file degrades to a fresh save without
// mirroring project keys
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: A malformed user config file degrades to a fresh save without mirroring project keys
#[test]
fn a_malformed_user_config_degrades_to_a_fresh_save_without_mirroring_project_keys() {
    // @step Given the user-scope fspec-config.json contains invalid JSON
    let user = tempfile::tempdir().expect("user dir");
    seed_user(user.path(), "{ not json");
    // @step And the project spec/fspec-config.json holds agent and tools keys
    let cwd = tempfile::tempdir().expect("project dir");
    seed_project(
        cwd.path(),
        r#"{"agent":"claude","tools":{"test":{"command":"mvn test"}}}"#,
    );
    // @step When the mux config is saved to the user scope
    save_mux_config_with_dirs(
        user.path(),
        cwd.path(),
        &serde_json::from_str(MUX_JSON).unwrap(),
    )
    .expect("mux save must not fail on a malformed user file (existing best-effort contract)");
    // @step Then the user-scope fspec-config.json contains the saved tui.mux value
    let config = read_user(user.path());
    assert!(
        config["tui"]["mux"].is_object(),
        "tui.mux must be persisted: {config}"
    );
    // @step And the user-scope fspec-config.json does NOT contain the tools or agent keys
    assert!(
        config.get("tools").is_none(),
        "must not mirror 'tools' from the project file: {config}"
    );
    assert!(
        config.get("agent").is_none(),
        "must not mirror 'agent' from the project file: {config}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Project-scope tui.mux still overrides the user-scope value on
// load (regression guard — load behavior must NOT change)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Project-scope tui.mux still overrides the user-scope value on load
#[test]
fn project_scope_tui_mux_still_overrides_the_user_scope_value_on_load() {
    // @step Given the user-scope fspec-config.json holds tui.mux with orientation Horizontal
    let user = tempfile::tempdir().expect("user dir");
    seed_user(
        user.path(),
        r#"{"tui":{"mux":{"orientation":"Horizontal","splits":[50],"panes":["Board","Agent"],"focused_pane":1,"enabled":true}}}"#,
    );
    // @step And the project spec/fspec-config.json holds tui.mux with orientation Vertical
    let cwd = tempfile::tempdir().expect("project dir");
    seed_project(
        cwd.path(),
        r#"{"tui":{"mux":{"orientation":"Vertical","splits":[40],"panes":["Board","Agent"],"focused_pane":1,"enabled":true}}}"#,
    );
    // @step When the mux config is loaded for the project
    let value =
        load_mux_config_with_dirs(user.path(), cwd.path()).expect("project tui.mux must load");
    // @step Then the loaded mux orientation is Vertical
    assert_eq!(
        value["orientation"],
        json!("Vertical"),
        "the project-scope value must still win on load: {value}"
    );
}
