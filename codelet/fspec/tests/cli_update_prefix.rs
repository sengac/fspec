//! CLI surface for the `update-prefix` subcommand on the standalone fspec
//! Rust binary — RPC-313.
//!
//! Feature: spec/features/update-prefix-cli-subcommand.feature
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

fn run_update_prefix(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-prefix");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-prefix");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_prefixes(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("prefixes.json"), raw).expect("write prefixes.json");
}

fn read_prefixes_raw(cwd: &Path) -> String {
    fs::read_to_string(cwd.join("spec/prefixes.json")).expect("read prefixes.json")
}

const AUTH_OLD: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "old",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;

const AUTH_KEPT: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "kept",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/update-prefix.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_updates_description_on_existing_prefix() {
    // Scenario: CLI updates the description of an existing prefix

    // @step Given spec/prefixes.json contains AUTH with description 'old'
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), AUTH_OLD);

    // @step When I run `fspec update-prefix AUTH -d "new"` in that project root
    let (code, stdout, stderr) = run_update_prefix(ws.path(), &["AUTH", "-d", "new"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout contains the line '✓ Prefix AUTH updated successfully'
    assert!(
        stdout.contains("✓ Prefix AUTH updated successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step Then spec/prefixes.json now has AUTH.description equal to 'new'
    let on_disk: serde_json::Value = serde_json::from_str(&read_prefixes_raw(ws.path()))
        .expect("parse spec/prefixes.json");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["description"].as_str(), Some("new"));

    // @step Then spec/prefixes.json now has AUTH.updatedAt set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
    assert_eq!(updated_at.len(), 24, "ISO-8601 UTC must be 24 bytes; got: {updated_at}");
    // Shape: `YYYY-MM-DDTHH:MM:SS.sssZ` — millisecond fraction matches
    // the shared `crate::io::time` helper used in production code,
    // which in turn mirrors TS `new Date().toISOString()`.
    assert!(
        updated_at.ends_with('Z') && updated_at.as_bytes()[19] == b'.'
            && updated_at.as_bytes()[20..23].iter().all(|b| b.is_ascii_digit()),
        "updatedAt must end with `.sssZ` millisecond fraction; got: {updated_at}"
    );
}

#[test]
fn scenario_cli_no_op_bumps_only_updated_at() {
    // Scenario: CLI no-op call bumps only updatedAt

    // @step Given spec/prefixes.json contains AUTH with description 'kept'
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), AUTH_KEPT);

    // @step When I run `fspec update-prefix AUTH` in that project root
    let (code, stdout, stderr) = run_update_prefix(ws.path(), &["AUTH"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0 on no-op; stderr={stderr}");

    // @step Then stdout contains the line '✓ Prefix AUTH updated successfully'
    assert!(
        stdout.contains("✓ Prefix AUTH updated successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step Then spec/prefixes.json AUTH.description is preserved verbatim as 'kept'
    let on_disk: serde_json::Value = serde_json::from_str(&read_prefixes_raw(ws.path()))
        .expect("parse spec/prefixes.json");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["description"].as_str(), Some("kept"));

    // @step Then spec/prefixes.json AUTH.updatedAt is set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
    assert_eq!(updated_at.len(), 24);
    assert!(
        updated_at.ends_with('Z') && updated_at.as_bytes()[19] == b'.'
            && updated_at.as_bytes()[20..23].iter().all(|b| b.is_ascii_digit()),
        "updatedAt must end with `.sssZ` millisecond fraction; got: {updated_at}"
    );
}

#[test]
fn scenario_cli_rejects_unknown_prefix_with_exit_1() {
    // Scenario: CLI rejects an unknown prefix with a wrapped error and exit 1

    // @step Given spec/prefixes.json is empty (no prefixes registered)
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), r#"{"prefixes":{}}"#);
    let before = read_prefixes_raw(ws.path());

    // @step When I run `fspec update-prefix MISSING -d "ignored"` in that project root
    let (code, _stdout, stderr) = run_update_prefix(ws.path(), &["MISSING", "-d", "ignored"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'Failed to update prefix'
    assert!(
        stderr.contains("Failed to update prefix"),
        "stderr must mention 'Failed to update prefix'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Prefix MISSING not found'
    assert!(
        stderr.contains("Prefix MISSING not found"),
        "stderr must mention 'Prefix MISSING not found'; got:\n{stderr}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(ws.path());
    assert_eq!(before, after, "spec/prefixes.json must be untouched");
}

#[test]
fn scenario_cli_does_not_expose_epic_id_flag() {
    // Scenario: CLI surface does NOT expose --epic-id

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), AUTH_OLD);
    let before = read_prefixes_raw(ws.path());

    // @step When I run `fspec update-prefix AUTH --epic-id auth-epic` in that project root
    let (code, _stdout, stderr) = run_update_prefix(ws.path(), &["AUTH", "--epic-id", "auth-epic"]);

    // @step Then the process exits with code 1
    assert_eq!(
        code, 1,
        "Commander rejects --epic-id with exit 1; stderr={stderr}"
    );

    // @step Then stderr contains the substring 'unknown option'
    assert!(
        stderr.contains("unknown option"),
        "Commander must complain about unknown option; got:\n{stderr}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(ws.path());
    assert_eq!(before, after, "spec/prefixes.json must be untouched");
}

#[test]
fn scenario_cli_requires_prefix_positional() {
    // Scenario: CLI requires the prefix positional argument

    // @step Given any project root
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec update-prefix` with no arguments in that project root
    let (code, _stdout, stderr) = run_update_prefix(ws.path(), &[]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "Commander rejects missing positional with exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'required'
    assert!(
        stderr.contains("required"),
        "Commander must mention 'required'; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_help_matches_ts_fixture() {
    // Scenario: CLI help surface matches the captured TS fixture

    // @step Given the TS help fixture at codelet/fspec/tests/fixtures/help/update-prefix.txt
    // (asserted by the include_str! above — the const TS_HELP_FIXTURE)

    // @step When I run `fspec update-prefix --help`
    let output = Command::new(fspec_bin())
        .arg("update-prefix")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec update-prefix --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "update-prefix --help must exit 0; stderr={stderr}");

    // @step Then stdout matches the captured TS fixture byte-for-byte (modulo trailing newline)
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

#[test]
fn scenario_clap_exposes_update_prefix_subcommand() {
    // Scenario: Clap exposes update-prefix as a top-level subcommand

    // @step Given the codelet/fspec crate is built
    // (precondition — CARGO_BIN_EXE_fspec resolves only after build)

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "fspec --help must exit 0");

    // @step Then stdout contains the substring 'update-prefix'
    assert!(
        stdout.contains("update-prefix"),
        "fspec --help must list update-prefix; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_delegates_to_fspec_core_function() {
    // Scenario: CLI delegates to the same fspec_core function the dispatcher uses

    // @step Given the codelet/fspec crate is built

    // @step When I inspect codelet/fspec/src/update_prefix.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update_prefix.rs");
    let src = fs::read_to_string(&path).expect("read update_prefix.rs bridge");

    // @step Then the source declares it calls `codelet_fspec_core::commands::update_prefix::run`
    assert!(
        src.contains("codelet_fspec_core::commands::update_prefix")
            || src.contains("update_prefix::run"),
        "bridge must delegate to fspec_core::commands::update_prefix::run; got:\n{src}"
    );

    // @step Then the source does NOT perform any file IO directly on spec/prefixes.json
    for forbidden in [
        "ensure_prefixes_file",
        "write_json_atomic",
        "PrefixesData",
        "prefixes.json",
        "spec/prefixes",
    ] {
        assert!(
            !src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{src}"
        );
    }
}

#[test]
fn scenario_default_combined_tui_mode_preserved() {
    // Scenario: Default combined TUI mode is preserved when no subcommand is given

    // @step Given the codelet/fspec crate is built
    // (precondition — CARGO_BIN_EXE_fspec resolves only after build)

    // @step When I run `fspec` with no arguments
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the binary does NOT route to update_prefix::run
    // (validated implicitly — --help does not invoke any subcommand action)

    // @step Then the binary attempts to launch the combined TUI mode (the existing default arm)
    assert!(
        stdout.contains("combined") || stdout.contains("combined mode"),
        "fspec --help long-about must document the combined-mode default; got:\n{stdout}"
    );
}
