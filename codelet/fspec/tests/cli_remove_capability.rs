//! CLI surface for the `remove-capability` subcommand on the standalone
//! fspec Rust binary — RPC-269.
//!
//! Feature: spec/features/remove-capability-cli-subcommand.feature
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

fn run_remove_cap(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-capability");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-capability");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path, value: &serde_json::Value) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser"),
    )
    .expect("write foundation.json");
}

fn read_foundation(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/foundation.json")).expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn caps(data: &serde_json::Value) -> &Vec<serde_json::Value> {
    data["solutionSpace"]["capabilities"]
        .as_array()
        .expect("capabilities array")
}

fn foundation_with_caps(capabilities: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": capabilities}
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes remove-capability with one positional arg in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_remove_capability_with_positional_arg_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-capability --help`
    let output = Command::new(fspec_bin())
        .arg("remove-capability")
        .arg("--help")
        .output()
        .expect("spawn remove-capability --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "remove-capability --help must exit 0; stderr={stderr}");

    // @step And stdout describes the remove-capability subcommand
    assert!(
        stdout.contains("remove-capability") || stdout.contains("REMOVE-CAPABILITY"),
        "help must describe remove-capability; got:\n{stdout}"
    );

    // @step And stdout mentions the `<name>` argument
    assert!(
        stdout.contains("name"),
        "help must mention name; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes a capability and prints the success line on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_capability_and_prints_success_line() {
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'User Authentication'},{name:'Search'}]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_caps(serde_json::json!([
            {"name": "User Authentication", "description": "a"},
            {"name": "Search", "description": "s"}
        ])),
    );

    // @step When I run `./codelet/target/release/fspec remove-capability "Search"`
    let (code, stdout, stderr) = run_remove_cap(ws.path(), &["Search"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Removed capability "Search" from foundation.json'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Removed capability \"Search\" from foundation.json")),
        "missing Removed line; got:\n{stdout}"
    );

    // @step And spec/foundation.json solutionSpace.capabilities has length 1
    let data = read_foundation(ws.path());
    assert_eq!(caps(&data).len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the no-capabilities detail line and exits 1 when none exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_no_capabilities_detail_and_exits_1() {
    // @step Given spec/foundation.json exists with an empty solutionSpace.capabilities array
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_caps(serde_json::json!([])));

    // @step When I run `./codelet/target/release/fspec remove-capability "X"`
    let (code, _stdout, stderr) = run_remove_cap(ws.path(), &["X"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Capability "X" not found'
    assert!(
        stderr.contains("Capability \"X\" not found"),
        "stderr must mention not-found; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'No capabilities exist in foundation'
    assert!(
        stderr.contains("No capabilities exist in foundation"),
        "stderr must mention detail line; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI lists available capabilities and exits 1 when the name is not found
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_lists_available_capabilities_and_exits_1() {
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_caps(serde_json::json!([
            {"name": "Reporting", "description": "r"},
            {"name": "Search", "description": "s"}
        ])),
    );

    // @step When I run `./codelet/target/release/fspec remove-capability "Login"`
    let (code, _stdout, stderr) = run_remove_cap(ws.path(), &["Login"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Capability "Login" not found'
    assert!(
        stderr.contains("Capability \"Login\" not found"),
        "stderr must mention not-found; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Available capabilities: Reporting, Search'
    assert!(
        stderr.contains("Available capabilities: Reporting, Search"),
        "stderr must list available; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when foundation.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_foundation_missing() {
    // @step Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec remove-capability "X"`
    let (code, _stdout, stderr) = run_remove_cap(ws.path(), &["X"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'foundation.json not found'
    assert!(
        stderr.contains("foundation.json not found"),
        "stderr must mention missing foundation; got:\n{stderr}"
    );

    // @step And no spec/foundation.json file is created
    assert!(!ws.path().join("spec/foundation.json").exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'A'},{name:'B'}]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_caps(serde_json::json!([
            {"name": "A", "description": "a"},
            {"name": "B", "description": "b"}
        ])),
    );

    // @step When I dispatch remove-capability via fspec_core::dispatch::dispatch_command with name='A'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-capability".to_string(),
        args_json: r#"{"name":"A"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./codelet/target/release/fspec remove-capability "B"` afterwards exits 0
    let (code, stdout, stderr) = run_remove_cap(ws.path(), &["B"]);
    assert_eq!(code, 0, "CLI remove must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/foundation.json solutionSpace.capabilities is empty
    let data = read_foundation(ws.path());
    assert_eq!(caps(&data).len(), 0, "expected empty, got {data}");

    // @step And the CLI bridge module codelet/fspec/src/remove_capability.rs contains NO inline draft probing, matching, or JSON-mutation logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_capability.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_capability.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "foundation.json.draft",
        "write_json_atomic",
        "ensure_foundation_file",
        "solutionSpace",
        "findIndex",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: remove-capability --help is byte-for-byte identical to the captured fixture
// ─────────────────────────────────────────────────────────────────────────

const HELP_FIXTURE: &str = include_str!("fixtures/help/remove-capability.txt");

#[test]
fn scenario_remove_capability_help_matches_fixture() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-capability --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("remove-capability")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-capability --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "remove-capability --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-capability.txt
    assert_eq!(stdout, HELP_FIXTURE);
}
