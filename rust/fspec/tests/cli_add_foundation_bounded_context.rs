//! CLI surface for the `add-foundation-bounded-context` subcommand on the
//! standalone fspec Rust binary — RPC-183.
//!
//! Feature: spec/features/add-foundation-bounded-context-cli-subcommand.feature
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

fn run_add(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-foundation-bounded-context");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec add-foundation-bounded-context");
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

fn empty_foundation() -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": []}
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-foundation-bounded-context.txt");

#[test]
fn scenario_help_output_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-foundation-bounded-context --help`
    let output = Command::new(fspec_bin())
        .arg("add-foundation-bounded-context")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-foundation-bounded-context --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-foundation-bounded-context.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'ADD-FOUNDATION-BOUNDED-CONTEXT'
    assert!(
        stdout.starts_with("\nADD-FOUNDATION-BOUNDED-CONTEXT"),
        "stdout must start with blank line + heading; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully appends a bounded context and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_appends_bounded_context_and_prints_success_line() {
    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I run `fspec add-foundation-bounded-context "Order Management"` in that tempdir
    let (code, stdout, stderr) = run_add(ws.path(), &["Order Management"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Added bounded context "Order Management" to foundation Event Storm'
    assert!(
        stdout.contains("✓ Added bounded context \"Order Management\" to foundation Event Storm"),
        "missing success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows eventStorm.items has length 1
    let data = read_foundation(ws.path());
    let items = data["eventStorm"]["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);

    // @step And spec/foundation.json on disk shows eventStorm.items[0].text='Order Management'
    assert_eq!(items[0]["text"].as_str(), Some("Order Management"));

    // @step And spec/foundation.json on disk shows eventStorm.items[0].type='bounded_context'
    assert_eq!(items[0]["type"].as_str(), Some("bounded_context"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints ONLY the success line (no FOUNDATION.md regeneration line)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_only_success_line_no_regeneration_line() {
    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I run `fspec add-foundation-bounded-context "Identity"` in that tempdir
    let (code, stdout, stderr) = run_add(ws.path(), &["Identity"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout does NOT contain the substring 'Regenerated' (the TS command calls generateFoundationMdCommand whose result is discarded — it never prints a regeneration line)
    assert!(
        !stdout.contains("Regenerated"),
        "stdout must NOT contain a regeneration line (TS parity); got:\n{stdout}"
    );

    // @step And stdout contains the substring '✓ Added bounded context "Identity" to foundation Event Storm'
    assert!(
        stdout.contains("✓ Added bounded context \"Identity\" to foundation Event Storm"),
        "missing success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I dispatch add-foundation-bounded-context via fspec_core::dispatch::dispatch_command with text='C1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-foundation-bounded-context".to_string(),
        args_json: r#"{"text":"C1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-foundation-bounded-context "C2"` afterwards exits 0
    let (code, stdout, stderr) = run_add(ws.path(), &["C2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows eventStorm.items has length 2
    let data = read_foundation(ws.path());
    let items = data["eventStorm"]["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2, "expected two items, got {items:?}");

    // @step And the CLI bridge module rust/fspec/src/add_foundation_bounded_context.rs contains NO inline item construction, seeding, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_foundation_bounded_context.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "big_picture",
        "nextItemId",
        "write_json_atomic",
        "ensure_foundation_file",
        "bounded_context",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
