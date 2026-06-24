//! CLI surface for the `add-persona` subcommand on the standalone
//! fspec Rust binary — RPC-186.
//!
//! Feature: spec/features/add-persona-cli-subcommand.feature
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

fn run_add_persona(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-persona");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-persona");
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

fn foundation_one_real() -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "personas": [
            {"name": "Primary User", "description": "User description", "goals": ["User goal"]}
        ]
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-persona.txt");

#[test]
fn scenario_help_output_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-persona --help`
    let output = Command::new(fspec_bin())
        .arg("add-persona")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-persona --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "add-persona --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-persona.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'ADD-PERSONA'
    assert!(
        stdout.starts_with("\nADD-PERSONA"),
        "stdout must start with blank line + ADD-PERSONA; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI appends a persona and prints the multi-line success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_appends_persona_and_prints_success_block() {
    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_one_real());

    // @step When I run `fspec add-persona "QA Engineer" "Tests features" --goal "Catch regressions"` in that tempdir
    let (code, stdout, stderr) = run_add_persona(
        ws.path(),
        &[
            "QA Engineer",
            "Tests features",
            "--goal",
            "Catch regressions",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Added persona to foundation.json'
    assert!(
        stdout.contains("✓ Added persona to foundation.json"),
        "missing Added line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Name: QA Engineer'
    assert!(
        stdout.contains("  Name: QA Engineer"),
        "missing Name; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Description: Tests features'
    assert!(
        stdout.contains("  Description: Tests features"),
        "missing Description; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Goals: Catch regressions'
    assert!(
        stdout.contains("  Goals: Catch regressions"),
        "missing Goals; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows personas has length 2
    let f = read_json(ws.path(), "foundation.json");
    assert_eq!(f["personas"].as_array().expect("array").len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI joins multiple --goal flags with a comma and space
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_joins_multiple_goal_flags() {
    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_one_real());

    // @step When I run `fspec add-persona "Founder" "Runs the company" --goal "Ship fast" --goal "Stay safe"` in that tempdir
    let (code, stdout, stderr) = run_add_persona(
        ws.path(),
        &[
            "Founder",
            "Runs the company",
            "--goal",
            "Ship fast",
            "--goal",
            "Stay safe",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '  Goals: Ship fast, Stay safe'
    assert!(
        stdout.contains("  Goals: Ship fast, Stay safe"),
        "missing joined goals; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with no --goal flag prints an empty Goals line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_no_goal_flag_prints_empty_goals_line() {
    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_one_real());

    // @step When I run `fspec add-persona "Observer" "Just watches"` in that tempdir
    let (code, stdout, stderr) = run_add_persona(ws.path(), &["Observer", "Just watches"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '  Goals: '
    assert!(
        stdout.contains("  Goals: "),
        "missing Goals line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows the last persona has goals=[]
    let f = read_json(ws.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("array");
    let last = personas.last().expect("at least one");
    assert_eq!(last["goals"], serde_json::json!([]));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports placeholder removal before the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_placeholder_removal() {
    // @step Given a project root tempdir with spec/foundation.json whose only persona is named '[QUESTION: Who uses this?]'
    let ws = tempfile::tempdir().expect("tempdir");
    let mut f = foundation_one_real();
    f["personas"] = serde_json::json!([
        {"name": "[QUESTION: Who uses this?]", "description": "d", "goals": []}
    ]);
    write_file(ws.path(), "foundation.json", &f);

    // @step When I run `fspec add-persona "Developer" "Builds features" --goal "Ship quality code"` in that tempdir
    let (code, stdout, stderr) = run_add_persona(
        ws.path(),
        &[
            "Developer",
            "Builds features",
            "--goal",
            "Ship quality code",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring 'Removed 1 placeholder persona(s)'
    assert!(
        stdout.contains("Removed 1 placeholder persona(s)"),
        "missing removal line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '✓ Added persona to foundation.json'
    assert!(
        stdout.contains("✓ Added persona to foundation.json"),
        "missing Added line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes to the draft when foundation.json.draft exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_to_draft_when_present() {
    // @step Given a project root tempdir with both spec/foundation.json and spec/foundation.json.draft present
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_one_real());
    write_file(ws.path(), "foundation.json.draft", &foundation_one_real());

    // @step When I run `fspec add-persona "Drafted" "Lives in the draft"` in that tempdir
    let (code, stdout, stderr) = run_add_persona(ws.path(), &["Drafted", "Lives in the draft"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Added persona to foundation.json.draft'
    assert!(
        stdout.contains("✓ Added persona to foundation.json.draft"),
        "missing draft Added line; got:\n{stdout}"
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

    // @step When I run `fspec add-persona "Nobody" "No file"` in that tempdir
    let (code, _stdout, stderr) = run_add_persona(ws.path(), &["Nobody", "No file"]);

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
    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "foundation.json", &foundation_one_real());

    // @step When I dispatch add-persona via fspec_core::dispatch::dispatch_command with name='Core User' description='From dispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-persona".to_string(),
        args_json: r#"{"name":"Core User","description":"From dispatcher","goals":[]}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-persona "Cli User" "From cli"` afterwards exits 0
    let (code, stdout, stderr) = run_add_persona(ws.path(), &["Cli User", "From cli"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows personas has length 3
    let f = read_json(ws.path(), "foundation.json");
    assert_eq!(f["personas"].as_array().expect("array").len(), 3);

    // @step And the CLI bridge module codelet/fspec/src/add_persona.rs contains NO inline placeholder, file-read, or file-write logic — its only computation is JSON arg marshalling and stdout rendering
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_persona.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_persona.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "QUESTION",
        "DETECTED",
        "read_to_string",
        "write_json_atomic",
        "foundation.json.draft",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
