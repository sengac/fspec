//! CLI surface for the `update-foundation` subcommand on the standalone fspec
//! Rust binary — RPC-312.
//!
//! Feature: spec/features/update-foundation-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-foundation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn valid_foundation() -> serde_json::Value {
    // Must be schema-valid: solutionSpace.capabilities requires >= 1 item
    // (Ajv minItems:1), since the final-path update now runs the same
    // generic-foundation schema gate the TS command does (D2 parity).
    serde_json::json!({
        "version": "2.0.0",
        "project": {
            "name": "Original Name",
            "vision": "Original Vision",
            "projectType": "cli-tool"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "Primary Problem",
                "description": "Problem description",
                "impact": "high"
            }
        },
        "solutionSpace": {
            "overview": "Solution overview",
            "capabilities": [
                { "name": "Core Capability", "description": "Capability description" }
            ]
        }
    })
}

fn write_foundation(project_root: &Path, value: &serde_json::Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn write_draft(project_root: &Path, value: &serde_json::Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json.draft"),
        serde_json::to_string_pretty(value).expect("ser draft"),
    )
    .expect("write foundation.json.draft");
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str(&raw).expect("parse json")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/update-foundation.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_update_foundation_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec update-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("update-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "update-foundation --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/update-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI updates a final foundation field and prints the success lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_updates_final_field_and_prints_success_lines() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &valid_foundation());

    // @step When I run `fspec update-foundation projectName "Acme Tool"` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["projectName", "Acme Tool"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Updated "projectName" section in FOUNDATION.md'
    assert!(
        stdout.contains("✓ Updated \"projectName\" section in FOUNDATION.md"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Regenerated: spec/FOUNDATION.md'
    assert!(
        stdout.contains("Regenerated: spec/FOUNDATION.md"),
        "stdout must mention MD regeneration; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows project.name='Acme Tool'
    let v = read_json(&ws.path().join("spec/foundation.json"));
    assert_eq!(v["project"]["name"].as_str(), Some("Acme Tool"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI updates the draft when a foundation.json.draft is present
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_updates_draft_when_present() {
    // @step Given a project root tempdir with an existing spec/foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");
    write_draft(ws.path(), &valid_foundation());

    // @step When I run `fspec update-foundation projectVision "Ship faster"` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["projectVision", "Ship faster"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Updated "projectVision" in foundation.json.draft'
    assert!(
        stdout.contains("✓ Updated \"projectVision\" in foundation.json.draft"),
        "stdout must contain canonical draft success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json.draft on disk shows project.vision='Ship faster'
    let v = read_json(&ws.path().join("spec/foundation.json.draft"));
    assert_eq!(v["project"]["vision"].as_str(), Some("Ship faster"));
}

// ─────────────────────────────────────────────────────────────────────────
// D1 parity: draft updates chain to the next-field <system-reminder>
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_draft_emits_next_field_reminder() {
    // @step Given a project root tempdir with a draft whose project.vision is an unfilled placeholder
    let ws = tempfile::tempdir().expect("tempdir");
    write_draft(
        ws.path(),
        &serde_json::json!({
            "version": "2.0.0",
            "project": {
                "name": "[QUESTION: What is the project name?]",
                "vision": "[QUESTION: What is the one-sentence vision?]",
                "projectType": "cli-tool"
            },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        }),
    );

    // @step When I fill projectName (the first placeholder field)
    let (code, stdout, stderr) = run_cmd(ws.path(), &["projectName", "Acme"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the draft success + Updated lines
    assert!(stdout.contains("✓ Updated \"projectName\" in foundation.json.draft"));
    assert!(stdout.contains("  Updated: spec/foundation.json.draft"));

    // @step And stdout chains to the next field's <system-reminder> (project.vision)
    assert!(
        stdout.contains("<system-reminder>"),
        "draft update must chain a <system-reminder>; got:\n{stdout}"
    );
    assert!(
        stdout.contains("Field 2/8: project.vision (elevator pitch)"),
        "reminder must target the next unfilled field; got:\n{stdout}"
    );
    // Default agent (no FSPEC_AGENT, no config) → non-meta-cognition wording.
    assert!(stdout.contains("Think a lot about the entire codebase."));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown section with exit 1 and the TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_section_with_exit_1() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &valid_foundation());
    let pre = fs::read(ws.path().join("spec/foundation.json")).unwrap();

    // @step When I run `fspec update-foundation bogusSection "whatever"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["bogusSection", "whatever"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error: prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Unknown section: "bogusSection"'
    assert!(
        stderr.contains("Unknown section: \"bogusSection\""),
        "stderr must contain canonical unknown-section message; got:\n{stderr}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(ws.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre, post);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &valid_foundation());

    // @step When I dispatch update-foundation via fspec_core::dispatch::dispatch_command with section='projectName' content='Via Dispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "update-foundation".to_string(),
        args_json: r#"{"section":"projectName","content":"Via Dispatcher"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec update-foundation projectVision "Via CLI"` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &["projectVision", "Via CLI"]);
    assert_eq!(
        code, 0,
        "CLI update must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows project.name='Via Dispatcher' and project.vision='Via CLI'
    let v = read_json(&ws.path().join("spec/foundation.json"));
    assert_eq!(v["project"]["name"].as_str(), Some("Via Dispatcher"));
    assert_eq!(v["project"]["vision"].as_str(), Some("Via CLI"));

    // @step And the CLI bridge module rust/fspec/src/update_foundation.rs contains NO inline field mapping, validation, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update_foundation.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/update_foundation.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "primaryProblem",
        "solutionSpace",
        "write_json_atomic",
        "Unknown section",
        "cannot be empty",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
