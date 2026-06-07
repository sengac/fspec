//! CLI surface for the `list-work-units` subcommand on the standalone fspec
//! Rust binary — RPC-253 follow-up (CLI argv → fspec_core::commands::
//! list_work_units::run).
//!
//! Feature: spec/features/list-work-units-cli-subcommand.feature
//!
//! Covers the six CLI-level scenarios added to the feature file when the
//! original dispatcher-only scope was broadened to include the shell argv
//! surface:
//!   1. Standalone fspec binary exposes list-work-units as a clap subcommand
//!   2. CLI list-work-units against empty directory creates default files and
//!      prints sentinel
//!   3. CLI list-work-units emits 2-space indented JSON when --format=json is
//!      passed
//!   4. CLI list-work-units --status filter matches dispatcher behavior
//!   5. CLI list-work-units exits 1 and writes to stderr when work-units.json
//!      is malformed
//!   6. Default combined TUI mode is preserved when no subcommand is provided
//!
//! Red phase: these tests MUST fail today because `codelet/fspec/src/main.rs`
//! only registers `daemon`, `client`, and `status` clap subcommands —
//! `fspec list-work-units` returns clap's "unrecognized subcommand" diagnostic
//! and exit code 2 (NOT the structured exit-1-with-stderr path these tests
//! expect on the success / malformed-JSON paths). The CLI wrapper that bridges
//! clap argv → fspec_core::commands::list_work_units::run is the implementation
//! deliverable that flips these from red to green.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Build an empty workspace tempdir with NO `spec/` subdirectory. The CLI
/// wrapper is expected to auto-create both spec/work-units.json and
/// spec/prefixes.json on first run.
fn empty_workspace() -> TempDir {
    tempfile::tempdir().expect("create empty workspace tempdir")
}

