//! CLI surface for the `query-estimate-accuracy` subcommand on the standalone
//! fspec Rust binary — RPC-258.
//!
//! Feature: spec/features/query-estimate-accuracy-cli-subcommand.feature
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

fn run_qea(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-estimate-accuracy");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-estimate-accuracy");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn raw_work_units(entries: &[(&str, &str)]) -> String {
    let mut out = String::from("{\n  \"workUnits\": {");
    for (i, (id, body)) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\n    \"{id}\": {body}"));
    }
    out.push_str("\n  }\n}\n");
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes query-estimate-accuracy as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_query_estimate_accuracy_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-estimate-accuracy")
        .arg("--help")
        .output()
        .expect("spawn fspec query-estimate-accuracy --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-estimate-accuracy --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'query-estimate-accuracy' or 'QUERY-ESTIMATE-ACCURACY'
    assert!(
        stdout.contains("query-estimate-accuracy") || stdout.contains("QUERY-ESTIMATE-ACCURACY"),
        "help must mention the query-estimate-accuracy subcommand; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--work-unit-id'
    assert!(
        !stdout.contains("--work-unit-id"),
        "query-estimate-accuracy --help must NOT advertise --work-unit-id; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--by-prefix'
    assert!(
        !stdout.contains("--by-prefix"),
        "query-estimate-accuracy --help must NOT advertise --by-prefix; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "query-estimate-accuracy --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against an empty workspace prints the empty-report header and does not auto-create files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_workspace_prints_empty_report_header() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy` from that directory
    let (code, stdout, stderr) = run_qea(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-estimate-accuracy must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '📊 Estimation Accuracy Report'
    assert!(
        stdout.contains("📊 Estimation Accuracy Report"),
        "stdout must contain the report header; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'No completed work units with estimates and actuals found.'
    assert!(
        stdout.contains("No completed work units with estimates and actuals found."),
        "stdout must contain the empty-report sentinel; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'estimate field (story points)'
    assert!(
        stdout.contains("estimate field (story points)"),
        "stdout must include the guidance bullet text; got:\n{stdout}"
    );

    // @step And spec/work-units.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/work-units.json").exists(),
        "query-estimate-accuracy must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders the populated By Story Points section
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_populated_by_story_points_section() {
    // @step Given spec/work-units.json contains the done work units AUTH-001 (estimate=1 iterations=1), AUTH-002 (estimate=1 iterations=2), AUTH-003 (estimate=3 iterations=2), AUTH-004 (estimate=5 iterations=2)
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &raw_work_units(&[
            (
                "AUTH-001",
                r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":1,"iterations":1}"#,
            ),
            (
                "AUTH-002",
                r#"{"id":"AUTH-002","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":1,"iterations":2}"#,
            ),
            (
                "AUTH-003",
                r#"{"id":"AUTH-003","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"iterations":2}"#,
            ),
            (
                "AUTH-004",
                r#"{"id":"AUTH-004","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
            ),
        ]),
    );

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy`
    let (code, stdout, stderr) = run_qea(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-estimate-accuracy must exit 0 on populated workspace; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '📊 Estimation Accuracy Report'
    assert!(
        stdout.contains("📊 Estimation Accuracy Report"),
        "stdout must contain the report header; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'By Story Points:'
    assert!(
        stdout.contains("By Story Points:"),
        "stdout must contain 'By Story Points:' section; got:\n{stdout}"
    );

    // @step And stdout contains the substring '1 points:'
    assert!(
        stdout.contains("1 points:"),
        "stdout must contain '1 points:' bucket; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Average iterations: 1.5'
    assert!(
        stdout.contains("Average iterations: 1.5"),
        "stdout must contain 'Average iterations: 1.5'; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Samples: 2'
    assert!(
        stdout.contains("Samples: 2"),
        "stdout must contain 'Samples: 2'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --format=json prints pretty-printed JSON to stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_format_json_prints_pretty_printed_json() {
    // @step Given spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &raw_work_units(&[(
            "AUTH-001",
            r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
        )]),
    );

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy --format json`
    let (code, stdout, stderr) = run_qea(ws.path(), &["--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-estimate-accuracy --format json must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout parses as JSON whose root object has a 'byStoryPoints' field
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}\n{stdout}"));
    assert!(
        parsed["byStoryPoints"].is_object(),
        "root payload must have byStoryPoints field; got:\n{stdout}"
    );

    // @step And the byStoryPoints entry for '5' has avgIterations=2 and samples=1
    assert_eq!(
        parsed["byStoryPoints"]["5"]["avgIterations"].as_f64(),
        Some(2.0)
    );
    assert_eq!(parsed["byStoryPoints"]["5"]["samples"].as_u64(), Some(1));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not valid json");

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy`
    let (code, stdout, stderr) = run_qea(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec query-estimate-accuracy must exit 1 on malformed work-units.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Failed to query estimate accuracy:'
    assert!(
        stderr.contains("Failed to query estimate accuracy:"),
        "stderr must contain the wrapper prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain the inner parse-error substring; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_qea() {
    // @step Given the fspec Rust binary has query-estimate-accuracy registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step And the help output lists query-estimate-accuracy as an available subcommand
    assert!(
        help.contains("query-estimate-accuracy"),
        "fspec --help must list `query-estimate-accuracy` subcommand; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &raw_work_units(&[(
            "AUTH-001",
            r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "query-estimate-accuracy".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher payload byStoryPoints entry for '5' has samples=1
    assert_eq!(
        dispatcher_data["byStoryPoints"]["5"]["samples"].as_u64(),
        Some(1)
    );

    // @step And the CLI text output of `fspec query-estimate-accuracy` against the same on-disk state contains the line 'Samples: 1'
    let (code, stdout, _stderr) = run_qea(ws.path(), &[]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Samples: 1"),
        "CLI text output must reflect the same samples=1 figure as the dispatcher; got:\n{stdout}"
    );

    // @step And the CLI bridge module rust/fspec/src/query_estimate_accuracy.rs contains NO inline aggregation, prefix-grouping, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_estimate_accuracy.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/query_estimate_accuracy.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "avgIterations",
        "byStoryPoints",
        "byPrefix",
        "Estimation Accuracy Report",
        "By Story Points",
        "Average iterations",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: query-estimate-accuracy --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_QEA: &str = include_str!("fixtures/help/query-estimate-accuracy.txt");

#[test]
fn scenario_query_estimate_accuracy_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-estimate-accuracy --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-estimate-accuracy")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-estimate-accuracy --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "query-estimate-accuracy --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-estimate-accuracy.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QEA);

    // @step And stdout starts with a blank line followed by 'QUERY-ESTIMATE-ACCURACY'
    assert!(stdout.starts_with("\nQUERY-ESTIMATE-ACCURACY\n"));
}
