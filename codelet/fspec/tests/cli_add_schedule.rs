//! CLI surface for the `add-schedule` subcommand on the standalone
//! fspec Rust binary — RPC-191.
//!
//! Feature: spec/features/add-schedule-cli-subcommand.feature
//!          spec/features/add-schedule-rust-port.feature
//!
//! RED phase: the impl at
//! `codelet/fspec-core/src/commands/add_schedule.rs` is still a
//! `NotYetPorted` stub, the clap `Mode::AddSchedule` variant is NOT yet
//! wired into `codelet/fspec/src/main.rs`, the `intercept_ts_help`
//! arm does NOT exist, and the bridge module
//! `codelet/fspec/src/add_schedule.rs` has NOT been created. Every
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

fn run_add_schedule(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-schedule");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-schedule");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn schedule_entry(cwd: &Path, name: &str) -> Option<serde_json::Value> {
    let path = cwd.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&raw).ok()?;
    data.get("schedules")?.get(name).cloned()
}

/// Captured byte-exact TS reference output of
/// `node dist/index.js add-schedule --help` piped to non-TTY.
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-schedule.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-schedule --help is byte-for-byte identical to the TS
//           reference output
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_schedule_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec add-schedule --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-schedule")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-schedule --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-schedule --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/add-schedule.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "add-schedule --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout advertises the -n/--name, -c/--cron, -z/--timezone, -t/--type, -r/--role, -p/--prompt, --command, and -o/--overlap flags
    for flag in [
        "-n, --name",
        "-c, --cron",
        "-z, --timezone",
        "-t, --type",
        "-r, --role",
        "-p, --prompt",
        "--command",
        "-o, --overlap",
    ] {
        assert!(
            stdout.contains(flag),
            "add-schedule --help must advertise `{flag}`; got:\n{stdout}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI registers an agent schedule and delegates to the same
//           fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_registers_agent_schedule_and_delegates_to_fspec_core() {
    // @step Given an empty project root directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec add-schedule -n nightly-review -c "0 2 * * *" -z UTC -t agent -r "Security reviewer" -p "Review src/"` from a shell against that project root
    let (code, _stdout, stderr) = run_add_schedule(
        ws.path(),
        &[
            "-n",
            "nightly-review",
            "-c",
            "0 2 * * *",
            "-z",
            "UTC",
            "-t",
            "agent",
            "-r",
            "Security reviewer",
            "-p",
            "Review src/",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec add-schedule must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then spec/schedules.json contains a schedule named 'nightly-review' with jobType='agent'
    let entry = schedule_entry(ws.path(), "nightly-review")
        .expect("nightly-review entry must exist after CLI add");
    assert_eq!(entry["jobType"].as_str(), Some("agent"));

    // @step Then the CLI bridge module codelet/fspec/src/add_schedule.rs contains NO validation, schedule-construction, or file-writing logic beyond JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_schedule.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_schedule.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Invalid schedule name",
        "expected 5 fields",
        "Invalid timezone",
        "already exists",
        "write_json_atomic",
        "lastRunStatus",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
    // The bridge's only computation is JSON arg marshalling + delegation.
    assert!(
        bridge_src.contains("add_schedule::run") || bridge_src.contains("dispatch_command"),
        "bridge module must delegate to fspec_core::commands::add_schedule::run; got:\n{bridge_src}"
    );
}
