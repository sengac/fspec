//! Feature: spec/features/shared-config-merge.feature
//!
//! CONFIG-008 — shared `fspec-config.json` module with two-scope deep merge.
//!
//! Integration tests using OS temp dirs (`tempfile`) against the path-injectable
//! `*_with_dirs` cores. No fs mocking; user scope is a temp `data_dir` and project
//! scope is a temp `cwd` (its `spec/` subdir holds the project file).

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use codelet_common::fspec_config::{
    load_config_with_dirs, write_config_with_dirs, ConfigScope,
};
use serde_json::{json, Value};

/// Write `content` to `<data_dir>/fspec-config.json` (user scope).
fn seed_user(data_dir: &Path, content: &str) {
    fs::create_dir_all(data_dir).expect("data dir");
    fs::write(data_dir.join("fspec-config.json"), content).expect("write user config");
}

/// Write `content` to `<cwd>/spec/fspec-config.json` (project scope).
fn seed_project(cwd: &Path, content: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("spec dir");
    fs::write(spec.join("fspec-config.json"), content).expect("write project config");
}

/// Scenario: User-only config loads when no project file exists
#[test]
fn user_only_config_loads_when_no_project_file_exists() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing {"model":"opus"}
    seed_user(data.path(), r#"{"model":"opus"}"#);
    // @step And no project config file exists
    // (cwd has no spec/fspec-config.json)
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {"model":"opus"}
    assert_eq!(loaded, json!({"model":"opus"}));
}

/// Scenario: Project config deep-merges over user config per key
#[test]
fn project_config_deep_merges_over_user_config_per_key() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing {"a":1,"b":2}
    seed_user(data.path(), r#"{"a":1,"b":2}"#);
    // @step And a project config file containing {"b":3}
    seed_project(cwd.path(), r#"{"b":3}"#);
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {"a":1,"b":3}
    assert_eq!(loaded, json!({"a":1,"b":3}));
}

/// Scenario: Nested objects merge recursively preserving sibling keys
#[test]
fn nested_objects_merge_recursively_preserving_sibling_keys() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing {"ui":{"theme":"dark","font":12}}
    seed_user(data.path(), r#"{"ui":{"theme":"dark","font":12}}"#);
    // @step And a project config file containing {"ui":{"theme":"light"}}
    seed_project(cwd.path(), r#"{"ui":{"theme":"light"}}"#);
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {"ui":{"theme":"light","font":12}}
    assert_eq!(loaded, json!({"ui":{"theme":"light","font":12}}));
}

/// Scenario: Arrays and scalars replace instead of merging
#[test]
fn arrays_and_scalars_replace_instead_of_merging() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing {"tags":["a","b"]}
    seed_user(data.path(), r#"{"tags":["a","b"]}"#);
    // @step And a project config file containing {"tags":["c"]}
    seed_project(cwd.path(), r#"{"tags":["c"]}"#);
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {"tags":["c"]}
    assert_eq!(loaded, json!({"tags":["c"]}));
}

/// Scenario: Missing files load as an empty object without error
#[test]
fn missing_files_load_as_an_empty_object_without_error() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given no user config file exists
    // @step And no project config file exists
    // (both temp dirs are empty)
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {}
    assert_eq!(loaded, json!({}));
}

/// Scenario: Whitespace-only file loads as an empty object
#[test]
fn whitespace_only_file_loads_as_an_empty_object() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing only whitespace
    seed_user(data.path(), "   \n\t  ");
    // @step And no project config file exists
    // @step When the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {}
    assert_eq!(loaded, json!({}));
}

/// Scenario: Invalid JSON returns an error naming the offending path
#[test]
fn invalid_json_returns_an_error_naming_the_offending_path() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing invalid JSON text "{invalid"
    seed_user(data.path(), "{invalid");
    // @step When the config is loaded
    let result = load_config_with_dirs(data.path(), cwd.path());
    // @step Then loading returns an error mentioning the user config path
    let err = result.expect_err("expected invalid JSON error");
    let expected_path = data.path().join("fspec-config.json");
    assert!(
        err.contains(&expected_path.to_string_lossy().to_string()),
        "error `{err}` should mention path `{}`",
        expected_path.display()
    );
    assert!(err.contains("Invalid JSON"), "error `{err}` should say Invalid JSON");
}

/// Scenario: Writing user scope creates the data directory and file
#[test]
fn writing_user_scope_creates_the_data_directory_and_file() {
    let data_parent = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a data directory that does not yet exist
    let data_dir = data_parent.path().join("nested").join("data");
    assert!(!data_dir.exists());
    // @step When the config {"x":1} is written to the user scope
    write_config_with_dirs(ConfigScope::User, &json!({"x":1}), &data_dir, cwd.path())
        .expect("write user");
    // @step Then the file <data_dir>/fspec-config.json exists with content {"x":1}
    let written = data_dir.join("fspec-config.json");
    assert!(written.exists(), "user config file should exist");
    let parsed: Value =
        serde_json::from_str(&fs::read_to_string(&written).expect("read")).expect("parse");
    assert_eq!(parsed, json!({"x":1}));
}

/// Scenario: Writing project scope creates the spec directory and file
#[test]
fn writing_project_scope_creates_the_spec_directory_and_file() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd_parent = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a working directory with no spec directory
    let cwd = cwd_parent.path().join("project");
    fs::create_dir_all(&cwd).expect("cwd");
    assert!(!cwd.join("spec").exists());
    // @step When the config {"y":2} is written to the project scope
    write_config_with_dirs(ConfigScope::Project, &json!({"y":2}), data.path(), &cwd)
        .expect("write project");
    // @step Then the file <cwd>/spec/fspec-config.json exists with content {"y":2}
    let written = cwd.join("spec").join("fspec-config.json");
    assert!(written.exists(), "project config file should exist");
    let parsed: Value =
        serde_json::from_str(&fs::read_to_string(&written).expect("read")).expect("parse");
    assert_eq!(parsed, json!({"y":2}));
}

/// Scenario: Round-trip write then load merges with the other scope
#[test]
fn round_trip_write_then_load_merges_with_the_other_scope() {
    let data = tempfile::tempdir().expect("data tempdir");
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // @step Given a user config file containing {"u":1}
    seed_user(data.path(), r#"{"u":1}"#);
    // @step When the config {"x":1} is written to the project scope
    write_config_with_dirs(ConfigScope::Project, &json!({"x":1}), data.path(), cwd.path())
        .expect("write project");
    // @step And the config is loaded
    let loaded = load_config_with_dirs(data.path(), cwd.path()).expect("load");
    // @step Then the loaded config equals {"u":1,"x":1}
    assert_eq!(loaded, json!({"u":1,"x":1}));
}
