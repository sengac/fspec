//! CLI surface for the `list-schedules` subcommand on the standalone
//! fspec Rust binary — RPC-250.
//!
//! Feature: spec/features/list-schedules-rust-port.feature
//!         spec/features/list-schedules-cli-subcommand.feature
//!
//! GREEN phase: the impl at
//! `codelet/fspec-core/src/commands/list_schedules.rs` is ported, the
//! clap subcommand is wired into `codelet/fspec/src/main.rs`, and the
//! bridge module `codelet/fspec/src/list_schedules.rs` translates the
//! TS Commander.js `--json` boolean to the dispatcher's
//! `format: "json"` key. These tests exercise the end-to-end shell
//! surface and assert the "two-front-doors" parity with the
//! `dispatch_command` path.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_list_schedules(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-schedules");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-schedules");
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

fn canonical_schedules_json() -> String {
    r#"{
  "version": "0.7.1",
  "schedules": {
    "nightly-build": {
      "name": "nightly-build",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "status": "active",
      "command": "npm run build",
      "lastRunAt": null
    }
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-schedules as a subcommand with --json flag only
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_schedules_with_json_flag_only() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-schedules --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-schedules")
        .arg("--help")
        .output()
        .expect("spawn fspec list-schedules --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-schedules --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout describes the list-schedules subcommand
    assert!(
        stdout.contains("list-schedules") || stdout.contains("scheduled jobs"),
        "help must describe the list-schedules subcommand; got:\n{stdout}"
    );

    // @step Then stdout advertises the --json flag
    assert!(
        stdout.contains("--json"),
        "list-schedules --help must advertise --json; got:\n{stdout}"
    );

    // @step Then stdout does NOT advertise unrelated flags
    for forbidden in ["--status", "--prefix", "--epic", "--format", "--category", "--workspace"]
    {
        assert!(
            !stdout.contains(forbidden),
            "list-schedules --help must NOT advertise {forbidden}; got:\n{stdout}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory prints sentinel and does not auto-create files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_prints_sentinel_and_no_files() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-schedules` from that directory
    let (code, stdout, stderr) = run_list_schedules(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-schedules must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the sentinel 'No schedules configured.'
    assert!(
        stdout.contains("No schedules configured."),
        "stdout must contain 'No schedules configured.'; got:\n{stdout}"
    );

    // @step Then spec/schedules.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/schedules.json").exists(),
        "list-schedules must NOT auto-create spec/schedules.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: --json flag produces JSON payload
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_json_flag_emits_json_payload() {
    // @step Given an empty project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec list-schedules --json`
    let (code, stdout, stderr) = run_list_schedules(ws.path(), &["--json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-schedules --json must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout starts with the canonical JSON prefix
    assert!(
        stdout.trim_start().starts_with("{\n  \"schedules\": [],\n"),
        "stdout must begin with the canonical pretty-printed JSON; got:\n{stdout}"
    );

    // @step Then stdout contains the columns array
    assert!(
        stdout.contains("\"columns\":"),
        "stdout must contain the canonical columns key; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_populated_case() {
    // @step Given spec/schedules.json contains one shell schedule named 'nightly-build'
    let ws = tempfile::tempdir().expect("tempdir");
    write_schedules(ws.path(), &canonical_schedules_json());

    // @step When I run `./codelet/target/release/fspec list-schedules`
    let (code, stdout, stderr) = run_list_schedules(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-schedules must exit 0 on populated case; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the tab-separated header line
    assert!(
        stdout
            .lines()
            .any(|l| l == "Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run"),
        "stdout must contain tab-separated header; got:\n{stdout}"
    );

    // @step Then stdout contains the nightly-build row
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with("nightly-build\t0 2 * * *\tUTC\tshell\tactive\t")),
        "stdout must contain nightly-build data row; got:\n{stdout}"
    );

    // @step Then stdout contains the summary line
    assert!(
        stdout.lines().any(|l| l == "Total: 1 schedule(s)"),
        "stdout must contain 'Total: 1 schedule(s)'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode preserved after adding list-schedules
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_schedules() {
    // @step Given the fspec Rust binary has list-schedules registered alongside other subcommands
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

    // @step Then the help output lists list-schedules as an available subcommand
    assert!(
        help.contains("list-schedules"),
        "fspec --help must list `list-schedules` subcommand; got:\n{help}"
    );

    // @step Then the help output still documents the combined-mode default
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/schedules.json contains one shell schedule
    let ws = tempfile::tempdir().expect("tempdir");
    write_schedules(ws.path(), &canonical_schedules_json());

    // @step When I dispatch list-schedules through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-schedules".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let schedules_arr = dispatcher_data["schedules"]
        .as_array()
        .expect("schedules array");
    assert_eq!(schedules_arr.len(), 1);
    assert_eq!(schedules_arr[0]["name"].as_str(), Some("nightly-build"));

    // @step Then the CLI bridge path renders the same underlying data
    let (code, stdout, _stderr) = run_list_schedules(ws.path(), &["--json"]);
    assert_eq!(code, 0);
    let cli_data: serde_json::Value =
        serde_json::from_str(&stdout).expect("CLI --json stdout is JSON");
    assert_eq!(cli_data, dispatcher_data,
        "CLI --json output must equal dispatcher format='json' output verbatim");

    // @step Then the CLI bridge module contains NO inline schedule-aggregation, filter, or rendering logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_schedules.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_schedules.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // Forbid any inline aggregation/rendering markers — the bridge must only marshall args
    // and delegate. (Same anti-duplication guard used by RPC-253 in the
    // list_work_units bridge.)
    for forbidden in [
        "No schedules configured",
        "Total: ",
        "Name\\tCron",
        "lastRunAt",
        "See cron",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
