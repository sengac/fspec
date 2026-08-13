//! CLI surface for the `delete-epic` subcommand on the standalone fspec
//! Rust binary — RPC-217.
//!
//! Feature: spec/features/delete-epic-cli-subcommand.feature
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

fn run_delete_epic(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-epic");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-epic");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_spec_file(cwd: &Path, name: &str, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join(name), raw).expect("write spec file");
}

fn read_value(cwd: &Path, name: &str) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec").join(name)).expect("read spec file");
    serde_json::from_str(&raw).expect("parse JSON")
}

fn epics_with_auth() -> &'static str {
    r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Authentication",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

fn epics_with_dash() -> &'static str {
    r#"{
  "epics": {
    "dash": {
      "id": "dash",
      "title": "Dashboard",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

fn epics_with_auth_and_dash() -> &'static str {
    r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Authentication",
      "createdAt": "2026-06-01T00:00:00.000Z"
    },
    "dash": {
      "id": "dash",
      "title": "Dashboard",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes delete-epic with a positional arg and --force flag in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_delete_epic_with_arg_and_force_flag() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec delete-epic --help`
    let output = Command::new(fspec_bin())
        .arg("delete-epic")
        .arg("--help")
        .output()
        .expect("spawn delete-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-epic --help must exit 0; stderr={stderr}");

    // @step And stdout describes the delete-epic subcommand
    assert!(
        stdout.contains("delete-epic") || stdout.contains("DELETE-EPIC"),
        "help must describe delete-epic; got:\n{stdout}"
    );

    // @step And stdout mentions the `<epicId>` argument
    assert!(
        stdout.contains("epicId"),
        "help must mention epicId; got:\n{stdout}"
    );

    // @step And stdout advertises the `--force` flag
    assert!(
        stdout.contains("--force"),
        "help must advertise --force; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "delete-epic --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes an existing epic and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_existing_epic_and_prints_success_line() {
    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let ws = tempfile::tempdir().expect("tempdir");
    write_spec_file(ws.path(), "epics.json", epics_with_auth());

    // @step When I run `./rust/target/release/fspec delete-epic auth`
    let (code, stdout, stderr) = run_delete_epic(ws.path(), &["auth"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Epic auth deleted successfully'
    assert!(
        stdout
            .lines()
            .any(|l| l == "✓ Epic auth deleted successfully"),
        "missing success line; got:\n{stdout}"
    );

    // @step And the on-disk spec/epics.json no longer contains an 'auth' epic
    let data = read_value(ws.path(), "epics.json");
    assert!(data["epics"].get("auth").is_none());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI accepts --force without changing behaviour (TS impl ignores it)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_accepts_force_flag_without_changing_behaviour() {
    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let ws = tempfile::tempdir().expect("tempdir");
    write_spec_file(ws.path(), "epics.json", epics_with_auth());

    // @step When I run `./rust/target/release/fspec delete-epic auth --force`
    let (code, stdout, stderr) = run_delete_epic(ws.path(), &["auth", "--force"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Epic auth deleted successfully'
    assert!(
        stdout
            .lines()
            .any(|l| l == "✓ Epic auth deleted successfully"),
        "missing success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when the epic does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_epic_does_not_exist() {
    // @step Given spec/epics.json contains epic 'dash' with title='Dashboard'
    let ws = tempfile::tempdir().expect("tempdir");
    write_spec_file(ws.path(), "epics.json", epics_with_dash());

    // @step When I run `./rust/target/release/fspec delete-epic missing`
    let (code, stdout, stderr) = run_delete_epic(ws.path(), &["missing"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to delete epic:'
    // Parity with TS `src/commands/delete-epic.ts:106`:
    // `output.error('✗ Failed to delete epic:', error.message)`.
    assert!(
        stderr.contains("✗ Failed to delete epic:"),
        "stderr must contain TS-parity '✗ Failed to delete epic:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Epic missing not found'
    assert!(
        stderr.contains("Epic missing not found"),
        "stderr must report missing epic; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/epics.json contains epics 'auth' and 'dash'
    let ws = tempfile::tempdir().expect("tempdir");
    write_spec_file(ws.path(), "epics.json", epics_with_auth_and_dash());

    // @step When I dispatch delete-epic via fspec_core::dispatch::dispatch_command with epicId='auth'
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-epic".to_string(),
        args_json: r#"{"epicId":"auth"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `./rust/target/release/fspec delete-epic dash` afterwards exits 0
    let (code, stdout, stderr) = run_delete_epic(ws.path(), &["dash"]);
    assert_eq!(
        code, 0,
        "CLI delete must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/epics.json contains neither 'auth' nor 'dash' epics
    let data = read_value(ws.path(), "epics.json");
    assert!(data["epics"].get("auth").is_none(), "auth must be gone");
    assert!(data["epics"].get("dash").is_none(), "dash must be gone");

    // @step And the CLI bridge module rust/fspec/src/delete_epic.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/delete_epic.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/delete_epic.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The forbidden tokens here are domain-logic markers — keywords that
    // would only appear in a bridge that duplicated the core's epic
    // lookup, write, or rendering. The TS-parity stderr prefix
    // `✗ Failed to delete epic:` is literally what TS Commander.js
    // prints and IS part of the CLI surface, so it MUST live in the
    // bridge; we don't forbid it here. The dispatch path is enforced
    // by the `read_epics_or_empty` / `write_json_atomic` checks below.
    for forbidden in [
        "not found",
        "✓ Epic",
        "deleted successfully",
        "write_json_atomic",
        "read_epics_or_empty",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: delete-epic --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_DE: &str = include_str!("fixtures/help/delete-epic.txt");

#[test]
fn scenario_delete_epic_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec delete-epic --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("delete-epic")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-epic --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/delete-epic.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_DE);

    // @step And stdout starts with a blank line followed by 'DELETE-EPIC'
    assert!(stdout.starts_with("\nDELETE-EPIC\n"));
}
