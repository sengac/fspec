//! CLI surface for the `compare-implementations` subcommand on the
//! standalone fspec Rust binary — RPC-207.
//!
//! Feature: spec/features/compare-implementations-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the CLI subcommand / core impl are still stubs, so the
//! behavioural scenarios are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_compare(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("compare-implementations");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec compare-implementations");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, units: &[(&str, &[&str])]) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut entries = String::new();
    for (i, (id, tags)) in units.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        let tags_json = serde_json::to_string(tags).unwrap();
        entries.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"{id} title","type":"story","status":"backlog","tags":{tags_json}}}"#
        ));
    }
    let json = format!(r#"{{"workUnits":{{{entries}}}}}"#);
    fs::write(spec.join("work-units.json"), json).expect("write work-units.json");
}

fn write_coverage(project_root: &Path, rel_name: &str, test_file: &str, impl_file: &str) {
    let dir = project_root.join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    let body = json!({
        "scenarios": [{
            "name": "S1",
            "testMappings": [{
                "file": test_file,
                "lines": "1-10",
                "implMappings": [{ "file": impl_file, "lines": [1, 2] }]
            }]
        }]
    });
    fs::write(dir.join(rel_name), body.to_string()).expect("write coverage file");
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/compare-implementations.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI default output prints the green summary line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_default_output_prints_the_green_summary_line() {
    // @step Given a temp workspace contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &[("CLI-001", &["@cli"])]);

    // @step When I run `./codelet/target/release/fspec compare-implementations --tag @cli` from that workspace
    let (code, stdout, stderr) = run_compare(ws.path(), &["--tag", "@cli"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Compared 1 work units tagged with @cli'
    assert!(
        stdout.contains("✓ Compared 1 work units tagged with @cli"),
        "stdout must show summary; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --json prints 2-space JSON envelope to stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_json_prints_2_space_json_envelope_to_stdout() {
    // @step Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &[("CLI-001", &["@cli"]), ("CLI-002", &["@cli"])]);

    // @step When I run `./codelet/target/release/fspec compare-implementations --tag @cli --json` from that workspace
    let (code, stdout, stderr) = run_compare(ws.path(), &["--tag", "@cli", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout parses as JSON with workUnits, comparison, namingConventionDifferences, and coverage fields
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert!(parsed.get("workUnits").is_some(), "missing workUnits");
    assert!(parsed.get("comparison").is_some(), "missing comparison");
    assert!(
        parsed.get("namingConventionDifferences").is_some(),
        "missing namingConventionDifferences"
    );
    assert!(parsed.get("coverage").is_some(), "missing coverage");

    // @step And the JSON.workUnits array has 2 elements
    assert_eq!(parsed["workUnits"].as_array().map(|a| a.len()), Some(2));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --show-coverage includes deduplicated coverage file paths
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_show_coverage_includes_deduplicated_coverage_file_paths() {
    // @step Given a temp workspace contains spec/work-units.json with one work unit tagged @cli and one .feature.coverage file referencing one test file and one impl file
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &[("CLI-001", &["@cli"])]);
    write_coverage(ws.path(), "a.feature.coverage", "test/a.test.ts", "src/a.ts");

    // @step When I run `./codelet/target/release/fspec compare-implementations --tag @cli --show-coverage --json` from that workspace
    let (code, stdout, stderr) =
        run_compare(ws.path(), &["--tag", "@cli", "--show-coverage", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    // @step And the JSON.coverage array has one entry
    assert_eq!(parsed["coverage"].as_array().map(|a| a.len()), Some(1));

    // @step And the JSON coverage[0].testFiles array has one element
    assert_eq!(
        parsed["coverage"][0]["testFiles"].as_array().map(|a| a.len()),
        Some(1)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when work-units.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_work_units_json_is_missing() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec compare-implementations --tag @cli` from that directory
    let (code, _stdout, stderr) = run_compare(ws.path(), &["--tag", "@cli"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "expected non-zero exit; stderr={stderr}");

    // @step And stderr contains the substring '✗ Comparison failed:'
    assert!(
        stderr.contains("✗ Comparison failed:"),
        "stderr must show failure prefix; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing two work units tagged @cli
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    for ws in [ws_cli.path(), ws_disp.path()] {
        write_work_units(ws, &[("CLI-001", &["@cli"]), ("CLI-002", &["@cli"])]);
    }

    // @step When I run compare-implementations once via the dispatcher and once via the CLI --json on identical inputs
    let req = codelet_fspec_core::DispatchRequest {
        command: "compare-implementations".to_string(),
        args_json: r#"{"tag":"@cli"}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let disp_result = codelet_fspec_core::dispatch_command(req);
    let (_code, cli_stdout, _stderr) = run_compare(ws_cli.path(), &["--tag", "@cli", "--json"]);

    // @step Then both front doors produce the same JSON envelope
    let disp_data: Value =
        serde_json::from_str(&disp_result.data).expect("dispatcher data must be JSON");
    let cli_data: Value =
        serde_json::from_str(cli_stdout.trim()).expect("CLI stdout must be JSON");
    assert_eq!(
        disp_data, cli_data,
        "dispatcher and CLI envelopes must match;\ndisp={disp_data}\ncli={cli_data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: compare-implementations --help is byte-for-byte identical to the
//           TS formatCommandHelp reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_compare_implementations_help_is_byte_for_byte_identical() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec compare-implementations --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("compare-implementations")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn compare-implementations --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "compare-implementations --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/compare-implementations.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
