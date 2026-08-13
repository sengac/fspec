//! CLI surface for the `query-dependency-stats` subcommand on the standalone
//! fspec Rust binary — RPC-257.
//!
//! Feature: spec/features/query-dependency-stats-cli-subcommand.feature
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

fn run_query(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-dependency-stats");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-dependency-stats");
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes query-dependency-stats as a subcommand and prints
//           flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_query_dependency_stats_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec query-dependency-stats --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-dependency-stats")
        .arg("--help")
        .output()
        .expect("spawn fspec query-dependency-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-dependency-stats --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'query-dependency-stats'
    assert!(
        stdout.contains("query-dependency-stats") || stdout.contains("QUERY-DEPENDENCY-STATS"),
        "help must describe the query-dependency-stats subcommand; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with --format=json prints all ten canonical fields against
//           an empty workspace
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_format_json_prints_all_ten_canonical_fields_against_empty_workspace() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-dependency-stats --format json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-dependency-stats --format json must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout parses as JSON containing the fields totalBlocks, totalBlockedBy, totalDependsOn, totalRelatesTo, workUnitsWithDependencies, workUnitsWithBlockers, workUnitsBlockingOthers, workUnitsWithSoftDependencies, averageDependenciesPerUnit, maxDependencyChainDepth
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}\nstdout was:\n{stdout}"));
    for field in [
        "totalBlocks",
        "totalBlockedBy",
        "totalDependsOn",
        "totalRelatesTo",
        "workUnitsWithDependencies",
        "workUnitsWithBlockers",
        "workUnitsBlockingOthers",
        "workUnitsWithSoftDependencies",
        "averageDependenciesPerUnit",
        "maxDependencyChainDepth",
    ] {
        assert!(
            parsed.get(field).is_some(),
            "missing field `{field}` in:\n{stdout}"
        );
    }

    // @step Then every field except maxDependencyChainDepth is the JSON number 0
    for field in [
        "totalBlocks",
        "totalBlockedBy",
        "totalDependsOn",
        "totalRelatesTo",
        "workUnitsWithDependencies",
        "workUnitsWithBlockers",
        "workUnitsBlockingOthers",
        "workUnitsWithSoftDependencies",
        "averageDependenciesPerUnit",
    ] {
        assert_eq!(
            parsed[field].as_u64(),
            Some(0),
            "expected {field}=0 on empty workspace"
        );
    }

    // @step Then maxDependencyChainDepth is the JSON number 0
    assert_eq!(parsed["maxDependencyChainDepth"].as_u64(), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI without --format prints nothing to stdout (TS silent-text parity)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_without_format_prints_nothing_to_stdout() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-dependency-stats` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-dependency-stats must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout is exactly empty
    assert_eq!(
        stdout, "",
        "stdout must be exactly empty (TS silent-text parity); got:\n{stdout}"
    );

    // @step Then stderr is exactly empty
    assert_eq!(
        stderr, "",
        "stderr must be exactly empty (no warnings); got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with --format=text also prints nothing to stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_format_text_prints_nothing_to_stdout() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-dependency-stats --format text` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--format", "text"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-dependency-stats --format text must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is exactly empty
    assert_eq!(
        stdout, "",
        "stdout must be exactly empty for --format text (TS bug parity); got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec query-dependency-stats --format json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--format", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec query-dependency-stats must exit 1 on malformed work-units.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain 'Failed to parse work-units.json'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains A with blocks=['B'] and B with no dependency fields
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "A": { "id": "A", "title": "a", "status": "backlog", "blocks": ["B"], "createdAt": "x", "updatedAt": "x" },
    "B": { "id": "B", "title": "b", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["A", "B"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch query-dependency-stats through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "query-dependency-stats".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the DispatchResult.data parses as JSON with totalBlocks=1 and maxDependencyChainDepth=1
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(dispatcher_data["totalBlocks"].as_u64(), Some(1));
    assert_eq!(dispatcher_data["maxDependencyChainDepth"].as_u64(), Some(1));

    // @step Then the CLI bridge module rust/fspec/src/query_dependency_stats.rs contains NO inline aggregation, DFS, or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_dependency_stats.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/query_dependency_stats.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "totalBlocks",
        "totalBlockedBy",
        "totalDependsOn",
        "totalRelatesTo",
        "workUnitsBlockingOthers",
        "workUnitsWithBlockers",
        "workUnitsWithSoftDependencies",
        "workUnitsWithDependencies",
        "averageDependenciesPerUnit",
        "maxDependencyChainDepth",
        "calculateDepth",
        "calculate_depth",
        "visited",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: query-dependency-stats --help is byte-for-byte identical to TS
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_QDS: &str = include_str!("fixtures/help/query-dependency-stats.txt");

#[test]
fn scenario_query_dependency_stats_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-dependency-stats --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-dependency-stats")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-dependency-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "query-dependency-stats --help must exit 0; stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-dependency-stats.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QDS);

    // @step Then stdout starts with a blank line followed by 'QUERY-DEPENDENCY-STATS'
    assert!(stdout.starts_with("\nQUERY-DEPENDENCY-STATS\n"));
}
