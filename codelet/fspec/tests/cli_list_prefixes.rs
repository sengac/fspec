//! CLI surface for the `list-prefixes` subcommand on the standalone fspec
//! Rust binary — RPC-248.
//!
//! Feature: spec/features/list-prefixes-cli-subcommand.feature
//!
//! Green phase: these tests exercise the wired-up clap subcommand
//! (`Mode::ListPrefixes` in `codelet/fspec/src/main.rs`) and the ported
//! `codelet/fspec-core/src/commands/list_prefixes.rs` implementation.
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

fn run_list_prefixes(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-prefixes");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-prefixes");
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

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn canonical_prefixes_json() -> String {
    r#"{
  "prefixes": {
    "AUTH": { "prefix": "AUTH", "description": "Auth features", "createdAt": "x" },
    "DASH": { "prefix": "DASH", "description": "Dashboard", "createdAt": "x" }
  }
}"#
    .to_string()
}

fn canonical_work_units_json() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "status": "done", "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "t", "status": "backlog", "createdAt": "x", "updatedAt": "x" },
    "DASH-001": { "id": "DASH-001", "title": "t", "status": "done", "createdAt": "x", "updatedAt": "x" },
    "DASH-002": { "id": "DASH-002", "title": "t", "status": "done", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [],
    "done": ["AUTH-001", "DASH-001", "DASH-002"], "blocked": []
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-prefixes as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_prefixes_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-prefixes --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-prefixes")
        .arg("--help")
        .output()
        .expect("spawn fspec list-prefixes --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-prefixes --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the list-prefixes subcommand
    assert!(
        stdout.contains("list-prefixes") || stdout.contains("List all prefixes"),
        "help must describe the list-prefixes subcommand; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "list-prefixes --help must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--prefix'
    assert!(
        !stdout.contains("--prefix"),
        "list-prefixes --help must NOT advertise --prefix; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--epic'
    assert!(
        !stdout.contains("--epic"),
        "list-prefixes --help must NOT advertise --epic; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-prefixes --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-prefixes --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory prints sentinel and does not auto-create files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_prints_sentinel_and_no_files() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-prefixes` from that directory
    let (code, stdout, stderr) = run_list_prefixes(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-prefixes must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'No prefixes found'
    assert!(
        stdout.contains("No prefixes found"),
        "stdout must contain 'No prefixes found'; got:\n{stdout}"
    );

    // @step Then spec/prefixes.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/prefixes.json").exists(),
        "list-prefixes must NOT auto-create spec/prefixes.json"
    );

    // @step Then spec/work-units.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/work-units.json").exists(),
        "list-prefixes must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders prefix progress for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_prefix_progress() {
    // @step Given spec/prefixes.json contains AUTH (description 'Auth features') and DASH (description 'Dashboard') in that order
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), &canonical_prefixes_json());

    // @step Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), DASH-001 (done), DASH-002 (done)
    write_work_units(ws.path(), &canonical_work_units_json());

    // @step When I run `./codelet/target/release/fspec list-prefixes`
    let (code, stdout, stderr) = run_list_prefixes(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-prefixes must exit 0 on the populated case; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Prefixes (2)'
    assert!(
        stdout.contains("Prefixes (2)"),
        "stdout must contain 'Prefixes (2)' header; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'AUTH'
    assert!(
        stdout.contains("AUTH"),
        "stdout must contain 'AUTH'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '  Auth features'
    assert!(
        stdout.contains("  Auth features"),
        "stdout must contain '  Auth features' description; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Work Units: 1/2 (50%)'
    assert!(
        stdout.lines().any(|l| l == "  Work Units: 1/2 (50%)"),
        "stdout must contain exact line '  Work Units: 1/2 (50%)'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'DASH'
    assert!(
        stdout.contains("DASH"),
        "stdout must contain 'DASH'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  Work Units: 2/2 (100%)'
    assert!(
        stdout.lines().any(|l| l == "  Work Units: 2/2 (100%)"),
        "stdout must contain exact line '  Work Units: 2/2 (100%)'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when prefixes.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_prefixes_json_exits_1_with_stderr() {
    // @step Given spec/prefixes.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), "{ this is not valid json");

    // @step When I run `./codelet/target/release/fspec list-prefixes`
    let (code, stdout, stderr) = run_list_prefixes(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-prefixes must exit 1 on malformed prefixes.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse prefixes.json'
    assert!(
        stderr.contains("Failed to parse prefixes.json"),
        "stderr must contain 'Failed to parse prefixes.json'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 0 when work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_json_exits_0_silently() {
    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(
        ws.path(),
        r#"{
  "prefixes": {
    "AUTH": { "prefix": "AUTH", "description": "Auth features", "createdAt": "x" }
  }
}"#,
    );

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./codelet/target/release/fspec list-prefixes`
    let (code, stdout, stderr) = run_list_prefixes(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-prefixes must exit 0 when work-units.json is malformed (TS silently swallows); got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'AUTH'
    assert!(
        stdout.contains("AUTH"),
        "stdout must list AUTH; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring 'Work Units:'
    assert!(
        !stdout.contains("Work Units:"),
        "stdout must NOT include 'Work Units:' line when work-units.json fails to parse; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_prefixes() {
    // @step Given the fspec Rust binary has list-prefixes registered as a clap subcommand alongside daemon, client, status, and list-work-units
    // (asserted by the help-listing check below)

    // @step When I run `./codelet/target/release/fspec --help`
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

    // @step Then the help output lists daemon, client, status, list-work-units, and list-prefixes as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
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
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/prefixes.json contains AUTH (description 'Auth features') and spec/work-units.json contains AUTH-001 (done) and AUTH-002 (backlog)
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(
        ws.path(),
        r#"{
  "prefixes": {
    "AUTH": { "prefix": "AUTH", "description": "Auth features", "createdAt": "x" }
  }
}"#,
    );
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "status": "done", "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "t", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [],
    "done": ["AUTH-001"], "blocked": []
  }
}"#,
    );

    // @step When I dispatch list-prefixes through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-prefixes".to_string(),
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
    let dispatcher_auth = &dispatcher_data["prefixes"]
        .as_array()
        .expect("prefixes array")[0];
    assert_eq!(dispatcher_auth["prefix"].as_str(), Some("AUTH"));
    assert_eq!(dispatcher_auth["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(dispatcher_auth["completedWorkUnits"].as_u64(), Some(1));
    assert_eq!(dispatcher_auth["completionPercentage"].as_u64(), Some(50));

    // @step Then the dispatcher's DispatchResult.data shows AUTH at 1/2 (50%) and the CLI text output (`fspec list-prefixes`) shows the exact line '  Work Units: 1/2 (50%)' against the same on-disk state
    // The CLI today does NOT expose --format (per rule [10]), so we assert
    // the text path mirrors the same underlying data. This validates the
    // shared-function delegation (rule [11]).
    let (code, stdout, _stderr) = run_list_prefixes(ws.path(), &[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("AUTH"));
    assert!(
        stdout.lines().any(|l| l == "  Work Units: 1/2 (50%)"),
        "CLI text output must reflect the same 1/2 (50%) progress as the dispatcher; got:\n{stdout}"
    );

    // @step Then the CLI bridge module codelet/fspec/src/list_prefixes.rs contains NO inline prefix-aggregation, filter, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_prefixes.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_prefixes.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // Forbid any inline aggregation/rendering markers — the bridge must only marshall args
    // and delegate. (Same anti-duplication guard used by RPC-253 in the
    // list_work_units bridge.)
    for forbidden in [
        "completionPercentage",
        "totalWorkUnits",
        "completedWorkUnits",
        "No prefixes found",
        "Prefixes (",
        "Work Units:",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-prefixes --help is byte-for-byte identical to TS (RPC-248)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_LP: &str = include_str!("fixtures/help/list-prefixes.txt");

#[test]
fn scenario_list_prefixes_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec list-prefixes --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-prefixes")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-prefixes --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "list-prefixes --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-prefixes.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_LP);

    // @step And stdout starts with a blank line followed by 'LIST-PREFIXES'
    assert!(stdout.starts_with("\nLIST-PREFIXES\n"));
}
