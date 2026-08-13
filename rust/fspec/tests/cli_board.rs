//! CLI surface for the `board` subcommand on the standalone fspec Rust
//! binary — RPC-199.
//!
//! Feature: spec/features/board-cli-subcommand.feature
//!
//! PHASE B (TESTING): the clap subcommand + CLI bridge are not yet wired,
//! so these tests are RED until PHASE C. Each scenario maps 1:1 to a
//! Gherkin scenario; @step comments mirror the step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_board(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("board");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec board");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_foundation(root: &Path) {
    write_file(root, "spec/foundation.json", "{}\n");
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/board.txt");

const WORK_UNITS_DONE5_IMPL3: &str = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "Logout", "status": "implementing", "estimate": 3, "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": ["AUTH-002"], "validating": [],
    "done": ["AUTH-001"], "blocked": []
  }
}"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes board with --format and --limit and prints byte-parity help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_board_with_format_and_limit_byte_parity_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec board --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("board")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec board --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "board --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/board.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout starts with the Commander usage line 'Usage: fspec board [options]'
    // RPC-199 parity fix: `board` has no custom -help.ts in TS, so `board
    // --help` is bare Commander.js output (NOT the rich formatCommandHelp).
    assert!(stdout.starts_with("Usage: fspec board [options]"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI emits JSON board with story-point summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_emits_json_board_with_story_point_summary() {
    // @step Given a project root whose spec/foundation.json exists and spec/work-units.json contains AUTH-001 (done, estimate 5) and AUTH-002 (implementing, estimate 3)
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_file(ws.path(), "spec/work-units.json", WORK_UNITS_DONE5_IMPL3);

    // @step When I run `./rust/target/release/fspec board --format json` from that directory
    let (code, stdout, stderr) = run_board(ws.path(), &["--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step Then stdout parses as JSON whose summary field reads '3 points in progress, 5 points completed'
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("board --format json stdout must be JSON");
    assert_eq!(
        v["summary"].as_str(),
        Some("3 points in progress, 5 points completed")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 with stderr when foundation.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_with_stderr_when_foundation_missing() {
    // @step Given a project root with no spec/foundation.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec board --format json` from that directory
    let (code, _stdout, stderr) = run_board(ws.path(), &["--format", "json"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step Then stderr describes the missing foundation
    assert!(
        stderr.contains("foundation") || stderr.contains("Foundation"),
        "stderr must describe the missing foundation; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/foundation.json exists and spec/work-units.json contains AUTH-001 (done, estimate 5)
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_file(
        ws.path(),
        "spec/work-units.json",
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [],
    "done": ["AUTH-001"], "blocked": []
  }
}"#,
    );

    // @step When I dispatch board through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "board".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data summary matches the CLI's JSON summary against the same on-disk state
    let (code, stdout, _stderr) = run_board(ws.path(), &["--format", "json"]);
    assert_eq!(code, 0);
    let cli_v: serde_json::Value = serde_json::from_str(&stdout).expect("CLI stdout is JSON");
    assert_eq!(cli_v["summary"], dispatcher_data["summary"]);

    // @step Then the CLI bridge module rust/fspec/src/board.rs contains NO inline column-building, point-summing, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/board.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/board.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "points in progress",
        "points completed",
        "completedPoints",
        "inProgressPoints",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic)"
        );
    }
}
