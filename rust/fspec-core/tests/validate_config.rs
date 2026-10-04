#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/config-scope-validation.feature
//
// CONFIG-009 — Dispatcher-contract tests for the NEW `validate-config`
// command. Each scenario maps to exactly one #[test] with @step comments
// mirroring the Gherkin steps.
//
// RED PHASE: the command does not exist yet, so the dispatcher returns
// UnknownCommand and these tests FAIL. They assert the real expected
// behaviour the implementation must satisfy.
//
// User-dir resolution under test: `validate-config` resolves the user-scope
// config from the `FSPEC_USER_DIR` env override (then `HOME/.fspec`). Every
// test points `FSPEC_USER_DIR` at a fresh temp dir so a developer's real
// `~/.fspec` never bleeds in; env mutation is process-global, so all tests
// are `#[serial]`.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path) -> DispatchRequest {
    DispatchRequest {
        command: "validate-config".to_string(),
        args_json: json!({}).to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Point `FSPEC_USER_DIR` at `dir` for the duration of `f`, then clear it.
/// Returns whatever `f` returns.
fn with_user_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    std::env::set_var("FSPEC_USER_DIR", dir);
    let result = f();
    std::env::remove_var("FSPEC_USER_DIR");
    result
}

/// Write a user-scope `fspec-config.json` inside `dir` (creates `dir` first).
fn write_user_config(dir: &Path, body: &str) {
    fs::create_dir_all(dir).expect("mkdir user dir");
    fs::write(dir.join("fspec-config.json"), body).expect("write user config");
}

fn envelope(result: &codelet_fspec_core::DispatchResult) -> Value {
    assert!(
        result.success,
        "dispatcher must return success=true; got {result:?}"
    );
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("envelope must be JSON: {e}; got:\n{}", result.data))
}

// ---------- scenarios ----------

#[test]
#[serial]
fn project_scope_keys_in_user_config_are_flagged_as_warnings() {
    // @step Given a user-scope config file containing tools, agent, and providers keys
    let user_dir = TempDir::new().expect("tempdir");
    write_user_config(
        user_dir.path(),
        r#"{"providers":{"openai":{}},"tools":{"test":{"command":"mvn test"}},"agent":"claude"}"#,
    );
    let project_root = TempDir::new().expect("tempdir");

    // @step When I run validate-config
    let env = with_user_dir(user_dir.path(), || {
        envelope(&dispatch_command(req(project_root.path())))
    });

    // @step Then validate-config reports a tools warning with remediation text
    assert!(
        env["warnings"]
            .as_array()
            .map(|a| a.iter().any(|w| w.as_str() == Some("`tools` is a project-scope key: Remove `tools` from your user-scope config; configure it per-project with `fspec configure-tools` (writes spec/fspec-config.json).")))
            .unwrap_or(false),
        "tools must be flagged; got: {env}"
    );
    assert!(
        env["message"]
            .as_str()
            .unwrap_or("")
            .contains("fspec configure-tools"),
        "tools warning must carry the configure-tools remediation; got: {env}"
    );

    // @step And validate-config reports an agent warning with remediation text
    assert!(
        env["warnings"]
            .as_array()
            .map(|a| a.iter().any(|w| w.as_str() == Some(
                "`agent` is a project-scope key: Remove `agent` from your user-scope config; it is set per-project by `fspec init`."
            )))
            .unwrap_or(false),
        "agent must be flagged; got: {env}"
    );
    assert!(
        env["message"].as_str().unwrap_or("").contains("fspec init"),
        "agent warning must carry the init remediation; got: {env}"
    );

    // Warnings are non-fatal: the envelope stays valid with exitCode 0.
    assert_eq!(
        env["valid"].as_bool(),
        Some(true),
        "warnings must not fail the run; got: {env}"
    );
    assert_eq!(
        env["exitCode"].as_u64(),
        Some(0),
        "exitCode must stay 0; got: {env}"
    );
}

#[test]
#[serial]
fn a_user_scope_only_config_passes_validation_without_warnings() {
    // @step Given a user-scope config file containing only providers, tui, rlcd, and research keys
    let user_dir = TempDir::new().expect("tempdir");
    write_user_config(
        user_dir.path(),
        r#"{"providers":{"openai":{}},"tui":{"lastUsedModel":"m"},"rlcd":{"url":"http://x"},"research":{"perplexity":{"apiKey":"k"}}}"#,
    );
    let project_root = TempDir::new().expect("tempdir");

    // @step When I run validate-config
    let env = with_user_dir(user_dir.path(), || {
        envelope(&dispatch_command(req(project_root.path())))
    });

    // @step Then validate-config reports zero project-scope warnings
    assert_eq!(
        env["warnings"].as_array().map(Vec::len),
        Some(0),
        "no warnings expected; got: {env}"
    );
    assert_eq!(env["valid"].as_bool(), Some(true));
    assert_eq!(env["exitCode"].as_u64(), Some(0));
    assert!(
        env["message"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("clean"),
        "clean message expected; got: {env}"
    );
}

#[test]
#[serial]
fn a_missing_user_config_file_validates_cleanly() {
    // @step Given no user-scope config file exists at all
    let user_dir = TempDir::new().expect("tempdir"); // empty dir, no fspec-config.json
    let project_root = TempDir::new().expect("tempdir");

    // @step When I run validate-config
    let env = with_user_dir(user_dir.path(), || {
        envelope(&dispatch_command(req(project_root.path())))
    });

    // @step Then validate-config reports zero project-scope warnings and does not fail
    assert_eq!(
        env["warnings"].as_array().map(Vec::len),
        Some(0),
        "no warnings expected; got: {env}"
    );
    assert_eq!(
        env["valid"].as_bool(),
        Some(true),
        "missing file must not fail; got: {env}"
    );
    assert_eq!(env["exitCode"].as_u64(), Some(0));
}

#[test]
#[serial]
fn a_malformed_user_config_file_is_reported_as_a_warning_not_a_crash() {
    // @step Given a user-scope config file containing invalid JSON
    let user_dir = TempDir::new().expect("tempdir");
    write_user_config(user_dir.path(), "{ not valid json");
    let project_root = TempDir::new().expect("tempdir");

    // @step When I run validate-config
    let env = with_user_dir(user_dir.path(), || {
        envelope(&dispatch_command(req(project_root.path())))
    });

    // @step Then validate-config reports a parse warning and does not crash
    assert_eq!(
        env["valid"].as_bool(),
        Some(true),
        "a parse warning must not fail the run; got: {env}"
    );
    assert_eq!(env["exitCode"].as_u64(), Some(0));
    assert!(
        env["message"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("malformed"),
        "message must name the malformed user config; got: {env}"
    );
}
