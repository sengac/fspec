//! CLI surface for the `show-epic` subcommand on the standalone fspec
//! Rust binary — RPC-302.
//!
//! Feature: spec/features/show-epic-cli-subcommand.feature
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

fn run_show_epic(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-epic");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-epic");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_epics(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn auth_epics_json() -> String {
    r#"{
  "epics": {
    "auth": { "id": "auth", "title": "Authentication", "description": "Login features", "createdAt": "x" }
  }
}"#
    .to_string()
}

fn auth_epics_no_desc_json() -> String {
    r#"{
  "epics": {
    "auth": { "id": "auth", "title": "Authentication", "createdAt": "x" }
  }
}"#
    .to_string()
}

fn auth_work_units_json() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "epic": "auth", "status": "done", "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "t", "epic": "auth", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [],
    "done": ["AUTH-001"], "blocked": []
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes show-epic as a subcommand and prints epicId-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_show_epic_with_epicid_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-epic --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-epic")
        .arg("--help")
        .output()
        .expect("spawn fspec show-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-epic --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the show-epic subcommand
    assert!(
        stdout.contains("show-epic") || stdout.contains("Display details of an epic"),
        "help must describe the show-epic subcommand; got:\n{stdout}"
    );

    // @step Then stdout advertises the required positional <epicId> argument
    assert!(
        stdout.contains("epicId")
            || stdout.contains("epic_id")
            || stdout.contains("EPIC_ID")
            || stdout.contains("<EPICID>")
            || stdout.contains("<EPIC_ID>"),
        "help must mention the positional epicId argument; got:\n{stdout}"
    );

    // @step Then stdout does NOT advertise the '--format' flag (TS show-epic-help.ts omits it from OPTIONS)
    // The `--format` flag is still registered on the clap variant for runtime
    // compatibility — we simply don't document it in the help block, matching
    // the TS Commander.js help reference byte-for-byte.
    assert!(
        !stdout.contains("--format"),
        "help must NOT advertise '--format' (TS parity); got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "show-epic --help must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--prefix'
    assert!(
        !stdout.contains("--prefix"),
        "show-epic --help must NOT advertise --prefix; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--epic'
    assert!(
        !stdout.contains("--epic\n") && !stdout.contains("--epic "),
        "show-epic --help must NOT advertise --epic flag; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "show-epic --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty workspace exits 1 with Epic not found error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_workspace_exits_1() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec show-epic auth` from that directory
    let (code, _stdout, stderr) = run_show_epic(ws.path(), &["auth"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec show-epic must exit 1 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Epic auth not found'
    assert!(
        stderr.contains("Epic auth not found"),
        "stderr must contain 'Epic auth not found'; got:\n{stderr}"
    );

    // @step Then spec/epics.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/epics.json").exists(),
        "show-epic must NOT auto-create spec/epics.json"
    );

    // @step Then spec/work-units.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/work-units.json").exists(),
        "show-epic must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders epic header and progress for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_epic_header_and_progress() {
    // @step Given spec/epics.json contains auth (title 'Authentication', description 'Login features')
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_json());

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    write_work_units(ws.path(), &auth_work_units_json());

    // @step When I run `./rust/target/release/fspec show-epic auth`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["auth"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-epic must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the line 'Epic: auth'
    assert!(
        stdout.lines().any(|l| l == "Epic: auth"),
        "stdout must contain exact line 'Epic: auth'; got:\n{stdout}"
    );

    // @step Then stdout contains the line 'Title: Authentication'
    assert!(
        stdout.lines().any(|l| l == "Title: Authentication"),
        "stdout must contain exact line 'Title: Authentication'; got:\n{stdout}"
    );

    // @step Then stdout contains the line 'Description: Login features'
    assert!(
        stdout.lines().any(|l| l == "Description: Login features"),
        "stdout must contain exact line 'Description: Login features'; got:\n{stdout}"
    );

    // @step Then stdout contains the line 'Progress:'
    assert!(
        stdout.lines().any(|l| l == "Progress:"),
        "stdout must contain exact line 'Progress:'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Total work units: 2'
    assert!(
        stdout.lines().any(|l| l == "  Total work units: 2"),
        "stdout must contain exact line '  Total work units: 2'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Completed: 1'
    assert!(
        stdout.lines().any(|l| l == "  Completed: 1"),
        "stdout must contain exact line '  Completed: 1'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Completion: 50%'
    assert!(
        stdout.lines().any(|l| l == "  Completion: 50%"),
        "stdout must contain exact line '  Completion: 50%'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when epics.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_epics_json_exits_1() {
    // @step Given spec/epics.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), "{ this is not valid json");

    // @step When I run `./rust/target/release/fspec show-epic auth`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["auth"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on malformed epics.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse epics.json'
    assert!(
        stderr.contains("Failed to parse epics.json"),
        "stderr must contain 'Failed to parse epics.json'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when epicId is not registered
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_unregistered_epic_id_exits_1() {
    // @step Given spec/epics.json contains auth (title 'Authentication')
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_json());

    // @step When I run `./rust/target/release/fspec show-epic nonexistent`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["nonexistent"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on unregistered epicId; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Epic nonexistent not found'
    assert!(
        stderr.contains("Epic nonexistent not found"),
        "stderr must contain 'Epic nonexistent not found'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 0 with text output when work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_json_exits_0() {
    // @step Given spec/epics.json contains auth (title 'Authentication')
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_no_desc_json());

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec show-epic auth`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["auth"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0 when work-units.json is malformed (TS bare catch); got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the line 'Epic: auth'
    assert!(
        stdout.lines().any(|l| l == "Epic: auth"),
        "stdout must contain 'Epic: auth' line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Total work units: 0'
    assert!(
        stdout.lines().any(|l| l == "  Total work units: 0"),
        "stdout must contain '  Total work units: 0'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Completion: 0%'
    assert!(
        stdout.lines().any(|l| l == "  Completion: 0%"),
        "stdout must contain '  Completion: 0%'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --format json emits JSON to stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_format_json_emits_json() {
    // @step Given spec/epics.json contains auth (title 'Authentication')
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_no_desc_json());

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    write_work_units(ws.path(), &auth_work_units_json());

    // @step When I run `./rust/target/release/fspec show-epic auth --format json`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["auth", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout parses as JSON whose root object has an 'epic' key
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(
        parsed["epic"].is_object(),
        "root.epic must be an object; got:\n{stdout}"
    );

    // @step Then the parsed JSON has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    assert_eq!(parsed["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(parsed["completedWorkUnits"].as_u64(), Some(1));
    let pct = parsed["completionPercentage"].as_f64().expect("f64");
    assert!((pct - 50.0).abs() < 1e-9, "expected 50; got {pct}");

    // @step Then the parsed JSON epic.id equals 'auth'
    assert_eq!(parsed["epic"]["id"].as_str(), Some("auth"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI -f json short flag matches the long form
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_f_short_flag_matches_long_form() {
    // @step Given spec/epics.json contains auth (title 'Authentication')
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_no_desc_json());

    // @step Given spec/work-units.json does NOT exist
    assert!(!ws.path().join("spec/work-units.json").exists());

    // @step When I run `./rust/target/release/fspec show-epic auth -f json`
    let (code, stdout, stderr) = run_show_epic(ws.path(), &["auth", "-f", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout parses as JSON with an 'epic' key
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(
        parsed["epic"].is_object(),
        "root.epic must be an object; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved() {
    // @step Given the fspec Rust binary has show-epic registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-epics

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

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, list-epics, and show-epic as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-epics",
        "show-epic",
    ] {
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
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root whose spec/epics.json contains auth (title 'Authentication') and spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(ws.path(), &auth_epics_no_desc_json());
    write_work_units(ws.path(), &auth_work_units_json());

    // @step When I dispatch show-epic through fspec_core::dispatch::dispatch_command with epicId='auth' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-epic".to_string(),
        args_json: r#"{"epicId":"auth","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data parses to a structure with totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    assert_eq!(parsed["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(parsed["completedWorkUnits"].as_u64(), Some(1));
    let pct = parsed["completionPercentage"].as_f64().expect("f64");
    assert!((pct - 50.0).abs() < 1e-9, "expected 50; got {pct}");

    // @step Then the CLI text output (./fspec show-epic auth) reflects the same '  Completion: 50%' line
    let (code, stdout, _stderr) = run_show_epic(ws.path(), &["auth"]);
    assert_eq!(code, 0);
    assert!(
        stdout.lines().any(|l| l == "  Completion: 50%"),
        "CLI text output must reflect the same 50% progress; got:\n{stdout}"
    );

    // @step Then the CLI bridge module rust/fspec/src/show_epic.rs contains NO inline aggregation, filter, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_epic.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/show_epic.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "completionPercentage",
        "totalWorkUnits",
        "completedWorkUnits",
        "Total work units:",
        "Completion:",
        "Title:",
        "Description:",
        "Epic not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-epic --help is byte-for-byte identical to TS (RPC-302)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_SE: &str = include_str!("fixtures/help/show-epic.txt");

#[test]
fn scenario_show_epic_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-epic --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-epic")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "show-epic --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-epic.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SE);

    // @step And stdout starts with a blank line followed by 'SHOW-EPIC'
    assert!(stdout.starts_with("\nSHOW-EPIC\n"));
}
