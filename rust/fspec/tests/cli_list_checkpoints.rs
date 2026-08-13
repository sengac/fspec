//! CLI surface for the `list-checkpoints` subcommand on the standalone fspec
//! Rust binary — RPC-242.
//!
//! Feature: spec/features/list-checkpoints-cli-subcommand.feature
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

fn run_list_checkpoints(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-checkpoints");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-checkpoints");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn init_git_repo(dir: &Path) {
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .status()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .status()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .status()
        .expect("git config name");
    fs::write(dir.join("README.md"), "# test\n").expect("seed README");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .status()
        .expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", "initial"])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

fn create_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str) {
    let ref_name = format!("refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}");
    let status = Command::new("git")
        .args(["update-ref", &ref_name, "HEAD"])
        .current_dir(dir)
        .status()
        .expect("git update-ref");
    assert!(status.success(), "git update-ref {ref_name} HEAD failed");
}

fn write_index_file(dir: &Path, work_unit_id: &str, entries: &[(&str, &str)]) {
    let index_dir = dir.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&index_dir).expect("mkdir fspec-checkpoints-index");
    let mut arr = String::from("[");
    for (i, (name, ts)) in entries.iter().enumerate() {
        if i > 0 {
            arr.push(',');
        }
        arr.push_str(&format!(
            r#"{{"name":"{name}","sha":"deadbeef","timestamp":"{ts}"}}"#
        ));
    }
    arr.push(']');
    let payload = format!(r#"{{"checkpoints":{arr}}}"#);
    fs::write(index_dir.join(format!("{work_unit_id}.json")), payload).expect("write index file");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-checkpoints as a subcommand with flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_checkpoints_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec list-checkpoints --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-checkpoints")
        .arg("--help")
        .output()
        .expect("spawn fspec list-checkpoints --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-checkpoints --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the list-checkpoints subcommand
    assert!(
        stdout.contains("list-checkpoints") || stdout.contains("List all checkpoints"),
        "help must describe the list-checkpoints subcommand; got:\n{stdout}"
    );

    // @step Then stdout describes a positional argument named work-unit-id or workUnitId
    assert!(
        stdout.contains("work-unit-id") || stdout.contains("workUnitId"),
        "help must describe a positional argument named work-unit-id or workUnitId; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-checkpoints --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-checkpoints --help must NOT advertise --workspace; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--prefix'
    assert!(
        !stdout.contains("--prefix"),
        "list-checkpoints --help must NOT advertise --prefix; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "list-checkpoints --help must NOT advertise --status; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-checkpoints --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_LC: &str = include_str!("fixtures/help/list-checkpoints.txt");

#[test]
fn scenario_list_checkpoints_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec list-checkpoints --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-checkpoints")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-checkpoints --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-checkpoints --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/list-checkpoints.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_LC);

    // @step And stdout starts with a blank line followed by 'LIST-CHECKPOINTS'
    assert!(stdout.starts_with("\nLIST-CHECKPOINTS\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Missing positional argument exits non-zero with clap's required-arg error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_missing_positional_argument_exits_non_zero() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec list-checkpoints` from a shell with NO positional argument
    let ws = tempfile::tempdir().expect("tempdir");
    let (code, stdout, stderr) = run_list_checkpoints(ws.path(), &[]);

    // @step Then the command exits with a non-zero code
    assert_ne!(
        code, 0,
        "fspec list-checkpoints with NO arg must exit non-zero; stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'work-unit-id' or the substring 'workUnitId'
    assert!(
        stderr.contains("work-unit-id")
            || stderr.contains("workUnitId")
            || stderr.contains("WORK_UNIT_ID"),
        "stderr must mention the missing required arg; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory prints sentinel and does not auto-init
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_prints_sentinel_and_no_git() {
    // @step Given an empty directory with no .git subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join(".git").exists());

    // @step When I run `./rust/target/release/fspec list-checkpoints AUTH-001` from that directory
    let (code, stdout, stderr) = run_list_checkpoints(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-checkpoints AUTH-001 must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'No checkpoints found for AUTH-001'
    assert!(
        stdout.contains("No checkpoints found for AUTH-001"),
        "stdout must contain 'No checkpoints found for AUTH-001'; got:\n{stdout}"
    );

    // @step Then the directory does NOT contain a .git subdirectory after the call
    assert!(
        !ws.path().join(".git").exists(),
        "list-checkpoints must NOT auto-create .git directory"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders manual checkpoint progress for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_manual_checkpoint() {
    // @step Given a git repository at the current working directory with a manual checkpoint 'baseline' for AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    create_checkpoint(ws.path(), "AUTH-001", "baseline");

    // @step Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    write_index_file(
        ws.path(),
        "AUTH-001",
        &[("baseline", "2026-06-01T10:00:00.000Z")],
    );

    // @step When I run the CLI with arg AUTH-001
    let (code, stdout, stderr) = run_list_checkpoints(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0 on the populated case; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Checkpoints for AUTH-001:'
    assert!(
        stdout.contains("Checkpoints for AUTH-001:"),
        "stdout must contain header; got:\n{stdout}"
    );

    // @step Then stdout contains the manual icon+label line
    // @step Then stdout contains the substring '📌 baseline (manual)'
    assert!(
        stdout.contains("\u{1F4CC}  baseline (manual)"),
        "stdout must contain manual icon+label line; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Created: 2026-06-01T10:00:00.000Z'
    assert!(
        stdout.contains("Created: 2026-06-01T10:00:00.000Z"),
        "stdout must contain Created line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_checkpoints() {
    // @step Given the Rust binary has list-checkpoints registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes
    // @step When I run the binary with --help
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "--help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, list-checkpoints as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-checkpoints",
    ] {
        assert!(
            help.contains(sub),
            "--help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running the binary with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "long-about must document combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose .git directory contains a manual checkpoint 'baseline' for AUTH-001 with index timestamp '2026-06-01T10:00:00.000Z'
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    create_checkpoint(ws.path(), "AUTH-001", "baseline");
    write_index_file(
        ws.path(),
        "AUTH-001",
        &[("baseline", "2026-06-01T10:00:00.000Z")],
    );

    // @step When I dispatch list-checkpoints through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-checkpoints".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = dispatcher_data["checkpoints"]
        .as_array()
        .expect("checkpoints array");
    assert_eq!(arr.len(), 1);

    // @step Then the dispatcher's DispatchResult.data shows checkpoint 'baseline' with timestamp '2026-06-01T10:00:00.000Z' and isAutomatic=false
    assert_eq!(arr[0]["name"].as_str(), Some("baseline"));
    assert_eq!(
        arr[0]["timestamp"].as_str(),
        Some("2026-06-01T10:00:00.000Z")
    );
    assert_eq!(arr[0]["isAutomatic"].as_bool(), Some(false));

    // @step When I run the CLI with arg AUTH-001 against the same on-disk state
    let (code, stdout, _stderr) = run_list_checkpoints(ws.path(), &["AUTH-001"]);
    assert_eq!(code, 0);

    // @step Then stdout contains the manual icon+label
    // @step Then stdout contains the substring '📌 baseline (manual)'
    assert!(
        stdout.contains("\u{1F4CC}  baseline (manual)"),
        "CLI text output must reflect the same baseline checkpoint as the dispatcher; got:\n{stdout}"
    );

    // @step Then the CLI bridge module contains NO inline checkpoint-listing, classification or rendering logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_checkpoints.rs");
    assert!(
        bridge_path.exists(),
        "src/list_checkpoints.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Checkpoints for",
        "No checkpoints found",
        "AUTO_CHECKPOINT_PATTERN",
        "list_ghost_checkpoints",
        "displayIcon",
        "isAutomatic",
        "fspec-checkpoints-index",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
