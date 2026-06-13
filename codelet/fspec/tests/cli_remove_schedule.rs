//! CLI surface for the `remove-schedule` subcommand on the standalone
//! fspec Rust binary — RPC-280.
//!
//! Feature: spec/features/remove-schedule-cli-subcommand.feature
//!          spec/features/remove-schedule-rust-port.feature
//!
//! RED phase: the impl at
//! `codelet/fspec-core/src/commands/remove_schedule.rs` is still a
//! `NotYetPorted` stub, the clap `Mode::RemoveSchedule` variant is NOT yet
//! wired into `codelet/fspec/src/main.rs`, the `intercept_ts_help`
//! arm does NOT exist, and the bridge module
//! `codelet/fspec/src/remove_schedule.rs` has NOT been created. Every
//! assertion below therefore FAILS until the Phase-C impl + wiring land.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_remove_schedule(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-schedule");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-schedule");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_schedules(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("schedules.json"), raw).expect("write schedules.json");
}

fn schedule_entry(cwd: &Path, name: &str) -> Option<serde_json::Value> {
    let path = cwd.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&raw).ok()?;
    data.get("schedules")?.get(name).cloned()
}

fn canonical_schedules_json() -> String {
    r#"{
  "version": "1.0.0",
  "schedules": {
    "nightly-review": {
      "name": "nightly-review",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "echo n"
    }
  }
}"#
    .to_string()
}

/// Captured byte-exact TS reference output of
/// `node dist/index.js remove-schedule --help` piped to non-TTY.
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-schedule.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: remove-schedule --help is byte-for-byte identical to the TS
//           reference output
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_remove_schedule_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec remove-schedule --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("remove-schedule")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-schedule --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "remove-schedule --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/remove-schedule.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "remove-schedule --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout describes the single positional <name> argument and advertises no flags
    assert!(
        stdout.contains("<name> (required)"),
        "help must describe the positional <name> argument; got:\n{stdout}"
    );
    assert!(
        stdout.contains("No options available"),
        "help must advertise no flags (No options available); got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes a schedule and delegates to the same fspec_core
//           function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_schedule_and_delegates_to_fspec_core() {
    // @step Given a project root whose spec/schedules.json contains a schedule named 'nightly-review'
    let ws = tempfile::tempdir().expect("tempdir");
    write_schedules(ws.path(), &canonical_schedules_json());

    // @step When I run `fspec remove-schedule nightly-review` from a shell against that project root
    let (code, _stdout, stderr) = run_remove_schedule(ws.path(), &["nightly-review"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec remove-schedule must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then spec/schedules.json contains no schedule named 'nightly-review'
    assert!(
        schedule_entry(ws.path(), "nightly-review").is_none(),
        "nightly-review must be removed after CLI remove"
    );

    // @step Then the CLI bridge module codelet/fspec/src/remove_schedule.rs contains NO schedule-mutation or file-writing logic beyond JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_schedule.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_schedule.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "does not exist",
        "write_json_atomic",
        ".remove(",
        "schedules.json",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
    // The bridge's only computation is JSON arg marshalling + delegation.
    assert!(
        bridge_src.contains("remove_schedule::run") || bridge_src.contains("dispatch_command"),
        "bridge module must delegate to fspec_core::commands::remove_schedule::run; got:\n{bridge_src}"
    );
}
