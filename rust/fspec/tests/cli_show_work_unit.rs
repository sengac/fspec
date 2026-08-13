//! CLI surface for the `show-work-unit` subcommand on the standalone fspec
//! Rust binary — RPC-308.
//!
//! Feature: spec/features/show-work-unit-cli-subcommand.feature
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

fn run_show_work_unit(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-work-unit");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.env_remove("FSPEC_DISABLE_REMINDERS");
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-work-unit");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_minimal_work_unit(cwd: &Path, id: &str, title: &str, status: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let all_states = [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ];
    let mut state_pairs = Vec::new();
    for st in &all_states {
        if *st == status {
            state_pairs.push(format!(r#""{st}":["{id}"]"#));
        } else {
            state_pairs.push(format!(r#""{st}":[]"#));
        }
    }
    let state_list = state_pairs.join(",");
    let json = format!(
        r#"{{
  "version":"0.7.1",
  "workUnits":{{"{id}":{{"id":"{id}","title":"{title}","type":"story","status":"{status}","createdAt":"2025-01-01T00:00:00.000Z","updatedAt":"2025-01-02T00:00:00.000Z"}}}},
  "states":{{ {state_list} }}
}}"#
    );
    fs::write(spec.join("work-units.json"), json).expect("write work-units.json");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes show-work-unit as a subcommand with positional workUnitId and a format option
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_show_work_unit_with_positional_and_format_option() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec show-work-unit --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-work-unit")
        .arg("--help")
        .output()
        .expect("spawn fspec show-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "show-work-unit --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains help describing the show-work-unit subcommand
    assert!(
        stdout.contains("show-work-unit")
            || stdout.to_lowercase().contains("work unit")
            || stdout.to_lowercase().contains("workunit"),
        "help must describe the show-work-unit subcommand; got:\n{stdout}"
    );

    // @step Then stdout mentions the workUnitId positional argument
    assert!(
        stdout.contains("workUnitId")
            || stdout.contains("WORKUNITID")
            || stdout.contains("work_unit_id")
            || stdout.contains("WORK_UNIT_ID"),
        "help must mention the workUnitId positional argument; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints text-format dump when the work unit exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_text_format_dump_when_work_unit_exists() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step Given spec/work-units.json contains AUTH-001 with title='Login', status='backlog', no rules
    write_minimal_work_unit(ws.path(), "AUTH-001", "Login", "backlog");

    // @step When I run `./rust/target/release/fspec show-work-unit AUTH-001` from that directory
    let (code, stdout, stderr) = run_show_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "show-work-unit must exit 0 for existing unit; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'AUTH-001'
    assert!(
        stdout.contains("AUTH-001"),
        "stdout must contain 'AUTH-001'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Type: story'
    assert!(
        stdout.contains("Type: story"),
        "stdout must contain 'Type: story'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Status: backlog'
    assert!(
        stdout.contains("Status: backlog"),
        "stdout must contain 'Status: backlog'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Login'
    assert!(
        stdout.contains("Login"),
        "stdout must contain 'Login'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints JSON payload when --format json is supplied
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_json_payload_when_format_json() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step Given spec/work-units.json contains AUTH-001 with title='Login', status='backlog'
    write_minimal_work_unit(ws.path(), "AUTH-001", "Login", "backlog");

    // @step When I run `./rust/target/release/fspec show-work-unit AUTH-001 --format json`
    let (code, stdout, stderr) = run_show_work_unit(ws.path(), &["AUTH-001", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "show-work-unit --format json must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout parses as JSON with id='AUTH-001', title='Login', type='story', status='backlog'
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {e}\n{stdout}"));
    assert_eq!(parsed["id"].as_str(), Some("AUTH-001"));
    assert_eq!(parsed["title"].as_str(), Some("Login"));
    assert_eq!(parsed["type"].as_str(), Some("story"));
    assert_eq!(parsed["status"].as_str(), Some("backlog"));

    // @step Then stdout uses 2-space indentation
    assert!(
        stdout.lines().any(|l| l.starts_with("  \"id\"")
            || l.starts_with("  \"title\"")
            || l.starts_with("  \"status\"")),
        "expected a 2-space-indented root field; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes the canonical message to stderr when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_unknown_work_unit_exits_1_with_canonical_stderr() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step Given spec/work-units.json contains AUTH-001 (any minimal shape) but NOT UNKNOWN-999
    write_minimal_work_unit(ws.path(), "AUTH-001", "t", "backlog");

    // @step When I run `./rust/target/release/fspec show-work-unit UNKNOWN-999`
    let (code, stdout, stderr) = run_show_work_unit(ws.path(), &["UNKNOWN-999"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "show-work-unit must exit 1 for unknown unit; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring "Work unit 'UNKNOWN-999' does not exist"
    assert!(
        stderr.contains("Work unit 'UNKNOWN-999' does not exist"),
        "stderr must contain canonical missing-work-unit message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when spec/work-units.json is absent (no auto-create)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_no_auto_create_when_work_units_json_absent() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec show-work-unit AUTH-001`
    let (code, stdout, stderr) = run_show_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 when work-units.json absent; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then spec/work-units.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/work-units.json").exists(),
        "show-work-unit must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_show_work_unit() {
    // @step Given the fspec Rust binary has show-work-unit registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and show-deleted

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, show-deleted, and show-work-unit as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "show-deleted",
        "show-work-unit",
    ] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined") || help.contains("Combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with title='Shared', status='backlog'
    let ws = tempfile::tempdir().expect("tempdir");
    write_minimal_work_unit(ws.path(), "AUTH-001", "Shared", "backlog");

    // @step When I dispatch show-work-unit through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-work-unit".to_string(),
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

    // @step Then the dispatcher's DispatchResult.data shows id='AUTH-001' and title='Shared'
    assert_eq!(dispatcher_data["id"].as_str(), Some("AUTH-001"));
    assert_eq!(dispatcher_data["title"].as_str(), Some("Shared"));

    // @step Then the CLI text output `fspec show-work-unit AUTH-001` against the same on-disk state shows the substring 'AUTH-001' and the line 'Status: backlog'
    let (code, stdout, _stderr) = run_show_work_unit(ws.path(), &["AUTH-001"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("AUTH-001"),
        "CLI text must contain AUTH-001; got:\n{stdout}"
    );
    assert!(
        stdout.lines().any(|l| l.contains("Status: backlog")),
        "CLI text must contain 'Status: backlog' line; got:\n{stdout}"
    );

    // @step Then the CLI bridge module rust/fspec/src/show_work_unit.rs contains NO inline projection, reminder generation, or feature-scan logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_work_unit.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/show_work_unit.rs must exist as the CLI bridge module; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "linkedFeatures",
        "systemReminders",
        "systemReminder",
        "deletedRules",
        "architectureNotes",
        "extract_work_unit_tags",
        "gherkin::",
        "FSPEC_DISABLE_REMINDERS",
        "LARGE ESTIMATE WARNING",
        "Invalid question format",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-work-unit --help is byte-for-byte identical to TS formatCommandHelp reference output
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_SWU: &str = include_str!("fixtures/help/show-work-unit.txt");

#[test]
fn scenario_show_work_unit_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-work-unit --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-work-unit")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "show-work-unit --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-work-unit.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SWU);

    // @step And stdout starts with a blank line followed by 'SHOW-WORK-UNIT'
    assert!(
        stdout.starts_with("\nSHOW-WORK-UNIT\n"),
        "stdout must start with blank line + SHOW-WORK-UNIT; got:\n{stdout}"
    );
}
