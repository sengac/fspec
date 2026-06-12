//! CLI surface for the `delete-work-unit` subcommand on the standalone fspec
//! Rust binary — RPC-223.
//!
//! Feature: spec/features/delete-work-unit-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_delete_work_unit(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-work-unit");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-work-unit");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse JSON")
}

fn wu_leaf_auth_001() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
}

fn wu_auth_and_dash() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "createdAt": "x", "updatedAt": "x"
    },
    "DASH-001": {
      "id": "DASH-001", "title": "Dash", "status": "backlog",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "DASH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes delete-work-unit with a positional arg and flags in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_delete_work_unit_with_arg_and_flags() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec delete-work-unit --help`
    let output = Command::new(fspec_bin())
        .arg("delete-work-unit")
        .arg("--help")
        .output()
        .expect("spawn delete-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-work-unit --help must exit 0; stderr={stderr}");

    // @step And stdout describes the delete-work-unit subcommand
    assert!(
        stdout.contains("delete-work-unit") || stdout.contains("DELETE-WORK-UNIT"),
        "help must describe delete-work-unit; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout advertises the `--cascade-dependencies` flag
    assert!(
        stdout.contains("--cascade-dependencies"),
        "help must advertise --cascade-dependencies; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "delete-work-unit --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes an existing leaf work unit and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_existing_leaf_work_unit_and_prints_success_line() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wu_leaf_auth_001());

    // @step When I run `./codelet/target/release/fspec delete-work-unit AUTH-001`
    let (code, stdout, stderr) = run_delete_work_unit(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Work unit AUTH-001 deleted successfully'
    assert!(
        stdout.lines().any(|l| l == "✓ Work unit AUTH-001 deleted successfully"),
        "missing success line; got:\n{stdout}"
    );

    // @step And the on-disk spec/work-units.json no longer contains the AUTH-001 work unit
    let data = read_work_units(ws.path());
    assert!(data["workUnits"].get("AUTH-001").is_none());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_work_unit_does_not_exist() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wu_leaf_auth_001());

    // @step When I run `./codelet/target/release/fspec delete-work-unit MISSING-999`
    let (code, stdout, stderr) = run_delete_work_unit(ws.path(), &["MISSING-999"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to delete work unit:'
    assert!(
        stderr.contains("✗ Failed to delete work unit:"),
        "stderr must contain TS-parity prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit 'MISSING-999' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-999' does not exist"),
        "stderr must report missing work unit; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI cascades dependencies and prints a blocks warning
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_cascades_dependencies_and_prints_blocks_warning() {
    // @step Given spec/work-units.json contains work unit AUTH-001 with blocks API-001 and work unit API-001 with blockedBy AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "blocks": ["API-001"], "createdAt": "x", "updatedAt": "x"
    },
    "API-001": {
      "id": "API-001", "title": "API", "status": "backlog",
      "blockedBy": ["AUTH-001"], "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "API-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I run `./codelet/target/release/fspec delete-work-unit AUTH-001 --cascade-dependencies`
    let (code, stdout, stderr) =
        run_delete_work_unit(ws.path(), &["AUTH-001", "--cascade-dependencies"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Work unit AUTH-001 deleted successfully'
    assert!(
        stdout.lines().any(|l| l == "✓ Work unit AUTH-001 deleted successfully"),
        "missing success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '⚠ This work unit blocks 1 work unit(s): API-001'
    assert!(
        stdout.contains("⚠ This work unit blocks 1 work unit(s): API-001"),
        "missing blocks warning; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/work-units.json contains work unit AUTH-001 and work unit DASH-001 each with status='backlog' and no dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wu_auth_and_dash());

    // @step When I dispatch delete-work-unit via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-work-unit".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step And running `./codelet/target/release/fspec delete-work-unit DASH-001` afterwards exits 0
    let (code, stdout, stderr) = run_delete_work_unit(ws.path(), &["DASH-001"]);
    assert_eq!(code, 0, "CLI delete must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/work-units.json contains neither AUTH-001 nor DASH-001 work units
    let data = read_work_units(ws.path());
    assert!(data["workUnits"].get("AUTH-001").is_none(), "AUTH-001 must be gone");
    assert!(data["workUnits"].get("DASH-001").is_none(), "DASH-001 must be gone");

    // @step And the CLI bridge module codelet/fspec/src/delete_work_unit.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/delete_work_unit.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/delete_work_unit.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "does not exist",
        "deleted successfully",
        "write_json_atomic",
        "ensure_work_units_file",
        "cascadeDependencies\"]",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: delete-work-unit --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_DWU: &str = include_str!("fixtures/help/delete-work-unit.txt");

#[test]
fn scenario_delete_work_unit_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec delete-work-unit --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("delete-work-unit")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-work-unit --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/delete-work-unit.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_DWU);

    // @step And stdout starts with a blank line followed by 'DELETE-WORK-UNIT'
    assert!(stdout.starts_with("\nDELETE-WORK-UNIT\n"));
}
