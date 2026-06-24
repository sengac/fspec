//! CLI surface for the `compact-work-unit` subcommand on the standalone fspec
//! Rust binary — RPC-206.
//!
//! Feature: spec/features/compact-work-unit-cli-subcommand.feature
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

fn run_compact_work_unit(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("compact-work-unit");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec compact-work-unit");
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

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw =
        fs::read_to_string(cwd.join("spec").join("work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse JSON")
}

/// rules array: `count` total, first `deleted_count` soft-deleted, ids 0..count.
fn rules_json(count: usize, deleted_count: usize) -> String {
    let mut out = Vec::new();
    for i in 0..count {
        let deleted = i < deleted_count;
        out.push(format!(
            r#"{{"id":{i},"text":"rule {i}","deleted":{deleted},"createdAt":"x"}}"#
        ));
    }
    format!("[{}]", out.join(","))
}

fn doc(status: &str, rules: &str, next_rule_id: usize) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001", "title": "Login", "status": "{status}",
      "rules": {rules}, "nextRuleId": {next_rule_id},
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes compact-work-unit with a positional arg and --force flag in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_compact_work_unit_with_arg_and_force_flag() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec compact-work-unit --help`
    let output = Command::new(fspec_bin())
        .arg("compact-work-unit")
        .arg("--help")
        .output()
        .expect("spawn compact-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "compact-work-unit --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the compact-work-unit subcommand
    assert!(
        stdout.contains("compact-work-unit") || stdout.contains("COMPACT-WORK-UNIT"),
        "help must describe compact-work-unit; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout advertises the `--force` flag
    assert!(
        stdout.contains("--force"),
        "help must advertise --force; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "compact-work-unit --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI compacts a done work unit and prints the removed-items summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_compacts_done_work_unit_and_prints_summary() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 2 deleted rules and 1 live rule
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("done", &rules_json(3, 2), 3));

    // @step When I run `./codelet/target/release/fspec compact-work-unit AUTH-001`
    let (code, stdout, stderr) = run_compact_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Compacted work unit AUTH-001'
    assert!(
        stdout
            .lines()
            .any(|l| l == "✓ Compacted work unit AUTH-001"),
        "missing compacted line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Rules: 2'
    assert!(
        stdout.contains("Rules: 2"),
        "missing removed-rules summary; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the no-op sentinel when there is nothing to remove
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_noop_sentinel_when_nothing_to_remove() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 2 live rules and no deleted items
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("done", &rules_json(2, 0), 2));

    // @step When I run `./codelet/target/release/fspec compact-work-unit AUTH-001`
    let (code, stdout, stderr) = run_compact_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line 'No deleted items to remove'
    assert!(
        stdout.lines().any(|l| l == "No deleted items to remove"),
        "missing no-op sentinel; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when forcing is required but absent
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_force_required_but_absent() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("specifying", &rules_json(1, 1), 1));

    // @step When I run `./codelet/target/release/fspec compact-work-unit AUTH-001`
    let (code, stdout, stderr) = run_compact_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to compact work unit:'
    assert!(
        stderr.contains("✗ Failed to compact work unit:"),
        "stderr must contain TS-parity prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Cannot compact work unit in 'specifying' status."
    assert!(
        stderr.contains("Cannot compact work unit in 'specifying' status."),
        "stderr must report force-gate error; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 1 deleted rule
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("done", &rules_json(1, 1), 1));

    // @step When I dispatch compact-work-unit via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    let req = codelet_fspec_core::DispatchRequest {
        command: "compact-work-unit".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And the AUTH-001 rules array in spec/work-units.json contains 0 items
    let data = read_work_units(ws.path());
    let rules = data["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rules.len(), 0, "expected 0 surviving rules; got {data}");

    // @step And the CLI bridge module codelet/fspec/src/compact_work_unit.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compact_work_unit.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/compact_work_unit.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "does not exist",
        "Compacted work unit",
        "No deleted items to remove",
        "write_json_atomic",
        "ensure_work_units_file",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: compact-work-unit --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_CWU: &str = include_str!("fixtures/help/compact-work-unit.txt");

#[test]
fn scenario_compact_work_unit_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec compact-work-unit --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("compact-work-unit")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn compact-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "compact-work-unit --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/compact-work-unit.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CWU);

    // @step And stdout starts with a blank line followed by 'COMPACT-WORK-UNIT'
    assert!(stdout.starts_with("\nCOMPACT-WORK-UNIT\n"));
}
