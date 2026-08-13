//! CLI surface for the `query-example-mapping-stats` subcommand on the
//! standalone fspec Rust binary — RPC-260.
//!
//! Feature: spec/features/query-example-mapping-stats-cli-subcommand.feature
//! Feature: spec/features/query-example-mapping-stats-rust-port.feature
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

    // @step Then stdout advertises the '--status' flag (TS parity — TS help advertises --status even though the CLI accepts --format)
    assert!(
        stdout.contains("--status"),
        "help must advertise --status; got:\n{stdout}"
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

// =========================================================================
// Scenarios from query-example-mapping-stats-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_empty_stats_when_work_units_auto_created_in_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON has workUnits=[] (empty array)
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(0));

    // @step Then the returned JSON has workUnitsWithRules=0, workUnitsWithExamples=0, workUnitsWithQuestions=0, workUnitsWithAssumptions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithExamples"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithAssumptions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=0, avgExamplesPerWorkUnit=0, avgQuestionsPerWorkUnit=0, avgAssumptionsPerWorkUnit=0
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgExamplesPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgAssumptionsPerWorkUnit"].as_f64(), Some(0.0));

    // @step Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created"
    );
}

#[test]
fn scenario_completeness_score_100_when_rules_and_examples_nonempty_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 2 rules, 1 example, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "Login", "backlog", 2, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has workUnitId='AUTH-001'
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );

    // @step Then the returned JSON workUnits[0] has rules=2, examples=1, questions=0, assumptions=0
    assert_eq!(data["workUnits"][0]["rules"].as_u64(), Some(2));
    assert_eq!(data["workUnits"][0]["examples"].as_u64(), Some(1));
    assert_eq!(data["workUnits"][0]["questions"].as_u64(), Some(0));
    assert_eq!(data["workUnits"][0]["assumptions"].as_u64(), Some(0));

    // @step Then the returned JSON workUnits[0] has completenessScore=100
    assert_eq!(
        data["workUnits"][0]["completenessScore"].as_u64(),
        Some(100)
    );
}

#[test]
fn scenario_completeness_score_0_when_only_questions() {
    // @step Given spec/work-units.json contains AUTH-002 with 0 rules, 0 examples, 1 question, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-002", "X", "backlog", 0, 0, 1, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=0
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(0));
}

#[test]
fn scenario_completeness_score_66_with_rules_only_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 rule, 0 examples, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "X", "backlog", 1, 0, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=66
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(66));
}

#[test]
fn scenario_completeness_score_67_with_examples_only_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 0 rules, 1 example, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "X", "backlog", 0, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=67
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(67));
}

#[test]
fn scenario_aggregate_counts_and_averages_reflect_every_retained_work_unit() {
    // @step Given spec/work-units.json contains AUTH-001 (2 rules, 1 example, 0 questions, 0 assumptions) and AUTH-002 (0 rules, 0 examples, 1 question, 0 assumptions)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0","r1"],"examples":["e0"],"questions":[],"assumptions":[]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","rules":[],"examples":[],"questions":["q0"],"assumptions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON has workUnitsWithRules=1, workUnitsWithExamples=1, workUnitsWithQuestions=1, workUnitsWithAssumptions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithExamples"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithAssumptions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=1
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(1.0));

    // @step Then the returned JSON has avgExamplesPerWorkUnit=0.5
    assert_eq!(data["avgExamplesPerWorkUnit"].as_f64(), Some(0.5));

    // @step Then the returned JSON has avgQuestionsPerWorkUnit=0.5
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.5));
}

#[test]
fn scenario_work_unit_id_filter_narrows_to_single_work_unit() {
    // @step Given spec/work-units.json contains AUTH-001 (2 rules, 1 example), AUTH-002 (0 rules, 1 question), AUTH-003 (1 rule)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0","r1"],"examples":["e0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-003": {"id":"AUTH-003","title":"C","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0"]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002","AUTH-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with workUnitId='AUTH-001' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"workUnitId":"AUTH-001","format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );

    // @step Then the returned JSON has workUnitsWithRules=1 and workUnitsWithQuestions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=2 and avgQuestionsPerWorkUnit=0
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(2.0));
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.0));
}

#[test]
fn scenario_work_unit_id_filter_against_missing_id_surfaces_error() {
    // @step Given spec/work-units.json contains AUTH-001, AUTH-002, AUTH-003
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x"},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x"},
    "AUTH-003": {"id":"AUTH-003","title":"C","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["AUTH-001","AUTH-002","AUTH-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with workUnitId='NOPE-999' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"workUnitId":"NOPE-999","format":"json"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring "Work unit 'NOPE-999' does not exist"
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Work unit 'NOPE-999' does not exist"))
            .unwrap_or(false),
        "error must mention missing work unit; got {result:?}"
    );
}

#[test]
fn scenario_has_questions_true_keeps_only_units_with_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with hasQuestions=true and format='json'
    let result = dispatch_qems(ws.path(), r#"{"hasQuestions":true,"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );
}

#[test]
fn scenario_has_questions_false_keeps_only_units_with_zero_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with hasQuestions=false and format='json'
    let result = dispatch_qems(ws.path(), r#"{"hasQuestions":false,"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-002'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-002")
    );
}

#[test]
fn scenario_questions_for_alice_keeps_only_units_mentioning_alice() {
    // @step Given spec/work-units.json contains AUTH-001 with question '@alice should we cache?' and AUTH-002 with question '@bob review'
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["@alice should we cache?"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":["@bob review"]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with questionsFor='alice' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"questionsFor":"alice","format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );
}

#[test]
fn scenario_result_json_field_order_matches_ts_shape() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 rule and 1 example
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "A", "backlog", 1, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the returned JSON field declaration order is workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit
    let expected_order = [
        "workUnits",
        "workUnitsWithRules",
        "workUnitsWithExamples",
        "workUnitsWithQuestions",
        "workUnitsWithAssumptions",
        "avgRulesPerWorkUnit",
        "avgExamplesPerWorkUnit",
        "avgQuestionsPerWorkUnit",
        "avgAssumptionsPerWorkUnit",
    ];
    let mut last_pos: i64 = -1;
    for field in expected_order {
        let key_pattern = format!("\"{field}\"");
        let pos = result
            .data
            .find(&key_pattern)
            .unwrap_or_else(|| panic!("field {field} missing from data:\n{}", result.data))
            as i64;
        assert!(
            pos > last_pos,
            "field order broken at {field}: pos {pos} <= last {last_pos}\nin:\n{}",
            result.data
        );
        last_pos = pos;
    }
}

#[test]
fn scenario_escalates_malformed_work_units_json_dispatcher() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I dispatch query-example-mapping-stats against that project root
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse work-units.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_per_work_unit_stats_record_carries_title_and_status() {
    // @step Given spec/work-units.json contains AUTH-001 with title 'Login flow' and status 'implementing'
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"Login flow","status":"implementing","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":[],"specifying":[],"testing":[],"implementing":["AUTH-001"],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has title='Login flow' and status='implementing'
    assert_eq!(data["workUnits"][0]["title"].as_str(), Some("Login flow"));
    assert_eq!(
        data["workUnits"][0]["status"].as_str(),
        Some("implementing")
    );
}
