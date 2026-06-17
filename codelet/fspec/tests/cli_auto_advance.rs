//! CLI surface for the `auto-advance` subcommand on the standalone fspec
//! Rust binary — RPC-198.
//!
//! Feature: spec/features/auto-advance-cli-subcommand.feature
//!
//! Framing A: the TypeScript shell is broken — the Commander action calls
//! `autoAdvance({ dryRun })` and NEVER wires workUnitId/from/event, so the
//! function reads `data.workUnits[undefined]`, which is missing, and ALWAYS
//! fails with "Work unit undefined not found", wrapped as
//! "Failed to auto-advance: Work unit undefined not found", exit code 1. The
//! Rust CLI bridge mirrors this broken behaviour verbatim (it marshals an
//! empty args object, ignoring --dry-run).
//!
//! PHASE B (red): until main.rs registers the clap subcommand AND the
//! intercept arm is added, these tests fail (subcommand not yet wired).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

mod common;

use common::fspec_bin;

fn workspace_with_valid_work_units() -> TempDir {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let body = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "testing",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": ["AUTH-001"],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#;
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

fn run_auto_advance(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("auto-advance");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec auto-advance");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shell auto-advance reproduces the broken Framing-A failure
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_shell_auto_advance_reproduces_broken_framing_a_failure() {
    // @step Given a working directory with a valid spec/work-units.json
    let ws = workspace_with_valid_work_units();

    // @step When I run `fspec auto-advance` from that directory
    let (code, stdout, stderr) = run_auto_advance(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "auto-advance must exit 1 (Framing A); stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains '✗ Failed to auto-advance:'
    assert!(
        stderr.contains("✗ Failed to auto-advance:"),
        "stderr must contain '✗ Failed to auto-advance:'; got:\n{stderr}"
    );

    // @step And stderr contains 'Work unit undefined not found'
    assert!(
        stderr.contains("Work unit undefined not found"),
        "stderr must contain 'Work unit undefined not found'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shell auto-advance with --dry-run behaves identically
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_shell_auto_advance_dry_run_behaves_identically() {
    // @step Given a working directory with a valid spec/work-units.json
    let ws = workspace_with_valid_work_units();

    // @step When I run `fspec auto-advance --dry-run` from that directory
    let (code, stdout, stderr) = run_auto_advance(ws.path(), &["--dry-run"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "auto-advance --dry-run must exit 1 (Framing A); stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains 'Work unit undefined not found'
    assert!(
        stderr.contains("Work unit undefined not found"),
        "stderr must contain 'Work unit undefined not found'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: auto-advance --help is byte-for-byte identical to the TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/auto-advance.txt");

#[test]
fn scenario_auto_advance_help_matches_ts_format_command_help_reference() {
    // @step Given the fspec Rust binary has been compiled

    // @step When I run `fspec auto-advance --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("auto-advance")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec auto-advance --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "auto-advance --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/auto-advance.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "auto-advance --help must match the TS fixture byte-for-byte; stderr={stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI bridge delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_bridge_delegates_to_same_fspec_core_function() {
    // @step Given the CLI bridge module codelet/fspec/src/auto_advance.rs
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auto_advance.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/auto_advance.rs must exist as the CLI bridge module; missing: {}",
        bridge_path.display()
    );

    // @step When I inspect its source
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then it contains no inline state-transition or work-units mutation logic
    // @step And its only computation is JSON arg marshalling before delegating to fspec_core::commands::auto_advance::run
    for forbidden in [
        "STATE_TRANSITIONS",
        "write_json_atomic",
        "No transition defined",
        "states.testing",
        "newState",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
    assert!(
        bridge_src.contains("auto_advance::run"),
        "bridge must delegate to fspec_core::commands::auto_advance::run; got:\n{bridge_src}"
    );
}
