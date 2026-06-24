//! CLI surface for the `add-capability` subcommand on the standalone
//! fspec Rust binary — RPC-173.
//!
//! Feature: spec/features/add-capability-cli-subcommand.feature
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

fn run_add_cap(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-capability");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-capability");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation_named(cwd: &Path, name: &str, value: &serde_json::Value) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join(name),
        serde_json::to_string_pretty(value).expect("ser"),
    )
    .expect("write foundation file");
}

fn write_foundation(cwd: &Path, value: &serde_json::Value) {
    write_foundation_named(cwd, "foundation.json", value);
}

fn read_foundation_named(cwd: &Path, name: &str) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec").join(name)).expect("read foundation file");
    serde_json::from_str(&raw).expect("parse foundation file")
}

fn read_foundation(cwd: &Path) -> serde_json::Value {
    read_foundation_named(cwd, "foundation.json")
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
// Scenario: Clap exposes add-capability with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_capability_with_positional_args_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-capability --help`
    let output = Command::new(fspec_bin())
        .arg("add-capability")
        .arg("--help")
        .output()
        .expect("spawn add-capability --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-capability --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the add-capability subcommand
    assert!(
        stdout.contains("add-capability") || stdout.contains("ADD-CAPABILITY"),
        "help must describe add-capability; got:\n{stdout}"
    );

    // @step And stdout mentions the `<name>` argument
    assert!(
        stdout.contains("name"),
        "help must mention name; got:\n{stdout}"
    );

    // @step And stdout mentions the `<description>` argument
    assert!(
        stdout.contains("description"),
        "help must mention description; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a capability and prints the success block on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_capability_and_prints_success_block() {
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_caps(serde_json::json!([])));

    // @step When I run `./codelet/target/release/fspec add-capability "Search" "Full text search"`
    let (code, stdout, stderr) = run_add_cap(ws.path(), &["Search", "Full text search"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Added capability to foundation.json'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Added capability to foundation.json")),
        "missing Added line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Name: Search'
    assert!(
        stdout.contains("  Name: Search"),
        "missing Name line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Description: Full text search'
    assert!(
        stdout.contains("  Description: Full text search"),
        "missing Description line; got:\n{stdout}"
    );

    // @step And spec/foundation.json solutionSpace.capabilities contains exactly one entry named 'Search'
    let data = read_foundation(ws.path());
    assert_eq!(caps(&data).len(), 1);
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("Search"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes to the draft and reports the draft file name
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_to_draft_and_reports_draft_file_name() {
    // @step Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'}]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation_named(
        ws.path(),
        "foundation.json.draft",
        &foundation_with_caps(serde_json::json!([{"name": "Reporting", "description": "r"}])),
    );

    // @step When I run `./codelet/target/release/fspec add-capability "Data Export" "Export to CSV"`
    let (code, stdout, stderr) = run_add_cap(ws.path(), &["Data Export", "Export to CSV"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Added capability to foundation.json.draft'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Added capability to foundation.json.draft")),
        "missing draft Added line; got:\n{stdout}"
    );

    // @step And spec/foundation.json.draft solutionSpace.capabilities has length 2
    let draft = read_foundation_named(ws.path(), "foundation.json.draft");
    assert_eq!(caps(&draft).len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the placeholder-removal line when only placeholders existed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_placeholder_removal_line() {
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'[QUESTION: What can users do?]', description:'[DETECTED: ...]'}]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_caps(serde_json::json!([
            {"name": "[QUESTION: What can users do?]", "description": "[DETECTED: ...]"}
        ])),
    );

    // @step When I run `./codelet/target/release/fspec add-capability "Login" "Authenticate users"`
    let (code, stdout, stderr) = run_add_cap(ws.path(), &["Login", "Authenticate users"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring 'Removed 1 placeholder capability(ies)'
    assert!(
        stdout.contains("Removed 1 placeholder capability(ies)"),
        "missing placeholder removal line; got:\n{stdout}"
    );

    // @step And stdout contains the line '✓ Added capability to foundation.json'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Added capability to foundation.json")),
        "missing Added line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when foundation.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_foundation_missing() {
    // @step Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec add-capability "X" "Y"`
    let (code, _stdout, stderr) = run_add_cap(ws.path(), &["X", "Y"]);

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
    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_caps(serde_json::json!([])));

    // @step When I dispatch add-capability via fspec_core::dispatch::dispatch_command with name='Via dispatcher' description='d'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-capability".to_string(),
        args_json: r#"{"name":"Via dispatcher","description":"d"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./codelet/target/release/fspec add-capability "Via CLI" "d"` afterwards exits 0
    let (code, stdout, stderr) = run_add_cap(ws.path(), &["Via CLI", "d"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json solutionSpace.capabilities contains two entries
    let data = read_foundation(ws.path());
    assert_eq!(caps(&data).len(), 2, "expected two entries, got {data}");

    // @step And the CLI bridge module codelet/fspec/src/add_capability.rs contains NO inline draft probing, placeholder detection, or JSON-mutation logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_capability.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_capability.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "foundation.json.draft",
        "QUESTION",
        "DETECTED",
        "write_json_atomic",
        "ensure_foundation_file",
        "solutionSpace",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-capability --help is byte-for-byte identical to the captured fixture
// ─────────────────────────────────────────────────────────────────────────

const HELP_FIXTURE: &str = include_str!("fixtures/help/add-capability.txt");

#[test]
fn scenario_add_capability_help_matches_fixture() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-capability --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-capability")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-capability --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-capability --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-capability.txt
    assert_eq!(stdout, HELP_FIXTURE);
}
