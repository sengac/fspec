//! CLI surface for the `add-domain-event` subcommand on the standalone fspec
//! Rust binary — RPC-179.
//!
//! Feature: spec/features/add-domain-event-cli-subcommand.feature
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

fn run_add_domain_event(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-domain-event");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-domain-event");
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

fn read_work_units(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_unit(id: &str, status: &str) -> String {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<serde_json::Value> = if *st == status {
            vec![serde_json::Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), serde_json::Value::Array(arr));
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

/// Seed a unit already carrying a single non-deleted event at id 0.
fn seed_unit_with_event(id: &str, status: &str, text: &str) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&seed_unit(id, status)).unwrap();
    v["workUnits"][id]["eventStorm"] = serde_json::json!({
        "level": "process_modeling",
        "items": [{
            "id": 0, "type": "event", "color": "orange",
            "text": text, "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    serde_json::to_string_pretty(&v).unwrap()
}

const TS_HELP_FIXTURE_ADE: &str = include_str!("fixtures/help/add-domain-event.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_domain_event_help_matches_ts_fixture() {
    // @step given the fspec Rust binary is built and on PATH

    // @step when I run `fspec add-domain-event --help`
    let output = Command::new(fspec_bin())
        .arg("add-domain-event")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-domain-event --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step then the exit code is 0
    assert_eq!(
        code, 0,
        "add-domain-event --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-domain-event.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_ADE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI appends a domain event and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_appends_domain_event_and_prints_success_line() {
    // @step given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("RPC-179", "specifying"));

    // @step when I run `fspec add-domain-event RPC-179 "UserRegistered"` in that tempdir
    let (code, stdout, stderr) = run_add_domain_event(ws.path(), &["RPC-179", "UserRegistered"]);

    // @step then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added domain event "UserRegistered" to RPC-179 (ID: 0)'
    assert!(
        stdout.contains("✓ Added domain event \"UserRegistered\" to RPC-179 (ID: 0)"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk shows RPC-179 eventStorm items has length 1
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["RPC-179"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a duplicate event with exit 1 and TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_duplicate_event_with_exit_1_and_error_prefix() {
    // @step given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying with a non-deleted event "UserRegistered" at id 0
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &seed_unit_with_event("RPC-179", "specifying", "UserRegistered"),
    );
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step when I run `fspec add-domain-event RPC-179 "UserRegistered"` in that tempdir
    let (code, _stdout, stderr) = run_add_domain_event(ws.path(), &["RPC-179", "UserRegistered"]);

    // @step then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add domain event:'
    assert!(
        stderr.contains("✗ Failed to add domain event:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Event 'UserRegistered' already exists (ID: 0)"
    assert!(
        stderr.contains("Event 'UserRegistered' already exists (ID: 0)"),
        "stderr must contain canonical dedup message; got:\n{stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("RPC-179", "specifying"));

    // @step when I dispatch add-domain-event via fspec_core::dispatch::dispatch_command with workUnitId='RPC-179' text='E1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-domain-event".to_string(),
        args_json: r#"{"workUnitId":"RPC-179","text":"E1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-domain-event RPC-179 "E2"` afterwards exits 0
    let (code, stdout, stderr) = run_add_domain_event(ws.path(), &["RPC-179", "E2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows RPC-179 eventStorm items has length 2
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["RPC-179"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 2);

    // @step And the CLI bridge module codelet/fspec/src/add_domain_event.rs contains NO inline event construction, dedup check, status guard, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_domain_event.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_domain_event.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The bridge IS allowed to print the success line (TS-parity: the success
    // `output.log(...)` lives in the action callback at src/commands/add-domain-event.ts,
    // NOT in addDomainEvent() which returns {success, eventId}). Every OTHER literal below
    // would constitute domain-logic duplication and is forbidden.
    for forbidden in [
        "eventStorm",
        "process_modeling",
        "already exists",
        "write_json_atomic",
        "Cannot add Event Storm items",
        "not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
