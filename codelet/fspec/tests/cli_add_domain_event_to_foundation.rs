//! CLI surface for the `add-domain-event-to-foundation` subcommand on the
//! standalone fspec Rust binary — RPC-180.
//!
//! Feature: spec/features/add-domain-event-to-foundation-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim. Twin:
//! codelet/fspec/tests/cli_add_command_to_foundation.rs (RPC-175).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-domain-event-to-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec add-domain-event-to-foundation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(project_root: &Path, value: &serde_json::Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn read_foundation(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec/foundation.json"))
        .expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

/// Foundation with a single bounded_context (id=0, 'Work Management') and
/// nextItemId=1.
fn foundation_with_work_management() -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {
            "level": "big_picture",
            "items": [{
                "id": 0,
                "type": "bounded_context",
                "text": "Work Management",
                "color": null,
                "deleted": false,
                "createdAt": "2026-06-01T00:00:00.000Z"
            }],
            "nextItemId": 1
        }
    })
}

fn first_event(foundation: &serde_json::Value) -> Option<serde_json::Value> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("event"))
        .cloned()
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-domain-event-to-foundation.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_domain_event_to_foundation_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-domain-event-to-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("add-domain-event-to-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-domain-event-to-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "add-domain-event-to-foundation --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-domain-event-to-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully appends a domain event and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_appends_domain_event() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_work_management());

    // @step When I run `fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["Work Management", "WorkUnitCreated"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added domain event "WorkUnitCreated" to "Work Management" bounded context'
    assert!(
        stdout.contains(
            "✓ Added domain event \"WorkUnitCreated\" to \"Work Management\" bounded context"
        ),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows eventStorm.items gained an event item with text='WorkUnitCreated' and boundedContextId=0
    let v = read_foundation(ws.path());
    let ev = first_event(&v).expect("an event item must exist");
    assert_eq!(ev["text"].as_str(), Some("WorkUnitCreated"));
    assert_eq!(ev["boundedContextId"].as_u64(), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI forwards the --description flag into the persisted item
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_forwards_description_flag() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_work_management());

    // @step When I run `fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated" --description "Signals work unit reached done status"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(
        ws.path(),
        &[
            "Work Management",
            "WorkUnitCreated",
            "--description",
            "Signals work unit reached done status",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And spec/foundation.json on disk shows the appended event item description='Signals work unit reached done status'
    let v = read_foundation(ws.path());
    let ev = first_event(&v).expect("an event item must exist");
    assert_eq!(
        ev["description"].as_str(),
        Some("Signals work unit reached done status")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a missing bounded context with exit 1 and the
//           TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_missing_bounded_context_with_exit_1() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_work_management());
    let pre_bytes = fs::read(ws.path().join("spec/foundation.json")).unwrap();

    // @step When I run `fspec add-domain-event-to-foundation "Nope" "WorkUnitCreated"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["Nope", "WorkUnitCreated"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error: prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Bounded context 'Nope' not found"
    assert!(
        stderr.contains("Bounded context 'Nope' not found"),
        "stderr must contain canonical missing-context message; got:\n{stderr}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_work_management());

    // @step When I dispatch add-domain-event-to-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' eventName='E1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-domain-event-to-foundation".to_string(),
        args_json: r#"{"contextName":"Work Management","eventName":"E1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-domain-event-to-foundation "Work Management" "E2"` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &["Work Management", "E2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows eventStorm.items contains both event items 'E1' and 'E2'
    let v = read_foundation(ws.path());
    let items = v["eventStorm"]["items"].as_array().expect("items array");
    let texts: Vec<&str> = items
        .iter()
        .filter(|i| i["type"].as_str() == Some("event"))
        .filter_map(|i| i["text"].as_str())
        .collect();
    assert!(texts.contains(&"E1"), "E1 must be present; got {texts:?}");
    assert!(texts.contains(&"E2"), "E2 must be present; got {texts:?}");

    // @step And the CLI bridge module codelet/fspec/src/add_domain_event_to_foundation.rs contains NO inline item construction, context lookup, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_domain_event_to_foundation.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_domain_event_to_foundation.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "bounded_context",
        "boundedContextId",
        "nextItemId",
        "write_json_atomic",
        "not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
