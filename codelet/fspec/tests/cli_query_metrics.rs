//! CLI surface for the `query-metrics` subcommand on the standalone fspec
//! Rust binary — RPC-261.
//!
//! Feature: spec/features/query-metrics-cli-subcommand.feature
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

fn run_query_metrics(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-metrics");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-metrics");
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

/// ISO-8601 stamp `2026-01-01T00:00:00.000Z` + `hours` hours.
fn iso_hour(hours: u32) -> String {
    let h = hours % 24;
    let extra_days = hours / 24;
    let day = 1 + extra_days;
    format!("2026-01-{day:02}T{h:02}:00:00.000Z")
}

const TS_HELP_FIXTURE_QM: &str = include_str!("fixtures/help/query-metrics.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exposes query-metrics as a subcommand with flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exposes_query_metrics_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec query-metrics --help` with NO_COLOR=1
    let output = Command::new(fspec_bin())
        .arg("query-metrics")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec query-metrics --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-metrics --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the captured fixture at codelet/fspec/tests/fixtures/help/query-metrics.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QM);

    // @step Then stdout starts with a blank line followed by 'QUERY-METRICS'
    assert!(
        stdout.starts_with("\nQUERY-METRICS\n"),
        "stdout must start '\\nQUERY-METRICS\\n'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against missing work-units.json exits 1 with stderr Query failed prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_missing_work_units_exits_1_with_query_failed() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-metrics` from that directory
    let (code, stdout, stderr) = run_query_metrics(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec query-metrics must exit 1 on missing work-units.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Query failed'
    assert!(
        stderr.contains("Query failed"),
        "stderr must contain 'Query failed'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to query metrics:'
    assert!(
        stderr.contains("Failed to query metrics:"),
        "stderr must contain 'Failed to query metrics:'; got:\n{stderr}"
    );

    // @step Then spec/work-units.json was NOT created
    assert!(
        !ws.path().join("spec/work-units.json").exists(),
        "query-metrics must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI JSON output matches dispatcher output for the same on-disk state
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_json_output_matches_dispatcher_output() {
    // @step Given spec/work-units.json contains AUTH-001 with stateHistory at hour 0 (backlog) and hour 5 (done)
    let ws = tempfile::tempdir().expect("tempdir");
    let t0 = iso_hour(0);
    let t5 = iso_hour(5);
    let raw = format!(
        r#"{{
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Login",
      "type": "story",
      "status": "done",
      "createdAt": "{t0}",
      "updatedAt": "{t5}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{t0}" }},
        {{ "state": "done",    "timestamp": "{t5}" }}
      ]
    }}
  }},
  "states": {{
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./codelet/target/release/fspec query-metrics --work-unit-id AUTH-001 --format json` against that workspace
    let (code, stdout, stderr) =
        run_query_metrics(ws.path(), &["--work-unit-id", "AUTH-001", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-metrics must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout parses as JSON with cycleTime='5 hours'
    let trimmed = stdout.trim_end_matches('\n');
    let cli_data: serde_json::Value =
        serde_json::from_str(trimmed).expect("CLI stdout must be JSON");
    assert_eq!(cli_data["cycleTime"].as_str(), Some("5 hours"));

    // @step Then stdout equals the DispatchResult.data produced by dispatch_command for the same on-disk state followed by a trailing newline
    let req = codelet_fspec_core::DispatchRequest {
        command: "query-metrics".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    assert_eq!(
        stdout,
        format!("{}\n", result.data),
        "CLI stdout must equal dispatcher data + trailing newline"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output for aggregate path renders a Project Metrics block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_for_aggregate_renders_project_metrics_block() {
    // @step Given spec/work-units.json contains AUTH-001 (story, done with stateHistory 0→2h), AUTH-002 (story, backlog), BUG-001 (bug, done with stateHistory 0→1h)
    let ws = tempfile::tempdir().expect("tempdir");
    let t0 = iso_hour(0);
    let t1 = iso_hour(1);
    let t2 = iso_hour(2);
    let raw = format!(
        r#"{{
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001", "title": "t", "type": "story", "status": "done",
      "createdAt": "{t0}", "updatedAt": "{t2}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{t0}" }},
        {{ "state": "done",    "timestamp": "{t2}" }}
      ]
    }},
    "AUTH-002": {{
      "id": "AUTH-002", "title": "t", "type": "story", "status": "backlog",
      "createdAt": "{t0}", "updatedAt": "{t0}"
    }},
    "BUG-001": {{
      "id": "BUG-001", "title": "t", "type": "bug", "status": "done",
      "createdAt": "{t0}", "updatedAt": "{t1}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{t0}" }},
        {{ "state": "done",    "timestamp": "{t1}" }}
      ]
    }}
  }},
  "states": {{
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001","BUG-001"], "blocked": []
  }}
}}"#
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./codelet/target/release/fspec query-metrics`
    let (code, stdout, stderr) = run_query_metrics(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-metrics must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the exact line 'Total Work Units: 3'
    assert!(
        stdout.lines().any(|l| l == "Total Work Units: 3"),
        "want line 'Total Work Units: 3'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line 'Completed Work Units: 2'
    assert!(
        stdout.lines().any(|l| l == "Completed Work Units: 2"),
        "want line 'Completed Work Units: 2'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'By Type:'
    assert!(stdout.contains("By Type:"), "missing 'By Type:'; got:\n{stdout}");

    // @step Then stdout contains the exact line '  story: 2 work units'
    assert!(
        stdout.lines().any(|l| l == "  story: 2 work units"),
        "want line '  story: 2 work units'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  bug: 1 work unit'
    assert!(
        stdout.lines().any(|l| l == "  bug: 1 work unit"),
        "want line '  bug: 1 work unit'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI bridge module delegates to fspec_core with no inline aggregation logic
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_bridge_module_delegates_with_no_inline_aggregation() {
    // @step Given the file codelet/fspec/src/query_metrics.rs exists as the CLI bridge module
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_metrics.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/query_metrics.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );

    // @step When I read the source of codelet/fspec/src/query_metrics.rs
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then the source does NOT contain the substring 'Project Metrics'
    assert!(
        !bridge_src.contains("Project Metrics"),
        "bridge module must NOT embed 'Project Metrics' (would duplicate fspec_core logic); got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'aggregateMetrics'
    assert!(
        !bridge_src.contains("aggregateMetrics"),
        "bridge module must NOT embed 'aggregateMetrics'; got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'cycleTime'
    assert!(
        !bridge_src.contains("cycleTime"),
        "bridge module must NOT embed 'cycleTime'; got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'hour'
    assert!(
        !bridge_src.contains("hour"),
        "bridge module must NOT embed hours-formatting strings; got:\n{bridge_src}"
    );

    // @step Then the source calls codelet_fspec_core::commands::query_metrics::run
    assert!(
        bridge_src.contains("query_metrics::run") || bridge_src.contains("query_metrics :: run"),
        "bridge module must delegate to codelet_fspec_core::commands::query_metrics::run; got:\n{bridge_src}"
    );
}
