//! CLI surface for the `generate-scenarios` subcommand on the standalone
//! fspec Rust binary — RPC-234.
//!
//! Feature: spec/features/generate-scenarios-cli-subcommand.feature
//!
//! RED PHASE: these tests exercise the (to-be-wired) clap subcommand
//! `Mode::GenerateScenarios` in `rust/fspec/src/main.rs` and the ported
//! `rust/fspec-core/src/commands/generate_scenarios.rs`. Each scenario maps
//! 1:1 to a Gherkin scenario; @step comments mirror the step text verbatim.
//! Until the shared wiring + port land, these fail (unknown command /
//! NotYetPorted / help-fixture mismatch) which is the correct red signal.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn run_gs(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("generate-scenarios");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec generate-scenarios");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, data: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(data).expect("serialize work-units"),
    )
    .expect("write work-units.json");
}

fn unit_data(id: &str, title: &str, status: &str) -> Value {
    let wu = json!({
        "id": id,
        "title": title,
        "type": "story",
        "status": status,
        "createdAt": "2026-06-01T00:00:00.000Z",
        "updatedAt": "2026-06-01T00:00:00.000Z",
    });
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), Value::Array(arr));
    }
    json!({
        "version": "0.7.1",
        "workUnits": { id: wu },
        "states": Value::Object(states),
    })
}

fn empty_work_units() -> Value {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert((*st).to_string(), Value::Array(vec![]));
    }
    json!({
        "version": "0.7.1",
        "workUnits": {},
        "states": Value::Object(states),
    })
}

fn ready_unit(id: &str, title: &str) -> Value {
    let mut data = unit_data(id, title, "specifying");
    data["workUnits"][id]["rules"] =
        json!([{ "id": 0, "text": "Password must be 8+ characters", "deleted": false }]);
    data["workUnits"][id]["examples"] =
        json!([{ "id": 0, "text": "User views the account settings page", "deleted": false }]);
    data["workUnits"][id]["userStory"] = json!({ "role": "registered user", "action": "log in securely", "benefit": "access my account" });
    data
}

fn write_existing_feature(project_root: &Path, file: &str, scenario_name: &str) {
    let dir = project_root.join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir spec/features");
    let content = format!(
        "@EXIST-001\nFeature: Existing Capability\n\n  Scenario: {scenario_name}\n    Given I am a registered user\n    When I log in with valid credentials\n    Then I should see the dashboard\n"
    );
    fs::write(dir.join(file), content).expect("write existing feature");
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/generate-scenarios.txt");

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_generate_scenarios_with_byte_parity_help() {
    // Scenario: Clap exposes generate-scenarios as a subcommand and prints byte-parity help

    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec generate-scenarios --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("generate-scenarios")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec generate-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "generate-scenarios --help must exit 0; stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/generate-scenarios.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout starts with a blank line followed by "GENERATE-SCENARIOS"
    assert!(
        stdout.starts_with("\nGENERATE-SCENARIOS\n"),
        "help must start with blank line + GENERATE-SCENARIOS; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_creates_a_context_only_feature_file_and_exits_0() {
    // Scenario: CLI creates a context-only feature file and exits 0

    // @step Given a temp working directory whose work unit RPC-001 is ready with a user story, a rule and an example
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("RPC-001", "User Authentication"));

    // @step When I run `fspec generate-scenarios RPC-001 --feature=user-auth` from that directory
    let (code, stdout, stderr) = run_gs(tmp.path(), &["RPC-001", "--feature=user-auth"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "generate-scenarios must exit 0; stderr={stderr}");

    // @step Then stdout contains the substring "✓ Created context-only feature file:"
    assert!(
        stdout.contains("✓ Created context-only feature file:"),
        "stdout must contain the creation line; got:\n{stdout}"
    );

    // @step Then stdout contains the substring "Contains example mapping context as comments (NO scenarios yet)"
    assert!(
        stdout.contains("Contains example mapping context as comments (NO scenarios yet)"),
        "stdout must contain the context note; got:\n{stdout}"
    );

    // @step Then the file spec/features/user-auth.feature exists on disk
    assert!(
        tmp.path().join("spec/features/user-auth.feature").exists(),
        "spec/features/user-auth.feature must exist"
    );
}

#[test]
fn scenario_cli_prints_failure_to_stderr_and_exits_1_for_a_missing_work_unit() {
    // Scenario: CLI prints failure to stderr and exits 1 for a missing work unit

    // @step Given a temp working directory with an empty work-units store
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &empty_work_units());

    // @step When I run `fspec generate-scenarios MISSING-001` from that directory
    let (code, _stdout, stderr) = run_gs(tmp.path(), &["MISSING-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "missing work unit must exit 1; stderr={stderr}");

    // @step Then stderr contains the substring "✗ Failed to generate scenarios:"
    assert!(
        stderr.contains("✗ Failed to generate scenarios:"),
        "stderr must contain the failure line; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_accepts_the_ignore_possible_duplicates_flag() {
    // Scenario: CLI accepts the ignore-possible-duplicates flag

    // @step Given a temp working directory whose work unit RPC-002 has an example matching an existing scenario above threshold
    let tmp = TempDir::new().expect("tempdir");
    let mut data = ready_unit("RPC-002", "Widget Catalog");
    data["workUnits"]["RPC-002"]["examples"] = json!([
        { "id": 0, "text": "User logs in with valid credentials and sees the dashboard", "deleted": false }
    ]);
    write_work_units(tmp.path(), &data);
    write_existing_feature(
        tmp.path(),
        "existing.feature",
        "User logs in with valid credentials and sees the dashboard",
    );

    // @step When I run `fspec generate-scenarios RPC-002 --feature=widgets --ignore-possible-duplicates` from that directory
    let (code, stdout, stderr) = run_gs(
        tmp.path(),
        &[
            "RPC-002",
            "--feature=widgets",
            "--ignore-possible-duplicates",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "ignore-possible-duplicates run must exit 0; stderr={stderr}; stdout={stdout}"
    );

    // @step Then the file spec/features/widgets.feature exists on disk
    assert!(
        tmp.path().join("spec/features/widgets.feature").exists(),
        "spec/features/widgets.feature must exist"
    );
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_generate_scenarios() {
    // Scenario: Default combined TUI mode is preserved when no subcommand is provided

    // @step Given the fspec Rust binary has generate-scenarios registered as a clap subcommand alongside the existing subcommands
    // (asserted by the help-listing check below)

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");

    // @step Then the command exits 0
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step Then the help output lists generate-scenarios as an available subcommand
    let help = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        help.contains("generate-scenarios"),
        "fspec --help must list the `generate-scenarios` subcommand; got:\n{help}"
    );
}
