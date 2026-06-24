//! CLI surface for the `add-aggregate-to-foundation` subcommand on the
//! standalone fspec Rust binary — RPC-166.
//!
//! Feature: spec/features/add-aggregate-to-foundation-cli-subcommand.feature
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

fn run_add(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-aggregate-to-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec add-aggregate-to-foundation");
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

fn foundation_with_context(text: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {
            "level": "big_picture",
            "items": [
                {"id": id, "type": "bounded_context", "text": text, "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}
            ],
            "nextItemId": id + 1
        }
    })
}

fn aggregates(data: &serde_json::Value) -> Vec<serde_json::Value> {
    data["eventStorm"]["items"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|i| i["type"].as_str() == Some("aggregate"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes add-aggregate-to-foundation with positional args and description flag in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_aggregate_to_foundation_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-aggregate-to-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("add-aggregate-to-foundation")
        .arg("--help")
        .output()
        .expect("spawn add-aggregate-to-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout describes the add-aggregate-to-foundation subcommand
    assert!(
        stdout.contains("add-aggregate-to-foundation")
            || stdout.contains("ADD-AGGREGATE-TO-FOUNDATION"),
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

    // @step And stdout mentions the `--description` option
    assert!(
        stdout.contains("--description"),
        "help must mention --description; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds an aggregate and prints the success message on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_aggregate_and_prints_success_message() {
    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_context("Sales", 0));

    // @step When I run `./codelet/target/release/fspec add-aggregate-to-foundation Sales Order`
    let (code, stdout, stderr) = run_add(ws.path(), &["Sales", "Order"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Added aggregate "Order" to "Sales" bounded context'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Added aggregate \"Order\" to \"Sales\" bounded context")),
        "missing success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json eventStorm.items contains one aggregate item with text='Order'
    let data = read_foundation(ws.path());
    let aggs = aggregates(&data);
    assert_eq!(aggs.len(), 1, "expected 1 aggregate; got {aggs:?}");
    assert_eq!(aggs[0]["text"].as_str(), Some("Order"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI persists the optional description via the -d flag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_persists_description_via_d_flag() {
    // @step Given spec/foundation.json contains a bounded_context item 'Billing' with id=0 in eventStorm.items
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_context("Billing", 0));

    // @step When I run `./codelet/target/release/fspec add-aggregate-to-foundation Billing Invoice -d "Billing root"`
    let (code, stdout, stderr) = run_add(ws.path(), &["Billing", "Invoice", "-d", "Billing root"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And the aggregate 'Invoice' has description='Billing root'
    let data = read_foundation(ws.path());
    let inv = aggregates(&data)
        .into_iter()
        .find(|a| a["text"].as_str() == Some("Invoice"))
        .expect("Invoice aggregate must exist");
    assert_eq!(inv["description"].as_str(), Some("Billing root"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown bounded context with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_bounded_context_with_exit_1() {
    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_context("Sales", 0));

    // @step When I run `./codelet/target/release/fspec add-aggregate-to-foundation Unknown Order`
    let (code, _stdout, stderr) = run_add(ws.path(), &["Unknown", "Order"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Bounded context 'Unknown' not found"
    assert!(
        stderr.contains("Bounded context 'Unknown' not found"),
        "stderr must mention not-found; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_context("Sales", 0));

    // @step When I dispatch add-aggregate-to-foundation via fspec_core::dispatch::dispatch_command with contextName='Sales' aggregateName='ViaDispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-aggregate-to-foundation".to_string(),
        args_json: r#"{"contextName":"Sales","aggregateName":"ViaDispatcher"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./codelet/target/release/fspec add-aggregate-to-foundation Sales ViaCli` afterwards exits 0
    let (code, stdout, stderr) = run_add(ws.path(), &["Sales", "ViaCli"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json eventStorm.items contains two aggregate items
    let data = read_foundation(ws.path());
    assert_eq!(
        aggregates(&data).len(),
        2,
        "expected two aggregates; data={data}"
    );

    // @step And the CLI bridge module codelet/fspec/src/add_aggregate_to_foundation.rs contains NO inline bounded-context lookup, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_aggregate_to_foundation.rs");
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
        "nextItemId",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-aggregate-to-foundation --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-aggregate-to-foundation.txt");

#[test]
fn scenario_add_aggregate_to_foundation_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-aggregate-to-foundation --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-aggregate-to-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-aggregate-to-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-aggregate-to-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
