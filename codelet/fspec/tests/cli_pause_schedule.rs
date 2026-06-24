//! CLI surface for the `pause-schedule` subcommand on the standalone
//! fspec Rust binary — RPC-254.
//!
//! Feature: spec/features/pause-schedule-cli-subcommand.feature
//!         spec/features/pause-schedule-rust-port.feature
//!
//! RED phase: the impl at
//! `codelet/fspec-core/src/commands/pause_schedule.rs` is still a stub, the
//! clap subcommand is NOT yet wired into `codelet/fspec/src/main.rs`, and
//! the bridge module `codelet/fspec/src/pause_schedule.rs` does not exist.
//! Every assertion below FAILS until the port lands. These tests exercise
//! the end-to-end shell surface and assert the "two-front-doors" parity
//! with the `dispatch_command` path.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_pause_schedule(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("pause-schedule");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec pause-schedule");
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

fn read_schedules_raw(cwd: &Path) -> String {
    fs::read_to_string(cwd.join("spec/schedules.json")).expect("read schedules.json")
}

fn read_schedules(cwd: &Path) -> serde_json::Value {
    serde_json::from_str(&read_schedules_raw(cwd)).expect("schedules.json is valid JSON")
}

fn one_shell_schedule(name: &str, status: &str) -> String {
    format!(
        r#"{{
  "version": "1.0.0",
  "schedules": {{
    "{name}": {{
      "name": "{name}",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "{status}",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "npm run build"
    }}
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI pauses an active schedule and prints a success message
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_pauses_active_schedule_and_prints_success() {
    // @step Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    let ws = tempfile::tempdir().expect("tempdir");
    write_schedules(ws.path(), &one_shell_schedule("nightly-review", "active"));

    // @step When I run `fspec pause-schedule nightly-review` from a shell against that project root
    let (code, stdout, stderr) = run_pause_schedule(ws.path(), &["nightly-review"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec pause-schedule must exit 0 on success; got {code}, stderr={stderr}"
    );

    // @step And stdout contains "✓ Schedule 'nightly-review' paused successfully"
    assert!(
        stdout.contains("✓ Schedule 'nightly-review' paused successfully"),
        "stdout must contain the success message; got:\n{stdout}"
    );

    // @step And spec/schedules.json now records the 'nightly-review' schedule with status 'paused'
    let data = read_schedules(ws.path());
    assert_eq!(
        data["schedules"]["nightly-review"]["status"].as_str(),
        Some("paused"),
        "nightly-review must be paused after the CLI call; got {data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports an error and exits 1 when the schedule does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_errors_and_exits_1_when_schedule_missing() {
    // @step Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    let ws = tempfile::tempdir().expect("tempdir");
    let original = one_shell_schedule("nightly-review", "active");
    write_schedules(ws.path(), &original);

    // @step When I run `fspec pause-schedule ghost` from a shell against that project root
    let (code, _stdout, stderr) = run_pause_schedule(ws.path(), &["ghost"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec pause-schedule must exit 1 on error; got {code}"
    );

    // @step And stderr contains "Schedule 'ghost' does not exist"
    assert!(
        stderr.contains("Schedule 'ghost' does not exist"),
        "stderr must contain the does-not-exist error; got:\n{stderr}"
    );

    // @step And spec/schedules.json is unchanged
    assert_eq!(
        read_schedules_raw(ws.path()),
        original,
        "schedules.json must be untouched on the missing-schedule error path"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pause-schedule --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

/// Captured byte-exact TS reference output of
/// `node dist/index.js pause-schedule --help` piped to non-TTY.
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/pause-schedule.txt");

#[test]
fn scenario_pause_schedule_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec pause-schedule --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("pause-schedule")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn pause-schedule --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "pause-schedule --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/pause-schedule.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "pause-schedule --help output must be byte-for-byte identical to TS reference"
    );

    // @step And stdout starts with a blank line followed by 'PAUSE-SCHEDULE'
    assert!(
        stdout.starts_with("\nPAUSE-SCHEDULE\n"),
        "help must start with blank line then PAUSE-SCHEDULE header; got first 40 bytes:\n{:?}",
        &stdout.chars().take(40).collect::<String>()
    );

    // @step And stdout contains the section header 'ARGUMENTS' followed by '  <name> (required)'
    assert!(
        stdout.contains("ARGUMENTS\n  <name> (required)"),
        "help must contain ARGUMENTS section with the <name> argument; got:\n{stdout}"
    );

    // @step And stdout contains the line 'resume-schedule - Resume a paused schedule'
    assert!(
        stdout.contains("resume-schedule - Resume a paused schedule\n"),
        "help must list the resume-schedule related command; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    let ws_dispatch = tempfile::tempdir().expect("tempdir dispatch");
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    write_schedules(
        ws_dispatch.path(),
        &one_shell_schedule("nightly-review", "active"),
    );
    write_schedules(
        ws_cli.path(),
        &one_shell_schedule("nightly-review", "active"),
    );

    // @step When I dispatch pause-schedule through fspec_core::dispatch::dispatch_command with name='nightly-review' AND I separately invoke `fspec pause-schedule nightly-review` against an identical project root
    let req = codelet_fspec_core::DispatchRequest {
        command: "pause-schedule".to_string(),
        args_json: r#"{"name":"nightly-review"}"#.to_string(),
        project_root: ws_dispatch.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    let (code, _stdout, stderr) = run_pause_schedule(ws_cli.path(), &["nightly-review"]);
    assert_eq!(code, 0, "CLI path must exit 0; stderr={stderr}");

    // @step Then both call sites produce the identical status transition to 'paused' in spec/schedules.json
    let dispatch_status = read_schedules(ws_dispatch.path())["schedules"]["nightly-review"]
        ["status"]
        .as_str()
        .map(str::to_string);
    let cli_status = read_schedules(ws_cli.path())["schedules"]["nightly-review"]["status"]
        .as_str()
        .map(str::to_string);
    assert_eq!(dispatch_status.as_deref(), Some("paused"));
    assert_eq!(
        dispatch_status, cli_status,
        "dispatcher and CLI must produce the identical status transition"
    );

    // @step And the CLI bridge module codelet/fspec/src/pause_schedule.rs contains NO inline schedule-mutation, validation, or file-writing logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pause_schedule.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/pause_schedule.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "does not exist",
        "already paused",
        "write_json_atomic",
        "paused successfully",
        "\"status\"",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is marshalling the name argument into the JSON args shape
    assert!(
        bridge_src.contains("dispatch_command") || bridge_src.contains("pause_schedule::run"),
        "bridge module must delegate to dispatch_command or fspec_core::commands::pause_schedule::run; got:\n{bridge_src}"
    );
}
