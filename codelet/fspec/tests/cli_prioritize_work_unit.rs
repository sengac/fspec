//! CLI surface for the `prioritize-work-unit` subcommand on the standalone
//! fspec Rust binary — RPC-255.
//!
//! Feature: spec/features/port-prioritize-work-unit-command-to-rust.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim. At the end of Phase B the behavioural
//! scenarios fail because the dispatcher still routes to the NotYetPorted
//! stub; after Phase C + supervisor wiring they turn green.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_prioritize(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("prioritize-work-unit");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec prioritize-work-unit");
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

fn read_states(cwd: &Path, status: &str) -> Vec<String> {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    let on_disk: serde_json::Value = serde_json::from_str(&raw).expect("parse work-units.json");
    on_disk["states"][status]
        .as_array()
        .expect("states array present")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn wu(id: &str, status: &str) -> String {
    format!(
        r#""{id}": {{ "id": "{id}", "title": "{id}", "status": "{status}", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }}"#
    )
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/prioritize-work-unit.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_position_top_moves_to_front() {
    // Scenario: Position top moves a work unit to the front of its column

    // @step Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {}, {} }}, "states": {{ "backlog": ["AUTH-002","AUTH-003","AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "backlog"),
        wu("AUTH-003", "backlog")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position top`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["AUTH-001", "--position", "top"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And the backlog order becomes AUTH-001, AUTH-002, AUTH-003
    assert_eq!(
        read_states(ws.path(), "backlog"),
        vec!["AUTH-001", "AUTH-002", "AUTH-003"]
    );
}

#[test]
fn scenario_numeric_position_is_one_based() {
    // Scenario: Numeric position is 1-based

    // @step Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-004, AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {}, {}, {} }}, "states": {{ "backlog": ["AUTH-002","AUTH-003","AUTH-004","AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "backlog"),
        wu("AUTH-003", "backlog"),
        wu("AUTH-004", "backlog")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position 3`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["AUTH-001", "--position", "3"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And the backlog order becomes AUTH-002, AUTH-003, AUTH-001, AUTH-004
    assert_eq!(
        read_states(ws.path(), "backlog"),
        vec!["AUTH-002", "AUTH-003", "AUTH-001", "AUTH-004"]
    );
}

#[test]
fn scenario_reject_numeric_position_below_one() {
    // Scenario: Reject numeric position below 1

    // @step Given spec/work-units.json backlog contains AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position 0`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["AUTH-001", "--position", "0"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Invalid position: 0. Position must be >= 1 (1-based index)"
    assert!(
        stderr.contains("Invalid position: 0. Position must be >= 1 (1-based index)"),
        "stderr must mention the invalid-position message; got:\n{stderr}"
    );
}

#[test]
fn scenario_detect_work_unit_missing_from_states_array() {
    // Scenario: Detect work unit missing from its own states array

    // @step Given AUTH-001 has status specifying but is listed only in states.testing
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": [], "specifying": [], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "specifying")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position top`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["AUTH-001", "--position", "top"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Data integrity error"
    assert!(stderr.contains("Data integrity error"), "got:\n{stderr}");

    // @step And stderr contains "states.specifying"
    assert!(stderr.contains("states.specifying"), "got:\n{stderr}");

    // @step And stderr contains "fspec repair-work-units"
    assert!(stderr.contains("fspec repair-work-units"), "got:\n{stderr}");
}

#[test]
fn scenario_reject_cross_column_relative_placement() {
    // Scenario: Reject cross-column relative placement

    // @step Given FEAT-017 is in states.specifying and AUTH-001 is in states.testing
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {} }}, "states": {{ "backlog": [], "specifying": ["FEAT-017"], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("FEAT-017", "specifying"),
        wu("AUTH-001", "testing")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit FEAT-017 --before AUTH-001`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["FEAT-017", "--before", "AUTH-001"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Data integrity error"
    assert!(
        stderr.contains("Cannot prioritize across columns") || stderr.contains("Data integrity error"),
        "stderr must reject cross-column placement; got:\n{stderr}"
    );
}

#[test]
fn scenario_reject_non_existent_work_unit() {
    // Scenario: Reject prioritizing a non-existent work unit

    // @step Given spec/work-units.json does not contain MISSING-999
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog")
    );
    write_work_units(ws.path(), &body);
    let before = fs::read_to_string(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec prioritize-work-unit MISSING-999 --position top`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["MISSING-999", "--position", "top"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Work unit 'MISSING-999' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-999' does not exist"),
        "got:\n{stderr}"
    );

    // @step And spec/work-units.json is byte-identical to its pre-call content
    let after = fs::read_to_string(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(before, after, "file must be untouched on missing work unit");
}

#[test]
fn scenario_reject_done_work_unit() {
    // Scenario: Reject prioritizing a done work unit

    // @step Given DONE-001 has status done and is in states.done
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": [], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": ["DONE-001"], "blocked": [] }} }}"#,
        wu("DONE-001", "done")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit DONE-001 --position top`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["DONE-001", "--position", "top"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Cannot prioritize work units in done column"
    assert!(
        stderr.contains("Cannot prioritize work units in done column"),
        "got:\n{stderr}"
    );
}

#[test]
fn scenario_relative_placement_before_and_after() {
    // Scenario: Relative placement with before and after

    // @step Given spec/work-units.json implementing order is AUTH-002, AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {} }}, "states": {{ "backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-002","AUTH-001"], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "implementing"),
        wu("AUTH-002", "implementing")
    );
    write_work_units(ws.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --before AUTH-002`
    let (code, _stdout, stderr) = run_prioritize(ws.path(), &["AUTH-001", "--before", "AUTH-002"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And the implementing order becomes AUTH-001, AUTH-002
    assert_eq!(
        read_states(ws.path(), "implementing"),
        vec!["AUTH-001", "AUTH-002"]
    );
}

#[test]
fn scenario_cli_delegates_to_fspec_core_function() {
    // Scenario: CLI delegates to the same fspec_core function as the dispatcher

    // @step Given the codelet/fspec crate is built

    // @step When I inspect codelet/fspec/src/prioritize_work_unit.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/prioritize_work_unit.rs");
    let src = fs::read_to_string(&path).expect("read prioritize_work_unit.rs bridge");

    // @step Then the source declares it calls codelet_fspec_core::commands::prioritize_work_unit::run
    assert!(
        src.contains("codelet_fspec_core::commands::prioritize_work_unit")
            || src.contains("prioritize_work_unit::run")
            || src.contains("core::run"),
        "bridge must delegate to fspec_core::commands::prioritize_work_unit::run; got:\n{src}"
    );

    // @step And the source does NOT perform any file IO directly on spec/work-units.json
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "WorkUnitsData",
        "work-units.json",
        "spec/work-units",
    ] {
        assert!(
            !src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{src}"
        );
    }
}

#[test]
fn scenario_cli_help_matches_ts_fixture() {
    // Scenario: CLI help surface matches the captured TS fixture

    // @step Given the TS help fixture at codelet/fspec/tests/fixtures/help/prioritize-work-unit.txt
    // (asserted by the include_str! above — the const TS_HELP_FIXTURE)

    // @step When I run `fspec prioritize-work-unit --help`
    let output = Command::new(fspec_bin())
        .arg("prioritize-work-unit")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec prioritize-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "prioritize-work-unit --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured TS fixture byte-for-byte
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
