//! CLI surface for the `remove-persona` subcommand on the standalone
//! fspec Rust binary — RPC-277.
//!
//! Feature: spec/features/remove-persona-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_remove_persona(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-persona");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-persona");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(cwd: &Path, name: &str, value: &serde_json::Value) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join(name),
        serde_json::to_string_pretty(value).expect("ser") + "\n",
    )
    .expect("write file");
}

fn read_json(cwd: &Path, name: &str) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec").join(name)).expect("read file");
    serde_json::from_str(&raw).expect("parse json")
}

fn read_raw(cwd: &Path, name: &str) -> String {
    fs::read_to_string(cwd.join("spec").join(name)).expect("read raw")
}

fn persona(name: &str) -> serde_json::Value {
    serde_json::json!({"name": name, "description": "d", "goals": []})
}

fn foundation_with(personas: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "personas": personas
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-persona.txt");

#[test]
fn scenario_help_output_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec remove-persona --help`
    let output = Command::new(fspec_bin())
        .arg("remove-persona")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-persona --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "remove-persona --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-persona.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'REMOVE-PERSONA'
    assert!(
        stdout.starts_with("\nREMOVE-PERSONA"),
        "stdout must start with blank line + REMOVE-PERSONA; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes a persona and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_persona_and_prints_success_line() {
    // @step Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "foundation.json",
        &foundation_with(serde_json::json!([persona("Primary User"), persona("Admin")])),
    );

    // @step When I run `fspec remove-persona "Admin"` in that tempdir
    let (code, stdout, stderr) = run_remove_persona(ws.path(), &["Admin"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Removed persona "Admin" from foundation.json'
    assert!(
        stdout.contains("✓ Removed persona \"Admin\" from foundation.json"),
        "missing Removed line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows personas has length 1
    let f = read_json(ws.path(), "foundation.json");
    assert_eq!(f["personas"].as_array().expect("array").len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes from the draft when foundation.json.draft exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_from_draft_when_present() {
    // @step Given a project root tempdir with spec/foundation.json.draft containing persona 'Drafted'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "foundation.json.draft",
        &foundation_with(serde_json::json!([persona("Drafted")])),
    );

    // @step When I run `fspec remove-persona "Drafted"` in that tempdir
    let (code, stdout, stderr) = run_remove_persona(ws.path(), &["Drafted"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Removed persona "Drafted" from foundation.json.draft'
    assert!(
        stdout.contains("✓ Removed persona \"Drafted\" from foundation.json.draft"),
        "missing draft Removed line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a non-existent persona with exit 1 and the available names
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_nonexistent_persona_with_exit_1() {
    // @step Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "foundation.json",
        &foundation_with(serde_json::json!([persona("Primary User"), persona("Admin")])),
    );
    let before = read_raw(ws.path(), "foundation.json");

    // @step When I run `fspec remove-persona "Ghost"` in that tempdir
    let (code, _stdout, stderr) = run_remove_persona(ws.path(), &["Ghost"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Persona "Ghost" not found'
    assert!(
        stderr.contains("Persona \"Ghost\" not found"),
        "stderr must mention not found; got:\n{stderr}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_raw(ws.path(), "foundation.json"), before);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports the empty-personas case with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_empty_personas_with_exit_1() {
    // @step Given a project root tempdir with spec/foundation.json whose personas array is empty
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_with(serde_json::json!([])));

    // @step When I run `fspec remove-persona "Admin"` in that tempdir
    let (code, _stdout, stderr) = run_remove_persona(ws.path(), &["Admin"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Persona "Admin" not found'
    assert!(
        stderr.contains("Persona \"Admin\" not found"),
        "stderr must mention not found; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports the missing-foundation error with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_foundation_with_exit_1() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `fspec remove-persona "Admin"` in that tempdir
    let (code, _stdout, stderr) = run_remove_persona(ws.path(), &["Admin"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'foundation.json not found'
    assert!(
        stderr.contains("foundation.json not found"),
        "stderr must mention missing foundation; got:\n{stderr}"
    );

    // @step And spec/foundation.json does not exist on disk
    assert!(!ws.path().join("spec/foundation.json").exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/foundation.json containing personas 'Primary User', 'Admin' and 'Guest'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "foundation.json",
        &foundation_with(serde_json::json!([
            persona("Primary User"),
            persona("Admin"),
            persona("Guest")
        ])),
    );

    // @step When I dispatch remove-persona via fspec_core::dispatch::dispatch_command with name='Guest'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-persona".to_string(),
        args_json: r#"{"name":"Guest"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step And running `fspec remove-persona "Admin"` afterwards exits 0
    let (code, stdout, stderr) = run_remove_persona(ws.path(), &["Admin"]);
    assert_eq!(code, 0, "CLI remove must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/foundation.json on disk shows personas has length 1
    let f = read_json(ws.path(), "foundation.json");
    assert_eq!(f["personas"].as_array().expect("array").len(), 1);

    // @step And the CLI bridge module codelet/fspec/src/remove_persona.rs contains NO inline persona-match, file-read, or file-write logic — its only computation is JSON arg marshalling and stdout rendering
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_persona.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_persona.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "find",
        "read_to_string",
        "write_json_atomic",
        "Available personas",
        "foundation.json.draft",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
