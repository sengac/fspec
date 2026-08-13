//! CLI surface for the `repair-work-units` subcommand on the standalone
//! fspec Rust binary — RPC-284.
//!
//! Feature: spec/features/port-repair-work-units-command-to-rust.feature
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

fn run_repair(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("repair-work-units");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec repair-work-units");
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

/// A work-units.json where AUTH-001 has status specifying but is listed
/// only in states.testing — exactly one repairable issue.
const CORRUPTED_ONE_ISSUE: &str = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Auth", "status": "specifying", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": [], "specifying": [], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/repair-work-units.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dry_run_still_writes_rebuilt_states() {
    // Scenario: Dry-run still writes the rebuilt states

    // @step Given AUTH-001 has status specifying but is listed only in states.testing
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), CORRUPTED_ONE_ISSUE);

    // @step When I run `fspec repair-work-units --dry-run`
    let (code, stdout, stderr) = run_repair(ws.path(), &["--dry-run"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains "✓ Repaired 1 issues"
    assert!(
        stdout.contains("✓ Repaired 1 issues"),
        "stdout must contain the repaired-count line; got:\n{stdout}"
    );

    // @step And states.specifying contains AUTH-001 on disk
    assert!(
        read_states(ws.path(), "specifying").contains(&"AUTH-001".to_string()),
        "dry-run must still write the rebuilt states (TS no-op flag parity)"
    );
}

#[test]
fn scenario_cli_repairs_corrupted_file_and_reports_count() {
    // Scenario: CLI repairs a corrupted file and reports the count

    // @step Given AUTH-001 has status specifying but is listed only in states.testing
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), CORRUPTED_ONE_ISSUE);

    // @step When I run `fspec repair-work-units`
    let (code, stdout, stderr) = run_repair(ws.path(), &[]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains "✓ Repaired 1 issues"
    assert!(
        stdout.contains("✓ Repaired 1 issues"),
        "stdout must contain the repaired-count line; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_delegates_to_fspec_core_function() {
    // Scenario: CLI delegates to the same fspec_core function as the dispatcher

    // @step Given the rust/fspec crate is built

    // @step When I inspect rust/fspec/src/repair_work_units.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repair_work_units.rs");
    let src = fs::read_to_string(&path).expect("read repair_work_units.rs bridge");

    // @step Then the source declares it calls codelet_fspec_core::commands::repair_work_units::run
    assert!(
        src.contains("codelet_fspec_core::commands::repair_work_units")
            || src.contains("repair_work_units::run")
            || src.contains("core::run"),
        "bridge must delegate to fspec_core::commands::repair_work_units::run; got:\n{src}"
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

    // @step Given the TS help fixture at rust/fspec/tests/fixtures/help/repair-work-units.txt
    // (asserted by the include_str! above — the const TS_HELP_FIXTURE)

    // @step When I run `fspec repair-work-units --help`
    let output = Command::new(fspec_bin())
        .arg("repair-work-units")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec repair-work-units --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "repair-work-units --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured TS fixture byte-for-byte
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
