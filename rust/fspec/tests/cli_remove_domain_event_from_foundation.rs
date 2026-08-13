//! CLI surface for the `remove-domain-event-from-foundation` subcommand on the
//! standalone fspec Rust binary — RPC-272.
//!
//! Feature: spec/features/remove-domain-event-from-foundation-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim. Twin:
//! rust/fspec/tests/cli_remove_command_from_foundation.rs (RPC-270).

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
    cmd.arg("remove-domain-event-from-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec remove-domain-event-from-foundation");
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

fn bounded_context(id: u64, text: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "type": "bounded_context",
        "text": text,
        "color": null,
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn event_item(id: u64, text: &str, bc_id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "type": "event",
        "text": text,
        "boundedContextId": bc_id,
        "color": "orange",
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn foundation_with_items(items: Vec<serde_json::Value>, next_item_id: u64) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {
            "level": "big_picture",
            "items": items,
            "nextItemId": next_item_id
        }
    })
}

fn event_deleted(foundation: &serde_json::Value, text: &str) -> Option<bool> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("event") && i["text"].as_str() == Some(text))
        .and_then(|i| i["deleted"].as_bool())
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-domain-event-from-foundation.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_remove_domain_event_from_foundation_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec remove-domain-event-from-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("remove-domain-event-from-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-domain-event-from-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "remove-domain-event-from-foundation --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-domain-event-from-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully soft-deletes a domain event and prints the
//           success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_soft_deletes_domain_event() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=false
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0),
            ],
            2,
        ),
    );

    // @step When I run `fspec remove-domain-event-from-foundation "Work Management" "WorkUnitCreated"` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["Work Management", "WorkUnitCreated"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Removed domain event "WorkUnitCreated" from "Work Management" bounded context'
    assert!(
        stdout.contains(
            "✓ Removed domain event \"WorkUnitCreated\" from \"Work Management\" bounded context"
        ),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows the WorkUnitCreated event item deleted=true
    let v = read_foundation(ws.path());
    assert_eq!(
        event_deleted(&v, "WorkUnitCreated"),
        Some(true),
        "event must be soft-deleted"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a missing event with exit 1 and the TS-parity prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_missing_event_with_exit_1() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0),
            ],
            2,
        ),
    );
    let pre_bytes = fs::read(ws.path().join("spec/foundation.json")).unwrap();

    // @step When I run `fspec remove-domain-event-from-foundation "Work Management" "Ghost"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["Work Management", "Ghost"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error: prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Domain event 'Ghost' not found in bounded context 'Work Management'"
    assert!(
        stderr.contains("Domain event 'Ghost' not found in bounded context 'Work Management'"),
        "stderr must contain canonical missing-event message; got:\n{stderr}"
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
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and events text='E1' and text='E2' both boundedContextId=0
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "E1", 0),
                event_item(2, "E2", 0),
            ],
            3,
        ),
    );

    // @step When I dispatch remove-domain-event-from-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' eventName='E1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-domain-event-from-foundation".to_string(),
        args_json: r#"{"contextName":"Work Management","eventName":"E1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec remove-domain-event-from-foundation "Work Management" "E2"` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &["Work Management", "E2"]);
    assert_eq!(
        code, 0,
        "CLI remove must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows both event items E1 and E2 with deleted=true
    let v = read_foundation(ws.path());
    assert_eq!(
        event_deleted(&v, "E1"),
        Some(true),
        "E1 must be soft-deleted"
    );
    assert_eq!(
        event_deleted(&v, "E2"),
        Some(true),
        "E2 must be soft-deleted"
    );

    // @step And the CLI bridge module rust/fspec/src/remove_domain_event_from_foundation.rs contains NO inline context lookup, event match, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_domain_event_from_foundation.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/remove_domain_event_from_foundation.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "bounded_context",
        "boundedContextId",
        "write_json_atomic",
        "not found",
        "deleted",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
