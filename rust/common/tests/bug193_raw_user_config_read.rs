//! Feature: spec/features/user-scope-config-saves-never-mirror-project-keys.feature
//!
//! BUG-193 — user-scope config writers must never read the deep-merged
//! (project-over-user) view when saving to user scope. This test file pins
//! the new `load_user_config_file` core in `codelet_common::fspec_config`:
//! a RAW user-file read with the same semantics as `load_config_file`
//! (missing/empty -> empty object, invalid JSON -> Err) but WITHOUT the
//! project-scope merge. The session-level save-core tests live in
//! `rust/sessions/tests/bug193_user_scope_save_no_project_mirror.rs`.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use codelet_common::fspec_config::{
    load_config_with_dirs, load_user_config_file, project_config_path, user_config_path,
};
use serde_json::json;

/// Scenario: The raw user-file read ignores the project-scope file entirely
#[test]
fn raw_user_file_read_ignores_the_project_scope_file() {
    // @step Given the user-scope fspec-config.json holds providers and rlcd sections
    let user = tempfile::tempdir().unwrap();
    fs::create_dir_all(user.path()).unwrap();
    fs::write(
        user_config_path(user.path()),
        r#"{"providers": {"openai": {}}, "rlcd": {"url": "http://127.0.0.1:8000"}}"#,
    )
    .unwrap();
    // @step And the project spec/fspec-config.json holds agent and tools.test keys
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("spec")).unwrap();
    fs::write(
        project_config_path(project.path()),
        r#"{"agent": "claude", "tools": {"test": {"command": "mvn test"}}}"#,
    )
    .unwrap();
    // @step When the raw user-scope config is loaded
    let loaded = load_user_config_file(user.path()).unwrap();
    // @step Then the loaded value holds ONLY the user-scope keys
    assert!(loaded.get("providers").is_some());
    assert!(loaded.get("rlcd").is_some());
    assert!(
        loaded.get("agent").is_none(),
        "agent must NOT leak from project scope"
    );
    assert!(
        loaded.get("tools").is_none(),
        "tools must NOT leak from project scope"
    );
    // @step And the deep-merged view still includes the project keys (load behavior unchanged)
    let merged = load_config_with_dirs(user.path(), project.path()).unwrap();
    assert_eq!(merged["agent"], json!("claude"));
    assert!(merged["tools"].is_object());
}

/// Scenario: A missing user file loads as an empty object
#[test]
fn missing_user_file_loads_as_an_empty_object() {
    // @step Given the user-scope directory has no fspec-config.json file
    let user = tempfile::tempdir().unwrap();
    // @step When the raw user-scope config is loaded
    let loaded = load_user_config_file(user.path()).unwrap();
    // @step Then the loaded value is an empty JSON object
    assert!(loaded.is_object());
    assert!(loaded.as_object().unwrap().is_empty());
}

/// Scenario: An empty user file loads as an empty object
#[test]
fn an_empty_user_file_loads_as_an_empty_object() {
    // @step Given the user-scope fspec-config.json contains only whitespace
    let user = tempfile::tempdir().unwrap();
    fs::create_dir_all(user.path()).unwrap();
    fs::write(user_config_path(user.path()), "   \n").unwrap();
    // @step When the raw user-scope config is loaded
    let loaded = load_user_config_file(user.path()).unwrap();
    // @step Then the loaded value is an empty JSON object
    assert!(loaded.as_object().unwrap().is_empty());
}

/// Scenario: A malformed user file surfaces a parse error (never a silent merge)
#[test]
fn a_malformed_user_file_surfaces_a_parse_error() {
    // @step Given the user-scope fspec-config.json contains invalid JSON
    let user = tempfile::tempdir().unwrap();
    fs::create_dir_all(user.path()).unwrap();
    fs::write(user_config_path(user.path()), "{ not json").unwrap();
    // @step When the raw user-scope config is loaded
    let result = load_user_config_file(user.path());
    // @step Then the load fails with an error mentioning the invalid JSON
    let err = result.expect_err("malformed JSON must surface Err");
    assert!(
        err.contains("Invalid JSON"),
        "error must name the invalid-JSON failure: {err}"
    );
}
