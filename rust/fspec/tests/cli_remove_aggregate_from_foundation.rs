//! CLI surface for the `remove-aggregate-from-foundation` subcommand on the
//! standalone fspec Rust binary — RPC-266.
//!
//! Feature: spec/features/remove-aggregate-from-foundation-cli-subcommand.feature
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

fn run_remove(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-aggregate-from-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec remove-aggregate-from-foundation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path, value: &serde_json::Value) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser"),
    )
    .expect("write foundation.json");
}

fn read_foundation(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/foundation.json")).expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn bounded_context(text: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id, "type": "bounded_context", "text": text,
        "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"
    })
}

fn aggregate(text: &str, id: u64, bc_id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id, "type": "aggregate", "text": text, "boundedContextId": bc_id,
        "color": "yellow", "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"
    })
}

fn foundation_with_items(items: Vec<serde_json::Value>) -> serde_json::Value {
    let next = items
        .iter()
        .filter_map(|i| i["id"].as_u64())
        .max()
        .map_or(0, |m| m + 1);
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {"level": "big_picture", "items": items, "nextItemId": next}
    })
}

fn find_aggregate(data: &serde_json::Value, text: &str) -> serde_json::Value {
    data["eventStorm"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["type"].as_str() == Some("aggregate") && i["text"].as_str() == Some(text))
        .cloned()
        .unwrap_or_else(|| panic!("aggregate '{text}' must exist; data={data}"))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes remove-aggregate-from-foundation with positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_remove_aggregate_from_foundation_in_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec remove-aggregate-from-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("remove-aggregate-from-foundation")
        .arg("--help")
        .output()
        .expect("spawn remove-aggregate-from-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout describes the remove-aggregate-from-foundation subcommand
    assert!(
        stdout.contains("remove-aggregate-from-foundation")
            || stdout.contains("REMOVE-AGGREGATE-FROM-FOUNDATION"),
        "help must describe the subcommand; got:\n{stdout}"
    );

    // @step And stdout mentions the `<context-name>` argument
    assert!(
        stdout.contains("context-name"),
        "help must mention context-name; got:\n{stdout}"
    );

    // @step And stdout mentions the `<aggregate-name>` argument
    assert!(
        stdout.contains("aggregate-name"),
        "help must mention aggregate-name; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI soft-deletes an aggregate and prints the success message on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_soft_deletes_aggregate_and_prints_success_message() {
    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(vec![bounded_context("Sales", 0), aggregate("Order", 1, 0)]),
    );

    // @step When I run `./rust/target/release/fspec remove-aggregate-from-foundation Sales Order`
    let (code, stdout, stderr) = run_remove(ws.path(), &["Sales", "Order"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Removed aggregate "Order" from "Sales" bounded context'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Removed aggregate \"Order\" from \"Sales\" bounded context")),
        "missing success line; got:\n{stdout}"
    );

    // @step And the aggregate 'Order' in eventStorm.items has deleted=true
    let data = read_foundation(ws.path());
    assert_eq!(
        find_aggregate(&data, "Order")["deleted"].as_bool(),
        Some(true)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown aggregate with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_aggregate_with_exit_1() {
    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(vec![bounded_context("Sales", 0), aggregate("Order", 1, 0)]),
    );

    // @step When I run `./rust/target/release/fspec remove-aggregate-from-foundation Sales Ghost`
    let (code, _stdout, stderr) = run_remove(ws.path(), &["Sales", "Ghost"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Aggregate 'Ghost' not found in bounded context 'Sales'"
    assert!(
        stderr.contains("Aggregate 'Ghost' not found in bounded context 'Sales'"),
        "stderr must mention aggregate not-found; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and aggregates 'Order' and 'Shipment' linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(
        ws.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0),
            aggregate("Order", 1, 0),
            aggregate("Shipment", 2, 0),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation via fspec_core::dispatch::dispatch_command with contextName='Sales' aggregateName='Order'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-aggregate-from-foundation".to_string(),
        args_json: r#"{"contextName":"Sales","aggregateName":"Order"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./rust/target/release/fspec remove-aggregate-from-foundation Sales Shipment` afterwards exits 0
    let (code, stdout, stderr) = run_remove(ws.path(), &["Sales", "Shipment"]);
    assert_eq!(
        code, 0,
        "CLI remove must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And both the 'Order' and 'Shipment' aggregates have deleted=true
    let data = read_foundation(ws.path());
    assert_eq!(
        find_aggregate(&data, "Order")["deleted"].as_bool(),
        Some(true)
    );
    assert_eq!(
        find_aggregate(&data, "Shipment")["deleted"].as_bool(),
        Some(true)
    );

    // @step And the CLI bridge module rust/fspec/src/remove_aggregate_from_foundation.rs contains NO inline bounded-context lookup, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_aggregate_from_foundation.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "ensure_foundation_file",
        "write_json_atomic",
        "boundedContextId",
        "eventStorm",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: remove-aggregate-from-foundation --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-aggregate-from-foundation.txt");

#[test]
fn scenario_remove_aggregate_from_foundation_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec remove-aggregate-from-foundation --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("remove-aggregate-from-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-aggregate-from-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/remove-aggregate-from-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
