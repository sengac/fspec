//! CLI surface for the `validate-work-units` subcommand on the standalone
//! fspec Rust binary — RPC-325.
//!
//! Feature: spec/features/validate-work-units-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + core impl are not yet wired,
//! so these tests are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_validate_work_units(cwd: &Path) -> (i32, String, String) {
    let output = Command::new(fspec_bin())
        .arg("validate-work-units")
        .current_dir(cwd)
        .output()
        .expect("spawn fspec validate-work-units");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(root: &Path, value: &Value) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(value).unwrap(),
    )
    .expect("write work-units.json");
}

const TS_HELP_FIXTURE_VWU: &str = include_str!("fixtures/help/validate-work-units.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: validate-work-units --help is byte-for-byte identical to the TS reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_validate_work_units_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec validate-work-units --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("validate-work-units")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-work-units --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "validate-work-units --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/validate-work-units.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_VWU);

    // @step And stdout starts with a blank line followed by 'VALIDATE-WORK-UNITS'
    assert!(
        stdout.starts_with("\nVALIDATE-WORK-UNITS\n"),
        "got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints success and exits 0 for a clean store
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_clean_store_prints_success_exits_0() {
    // @step Given spec/work-units.json contains consistent work units with valid statuses and matching state arrays
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "done" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I run `./rust/target/release/fspec validate-work-units`
    let (code, stdout, stderr) = run_validate_work_units(ws.path());

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ All work units are valid'
    assert!(
        stdout.contains("✓ All work units are valid"),
        "got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints errors to stderr and exits 1 for a corrupt store
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_corrupt_store_prints_errors_to_stderr_exits_1() {
    // @step Given spec/work-units.json contains AUTH-002 with a non-existent parent AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I run `./rust/target/release/fspec validate-work-units`
    let (code, stdout, stderr) = run_validate_work_units(ws.path());

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stderr contains the substring '✗ Failed to validate work units'
    // TS parity: a missing parent triggers an uncaught TypeError
    // (`workUnitsData.workUnits[parent].children` on undefined), surfaced by
    // the catch block rather than the structured "non-existent parent" error.
    assert!(
        stderr.contains("✗ Failed to validate work units"),
        "got stderr:\n{stderr}"
    );

    // @step Then stderr contains the substring "Cannot read properties of undefined (reading 'children')"
    assert!(
        stderr.contains("Cannot read properties of undefined (reading 'children')"),
        "got stderr:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains an invalid status value
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "review" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch validate-work-units through fspec_core::dispatch::dispatch_command and also run `./rust/target/release/fspec validate-work-units` against the same on-disk state
    let req = codelet_fspec_core::DispatchRequest {
        command: "validate-work-units".to_string(),
        args_json: "{}".to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then both paths agree the store is invalid
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "dispatcher must report valid=false; got {data}"
    );
    let (code, _stdout, _stderr) = run_validate_work_units(ws.path());
    assert_eq!(code, 1, "CLI must also exit 1 for the same invalid state");

    // @step Then the CLI bridge module rust/fspec/src/validate_work_units.rs contains NO inline validation logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/validate_work_units.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/validate_work_units.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // NOTE: only VALIDATION-LOGIC strings are forbidden here. Presentation
    // strings (e.g. "✓ All work units are valid") may legitimately live in
    // the bridge if rendering is performed CLI-side — that is a PHASE C
    // design choice, not a duplication of validation logic.
    for forbidden in [
        "references non-existent parent",
        "Invalid status value",
        "State consistency error",
        "contains empty strings or non-strings",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects the documented-only --fix flag at runtime
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_documented_only_fix_flag() {
    // @step Given the --fix option appears in `validate-work-units --help` output (for byte-parity with the TS rich help)
    let help = Command::new(fspec_bin())
        .arg("validate-work-units")
        .arg("--help")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-work-units --help");
    let help_stdout = String::from_utf8_lossy(&help.stdout).into_owned();
    assert!(
        help_stdout.contains("--fix"),
        "--fix must remain documented in --help output; got:\n{help_stdout}"
    );

    // @step When I run `./rust/target/release/fspec validate-work-units --fix` against a clean store
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {},
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );
    let output = Command::new(fspec_bin())
        .arg("validate-work-units")
        .arg("--fix")
        .current_dir(ws.path())
        .output()
        .expect("spawn validate-work-units --fix");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "documented-only --fix must be rejected at runtime; stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring "error: unknown option '--fix'"
    assert!(
        stderr.contains("error: unknown option '--fix'"),
        "must report unknown option for --fix; got stderr:\n{stderr}"
    );

    // @step Then the matching TS command `fspec validate-work-units --fix` also exits 1 with the same 'unknown option' message
    // (Parity assertion documented here; the TS reference is exercised by the
    // out-of-band parity harness — see spec/attachments/RPC-003/parity-review-2026-06-14.md.)
}