/// Build a workspace tempdir with the given JSON value written verbatim to
/// `spec/work-units.json`. Used to seed the filter-behaviour scenarios with
/// the canonical (AUTH-001, AUTH-002, DASH-001) fixture and the malformed-JSON
/// scenario.
fn workspace_with_work_units_json(body: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

/// Canonical three-unit fixture matching the dispatcher-level test in
/// `codelet/fspec-core/tests/list_work_units.rs`. AUTH-001 (backlog, epic ux),
/// AUTH-002 (implementing), DASH-001 (backlog) in that insertion order.
fn three_unit_store_json() -> String {
    r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "backlog",
      "epic": "ux",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout feature",
      "status": "implementing",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "DASH-001": {
      "id": "DASH-001",
      "title": "User dashboard",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
    .to_string()
}

/// Spawn `fspec list-work-units` with the given extra args and CWD. Returns
/// (exit_code, stdout, stderr) for assertion. We never pass `--workspace`
/// because the CLI wrapper for argv-driven invocation MUST resolve the
/// project root from CWD (parity with the TS `process.cwd()` default).
fn run_list_work_units(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-work-units");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-work-units");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Standalone fspec binary exposes list-work-units as a clap
//           subcommand
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_standalone_fspec_binary_exposes_list_work_units_as_a_clap_subcommand() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by `env!("CARGO_BIN_EXE_fspec")` in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-work-units --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-work-units")
        .arg("--help")
        .output()
        .expect("spawn fspec list-work-units --help");

    // @step Then the command exits 0 and prints TS-style help listing --status, --prefix, --epic flags
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "fspec list-work-units --help must exit 0; got exit {code}, stderr={stderr}"
    );
    // clap routes `--help` output to stdout by default.
    let help = if stdout.is_empty() { stderr.to_string() } else { stdout.to_string() };
    for flag in ["--status", "--prefix", "--epic"] {
        assert!(
            help.contains(flag),
            "help output must list {flag}; got:\n{help}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI list-work-units against empty directory creates default
//           files and prints sentinel
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_dir_creates_defaults_and_prints_sentinel() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = empty_workspace();
    assert!(
        !ws.path().join("spec").exists(),
        "precondition: spec/ must not exist before running the CLI"
    );

    // @step When I run `./codelet/target/release/fspec list-work-units` from that directory
    let (code, stdout, stderr) = run_list_work_units(ws.path(), &[]);

    // @step Then the command exits 0 and prints 'No work units found' to stdout
    assert_eq!(
        code, 0,
        "fspec list-work-units must exit 0 on empty workspace; got exit {code}, stderr={stderr}"
    );
    assert!(
        stdout.contains("No work units found"),
        "stdout must contain the sentinel 'No work units found'; got:\n{stdout}"
    );

    // @step Then spec/work-units.json and spec/prefixes.json are created in the directory
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created on first run"
    );
    assert!(
        ws.path().join("spec/prefixes.json").exists(),
        "spec/prefixes.json must be auto-created on first run"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI list-work-units emits 2-space indented JSON when
//           --format=json is passed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_emits_two_space_indented_json_for_format_json() {
    // @step Given spec/work-units.json contains AUTH-001 (backlog, epic 'ux') and AUTH-002 (implementing)
    let ws = workspace_with_work_units_json(&three_unit_store_json());
    // Drop DASH-001 from the fixture by overwriting with a two-unit store,
    // matching the scenario wording exactly:
    let two_unit = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "backlog",
      "epic": "ux",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout feature",
      "status": "implementing",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;
    fs::write(ws.path().join("spec/work-units.json"), two_unit)
        .expect("rewrite two-unit fixture");

    // @step When I run `./codelet/target/release/fspec list-work-units --format=json`
    let (code, stdout, stderr) = run_list_work_units(ws.path(), &["--format=json"]);

    // @step Then the command exits 0 and stdout contains a parseable JSON object with a workUnits array of length 2
    assert_eq!(
        code, 0,
        "fspec list-work-units --format=json must exit 0; got exit {code}, stderr={stderr}"
    );
    let parsed: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be parseable JSON");
    let arr = parsed
        .get("workUnits")
        .and_then(Value::as_array)
        .expect("top-level workUnits array");
    assert_eq!(
        arr.len(),
        2,
        "workUnits array must have exactly 2 entries; got {arr:?}"
    );

    // @step Then the JSON includes id, title, status, and epic 'ux' for AUTH-001 in insertion order
    let first = &arr[0];
    assert_eq!(first.get("id").and_then(Value::as_str), Some("AUTH-001"));
    assert_eq!(
        first.get("title").and_then(Value::as_str),
        Some("Login feature")
    );
    assert_eq!(first.get("status").and_then(Value::as_str), Some("backlog"));
    assert_eq!(first.get("epic").and_then(Value::as_str), Some("ux"));
    let second = &arr[1];
    assert_eq!(second.get("id").and_then(Value::as_str), Some("AUTH-002"));
    // AUTH-002 has no epic — must NOT appear in output (rule [6]).
    assert!(
        second.get("epic").is_none(),
        "AUTH-002 must not carry an epic field; got {second:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI list-work-units --status filter matches dispatcher behavior
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_status_filter_matches_dispatcher_behavior() {
    // @step Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    let ws = workspace_with_work_units_json(&three_unit_store_json());

    // @step When I run `./codelet/target/release/fspec list-work-units --status=backlog --format=json`
    let (code, stdout, stderr) =
        run_list_work_units(ws.path(), &["--status=backlog", "--format=json"]);

    // @step Then stdout contains a JSON workUnits array of length 2 with AUTH-001 and DASH-001 in that order
    let parsed: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be parseable JSON");
    let arr = parsed
        .get("workUnits")
        .and_then(Value::as_array)
        .expect("top-level workUnits array");
    let ids: Vec<&str> = arr
        .iter()
        .map(|e| e.get("id").and_then(Value::as_str).unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        vec!["AUTH-001", "DASH-001"],
        "status=backlog filter must keep AUTH-001 + DASH-001 in insertion order; got {ids:?}"
    );

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-work-units --status=backlog must exit 0; got exit {code}, stderr={stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI list-work-units exits 1 and writes to stderr when
//           work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = workspace_with_work_units_json("{ this is not valid json");

    // @step When I run `./codelet/target/release/fspec list-work-units`
    let (code, stdout, stderr) = run_list_work_units(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-work-units must exit 1 on malformed work-units.json; got exit {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain 'Failed to parse work-units.json'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is
//           provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_is_preserved_when_no_subcommand() {
    // @step Given the fspec Rust binary has list-work-units registered as a clap subcommand alongside daemon, client, and status
    // (Asserted via the help-listing check below.)

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, and list-work-units as available subcommands
    for sub in ["daemon", "client", "status", "list-work-units"] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI tolerates unknown work-unit type values for parity with the
//           TypeScript runtime
//
// Defect: the previously-shipped Rust port modelled `WorkUnit.type` as the
// strict `WorkUnitType { Story, Task, Bug }` enum and rejected any other
// string at serde-deserialization time. The real spec/work-units.json in
// this repo contains type="feature" (line 52721) which the TS implementation
// accepts at runtime (TS type union is a compile-time-only constraint —
// JSON.parse accepts any string). Rust MUST mirror that runtime tolerance.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_tolerates_unknown_work_unit_type_values() {
    // @step Given spec/work-units.json contains FEAT-001 with type 'feature' (a value outside story/task/bug) and AUTH-001 with no type field
    let store = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "FEAT-001": {
      "id": "FEAT-001",
      "title": "Pre-existing feature-typed unit",
      "type": "feature",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;
    let ws = workspace_with_work_units_json(store);

    // @step When I run `./codelet/target/release/fspec list-work-units --type=story --format=json`
    let (code, stdout, stderr) =
        run_list_work_units(ws.path(), &["--type=story", "--format=json"]);

    // @step Then the command exits 0 and stdout contains a JSON workUnits array of length 1 with AUTH-001
    assert_eq!(
        code, 0,
        "fspec list-work-units --type=story must exit 0 against a file containing a `feature`-typed unit; got exit {code}, stdout={stdout}, stderr={stderr}"
    );
    let parsed: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be parseable JSON");
    let arr = parsed
        .get("workUnits")
        .and_then(Value::as_array)
        .expect("top-level workUnits array");
    let ids: Vec<&str> = arr
        .iter()
        .map(|e| e.get("id").and_then(Value::as_str).unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        vec!["AUTH-001"],
        "--type=story must include AUTH-001 (missing type defaults to story) and EXCLUDE FEAT-001 (type=feature); got {ids:?}"
    );

    // @step Then stderr does NOT contain the substring 'unknown variant'
    assert!(
        !stderr.contains("unknown variant"),
        "stderr must not surface a serde `unknown variant` diagnostic for type=feature; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Subcommand help excludes the global workspace flag
//
// Defect: codelet/fspec/src/main.rs declares `--workspace` with
// `global = true`, which makes clap inherit it into every subcommand's
// --help — including list-work-units, which does NOT consume cli.workspace
// (parity with TS `process.cwd()` per rule [15]). The flag must be removed
// from `list-work-units --help`.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_subcommand_help_excludes_global_workspace_flag() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-work-units --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-work-units")
        .arg("--help")
        .output()
        .expect("spawn fspec list-work-units --help");

    // @step Then the command exits 0
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        code, 0,
        "fspec list-work-units --help must exit 0; got exit {code}, stderr={stderr}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        !stdout.contains("--workspace"),
        "list-work-units --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-work-units --help (RPC-253)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_LWU: &str = include_str!("fixtures/help/list-work-units.txt");

#[test]
fn scenario_list_work_units_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec list-work-units --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-work-units")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-work-units --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "list-work-units --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-work-units.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_LWU);

    // @step And stdout starts with a blank line followed by 'LIST-WORK-UNITS'
    assert!(stdout.starts_with("\nLIST-WORK-UNITS\n"));

    // @step And stdout contains the section header 'TYPICAL WORKFLOW'
    assert!(stdout.contains("TYPICAL WORKFLOW\n"));
}
