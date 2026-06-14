//! CLI surface for the `validate-hooks` subcommand on the standalone fspec
//! Rust binary — RPC-322.
//!
//! Feature: spec/features/validate-hooks-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + core impl are not yet wired,
//! so these tests are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::Value;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_validate_hooks(cwd: &Path) -> (i32, String, String) {
    let output = Command::new(fspec_bin())
        .arg("validate-hooks")
        .current_dir(cwd)
        .output()
        .expect("spawn fspec validate-hooks");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_hooks(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

fn write_script(root: &Path, rel: &str) {
    let abs = root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir script parent");
    fs::write(&abs, "#!/bin/sh\necho hi\n").expect("write hook script");
}

const TS_HELP_FIXTURE_VH: &str = include_str!("fixtures/help/validate-hooks.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: validate-hooks --help is byte-for-byte identical to the TS reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_validate_hooks_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec validate-hooks --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("validate-hooks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "validate-hooks --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/validate-hooks.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_VH);

    // @step And stdout starts with a blank line followed by 'VALIDATE-HOOKS'
    assert!(stdout.starts_with("\nVALIDATE-HOOKS\n"), "got:\n{stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints success and exits 0 when all hook scripts exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_all_scripts_exist_prints_success_exits_0() {
    // @step Given spec/fspec-hooks.json configures one hook whose command script exists on disk
    let ws = tempfile::tempdir().expect("tempdir");
    write_script(ws.path(), "spec/hooks/lint.sh");
    write_hooks(
        ws.path(),
        r#"{ "hooks": { "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ] } }"#,
    );

    // @step When I run `./codelet/target/release/fspec validate-hooks`
    let (code, stdout, stderr) = run_validate_hooks(ws.path());

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ All hooks are valid'
    assert!(
        stdout.contains("✓ All hooks are valid"),
        "expected success message on stdout; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports missing scripts and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_missing_scripts_reports_and_exits_1() {
    // @step Given spec/fspec-hooks.json configures a hook with command 'spec/hooks/lint.sh' that does not exist on disk
    let ws = tempfile::tempdir().expect("tempdir");
    write_hooks(
        ws.path(),
        r#"{ "hooks": { "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ] } }"#,
    );

    // @step When I run `./codelet/target/release/fspec validate-hooks`
    let (code, stdout, stderr) = run_validate_hooks(ws.path());

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✗ Hook validation failed'
    assert!(
        stdout.contains("✗ Hook validation failed"),
        "expected failure header on stdout; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Hook command not found: spec/hooks/lint.sh'
    assert!(
        stdout.contains("Hook command not found: spec/hooks/lint.sh"),
        "expected the missing-script line on stdout; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a load failure and exits 1 when the config is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_missing_config_reports_load_failure_exits_1() {
    // @step Given an empty directory with no spec/fspec-hooks.json is set as the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/fspec-hooks.json").exists());

    // @step When I run `./codelet/target/release/fspec validate-hooks`
    let (code, stdout, stderr) = run_validate_hooks(ws.path());

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring 'Failed to load hook configuration'
    assert!(
        stdout.contains("Failed to load hook configuration"),
        "expected the load-failure message on stdout; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/fspec-hooks.json references a missing hook script
    let ws = tempfile::tempdir().expect("tempdir");
    write_hooks(
        ws.path(),
        r#"{ "hooks": { "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ] } }"#,
    );

    // @step When I dispatch validate-hooks through fspec_core::dispatch::dispatch_command and also run `./codelet/target/release/fspec validate-hooks` against the same on-disk state
    let req = codelet_fspec_core::DispatchRequest {
        command: "validate-hooks".to_string(),
        args_json: "{}".to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then both paths agree the configuration is invalid
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "dispatcher must report valid=false; got {data}"
    );
    let (code, stdout, stderr) = run_validate_hooks(ws.path());
    assert_eq!(
        code, 1,
        "CLI must agree the config is invalid by exiting 1; stdout={stdout}, stderr={stderr}"
    );

    // @step Then the CLI bridge module codelet/fspec/src/validate_hooks.rs contains NO inline validation logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/validate_hooks.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/validate_hooks.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Hook command not found",
        "fspec-hooks.json",
        "Failed to load hook configuration",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
