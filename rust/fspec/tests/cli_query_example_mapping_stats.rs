//! CLI surface for the `query-example-mapping-stats` subcommand on the
//! standalone fspec Rust binary — RPC-260.
//!
//! Feature: spec/features/query-example-mapping-stats-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect either a NotYetPorted error path or
//! a missing-subcommand failure. Once the subcommand is wired, the
//! green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_qems(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-example-mapping-stats");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec query-example-mapping-stats");
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

fn dispatch_qems(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "query-example-mapping-stats".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

/// Build minimal work-units.json containing a single WU with arrays
fn wu_with_arrays(
    id: &str,
    title: &str,
    status: &str,
    rules: usize,
    examples: usize,
    questions: usize,
    assumptions: usize,
) -> String {
    let make_arr = |n: usize, prefix: &str| -> String {
        let items: Vec<String> = (0..n).map(|i| format!(r#""{prefix}-{i}""#)).collect();
        format!("[{}]", items.join(","))
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "{title}", "status": "{status}",
      "createdAt": "x", "updatedAt": "x",
      "rules": {rules_json},
      "examples": {examples_json},
      "questions": {questions_json},
      "assumptions": {assumptions_json}
    }}
  }},
  "states": {{
    "backlog": ["{id}"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#,
        rules_json = make_arr(rules, "rule"),
        examples_json = make_arr(examples, "example"),
        questions_json = make_arr(questions, "question"),
        assumptions_json = make_arr(assumptions, "assumption"),
    )
}

// =========================================================================
// Scenarios from query-example-mapping-stats-cli-subcommand.feature
// =========================================================================

#[test]
fn scenario_clap_exposes_query_example_mapping_stats_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec query-example-mapping-stats --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-example-mapping-stats")
        .arg("--help")
        .output()
        .expect("spawn fspec query-example-mapping-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-example-mapping-stats --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'query-example-mapping-stats'
    assert!(
        stdout.contains("query-example-mapping-stats")
            || stdout.contains("QUERY-EXAMPLE-MAPPING-STATS"),
        "help must describe the subcommand; got:\n{stdout}"
    );

    // @step Then stdout advertises the '--format' flag the CLI accepts (CLI-016)
    assert!(
        stdout.contains("--format"),
        "help must advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workUnitId'
    assert!(
        !stdout.contains("--workUnitId"),
        "help must NOT expose --workUnitId; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--hasQuestions'
    assert!(
        !stdout.contains("--hasQuestions"),
        "help must NOT expose --hasQuestions; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--questionsFor'
    assert!(
        !stdout.contains("--questionsFor"),
        "help must NOT expose --questionsFor; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_format_json_prints_canonical_empty_stats() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-example-mapping-stats --format json` from that directory
    let (code, stdout, stderr) = run_qems(ws.path(), &["--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout parses as JSON containing the fields workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}\nstdout:\n{stdout}"));
    for field in [
        "workUnits",
        "workUnitsWithRules",
        "workUnitsWithExamples",
        "workUnitsWithQuestions",
        "workUnitsWithAssumptions",
        "avgRulesPerWorkUnit",
        "avgExamplesPerWorkUnit",
        "avgQuestionsPerWorkUnit",
        "avgAssumptionsPerWorkUnit",
    ] {
        assert!(
            parsed.get(field).is_some(),
            "missing field `{field}` in:\n{stdout}"
        );
    }

    // @step Then the parsed JSON workUnits is the empty array
    assert_eq!(parsed["workUnits"].as_array().map(Vec::len), Some(0));

    // @step Then the parsed JSON has workUnitsWithRules=0, workUnitsWithExamples=0, workUnitsWithQuestions=0, workUnitsWithAssumptions=0
    assert_eq!(parsed["workUnitsWithRules"].as_u64(), Some(0));
    assert_eq!(parsed["workUnitsWithExamples"].as_u64(), Some(0));
    assert_eq!(parsed["workUnitsWithQuestions"].as_u64(), Some(0));
    assert_eq!(parsed["workUnitsWithAssumptions"].as_u64(), Some(0));
}

#[test]
fn scenario_cli_without_format_prints_nothing_to_stdout() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec query-example-mapping-stats` from that directory
    let (code, stdout, stderr) = run_qems(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout is exactly empty
    assert_eq!(
        stdout, "",
        "stdout must be empty (TS silent-text parity); got:\n{stdout}"
    );

    // @step Then stderr is exactly empty
    assert_eq!(stderr, "", "stderr must be empty; got:\n{stderr}");
}

#[test]
fn scenario_cli_format_text_prints_nothing_to_stdout() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec query-example-mapping-stats --format text` from that directory
    let (code, stdout, stderr) = run_qems(ws.path(), &["--format", "text"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout is exactly empty
    assert_eq!(
        stdout, "",
        "stdout must be empty for --format text; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec query-example-mapping-stats --format json` from that directory
    let (code, stdout, stderr) = run_qems(ws.path(), &["--format", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain 'Failed to parse work-units.json'; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with 2 rules and 1 example
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "Login", "backlog", 2, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats through fspec_core::dispatch::dispatch_command with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the DispatchResult.data parses as JSON with workUnitsWithRules=1 and workUnitsWithExamples=1
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithExamples"].as_u64(), Some(1));

    // @step Then the CLI bridge module rust/fspec/src/query_example_mapping_stats.rs contains NO inline aggregation, filter, or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_example_mapping_stats.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "completenessScore",
        "workUnitsWithRules",
        "workUnitsWithExamples",
        "workUnitsWithQuestions",
        "workUnitsWithAssumptions",
        "avgRulesPerWorkUnit",
        "calculateCompletenessScore",
        "calculate_completeness_score",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_QEMS: &str = include_str!("fixtures/help/query-example-mapping-stats.txt");

#[test]
fn scenario_query_example_mapping_stats_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec query-example-mapping-stats --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-example-mapping-stats")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-example-mapping-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-example-mapping-stats.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QEMS);

    // @step Then stdout starts with a blank line followed by 'QUERY-EXAMPLE-MAPPING-STATS'
    assert!(stdout.starts_with("\nQUERY-EXAMPLE-MAPPING-STATS\n"));
}
