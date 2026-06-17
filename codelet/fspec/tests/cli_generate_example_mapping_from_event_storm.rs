//! CLI surface for the `generate-example-mapping-from-event-storm` subcommand
//! on the standalone fspec Rust binary — RPC-232.
//!
//! Feature: spec/features/generate-example-mapping-from-event-storm-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim. Until Phase C wiring
//! (clap variant + main.rs help intercept + dispatcher), these tests fail —
//! the intended red phase.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_generate(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("generate-example-mapping-from-event-storm");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec generate-example-mapping-from-event-storm");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn write_value(project_root: &Path, v: &serde_json::Value) {
    write_work_units(project_root, &serde_json::to_string_pretty(v).unwrap());
}

/// Seed a specifying work unit carrying the supplied eventStorm items.
fn seed_unit_with_event_storm(id: &str, items: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            id: {
                "id": id, "title": "title", "type": "story", "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z",
                "eventStorm": { "level": "process_modeling", "items": items, "nextItemId": 0 }
            }
        },
        "states": {
            "backlog": [], "specifying": [id], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

fn seed_unit_no_event_storm(id: &str) -> serde_json::Value {
    serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id, "title": "title", "type": "story", "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [id], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

const TS_HELP_FIXTURE_GEM: &str =
    include_str!("fixtures/help/generate-example-mapping-from-event-storm.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes the subcommand and prints flag-free --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_generate_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec generate-example-mapping-from-event-storm --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("generate-example-mapping-from-event-storm")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn generate-example-mapping-from-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/generate-example-mapping-from-event-storm.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_GEM);

    // @step Then stdout advertises the required positional <work-unit-id> argument
    assert!(
        stdout.contains("work-unit-id") || stdout.contains("workUnitId"),
        "help must advertise the positional argument; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI transforms Event Storm artifacts and prints the summary on success
//
// TS Console-output contract: the `.action()` callback calls
// `logger.success(...)` (Winston has no `success` level → TypeError swallowed
// by try/catch → `logger.error(...)` (file-only) → `process.exit(1)`). The TS
// binary therefore writes NOTHING to stdout/stderr and ALWAYS exits 1, while
// the work unit IS mutated + persisted before the throw. We assert byte-parity:
// exit 1, empty stdout/stderr, plus the persisted Example Mapping entries.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_transforms_and_prints_summary() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm of 1 policy and 1 hotspot in the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    let items = serde_json::json!([
        {
            "id": 0, "type": "policy", "color": "purple", "text": "Send welcome email",
            "when": "UserRegistered", "then": "SendWelcomeEmail", "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        },
        {
            "id": 1, "type": "hotspot", "color": "red", "text": "Email timeout",
            "concern": "How long to wait", "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }
    ]);
    write_value(ws.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I run `./codelet/target/release/fspec generate-example-mapping-from-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_generate(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 0
    // TS Console-output contract: always exits 1 with empty stdout/stderr
    // (the swallowed `logger.success` TypeError). Assert byte-parity.
    assert_eq!(code, 1, "expected exit 1 (TS logger.success TypeError); stderr={stderr}");
    assert_eq!(stdout, "", "stdout must be empty (file-only logger); got:\n{stdout}");
    assert_eq!(stderr, "", "stderr must be empty (file-only logger); got:\n{stderr}");

    // @step Then stdout contains 'Rules added: 1'
    // @step Then stdout contains 'Examples added: 0'
    // @step Then stdout contains 'Questions added: 1'
    // The summary is never printed (file-only logger); instead verify the work
    // unit was mutated + persisted (1 rule, 0 examples, 1 question).
    let raw = fs::read_to_string(ws.path().join("spec/work-units.json")).expect("read");
    let data: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let wu = &data["workUnits"]["AUTH-001"];
    assert_eq!(wu["rules"].as_array().map(Vec::len), Some(1), "1 rule persisted");
    assert_eq!(wu["examples"].as_array().map(Vec::len), Some(0), "0 examples persisted");
    assert_eq!(wu["questions"].as_array().map(Vec::len), Some(1), "1 question persisted");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty workspace exits 1 with missing-file error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_empty_workspace_exits_1_missing_file() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/work-units.json").exists());

    // @step When I run `./codelet/target/release/fspec generate-example-mapping-from-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_generate(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'Error:'
    // @step Then stderr contains the substring 'spec/work-units.json not found. Run fspec init first.'
    // TS Console-output contract: the failure path uses the file-only
    // `logger.error(...)`, so the binary writes NOTHING to stdout/stderr and
    // exits 1. Assert byte-parity.
    assert_eq!(stdout, "", "stdout must be empty (file-only logger); got:\n{stdout}");
    assert_eq!(stderr, "", "stderr must be empty (file-only logger); got:\n{stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when the unit has no Event Storm data
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_no_event_storm_data() {
    // @step Given spec/work-units.json contains AUTH-001 with no eventStorm field in the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_value(ws.path(), &seed_unit_no_event_storm("AUTH-001"));

    // @step When I run `./codelet/target/release/fspec generate-example-mapping-from-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_generate(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'has no Event Storm data'
    // TS Console-output contract: the failure path uses the file-only
    // `logger.error(...)`, so the binary writes NOTHING to stdout/stderr and
    // exits 1. Assert byte-parity.
    assert_eq!(stdout, "", "stdout must be empty (file-only logger); got:\n{stdout}");
    assert_eq!(stderr, "", "stderr must be empty (file-only logger); got:\n{stderr}");
}
