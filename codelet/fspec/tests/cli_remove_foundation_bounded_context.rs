//! CLI surface for the `remove-foundation-bounded-context` subcommand on the
//! standalone fspec Rust binary — RPC-274.
//!
//! Feature: spec/features/remove-foundation-bounded-context-cli-subcommand.feature
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

fn run_remove(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-foundation-bounded-context");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec remove-foundation-bounded-context");
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

fn read_foundation_raw(cwd: &Path) -> String {
    fs::read_to_string(cwd.join("spec/foundation.json")).expect("read foundation.json")
}

fn base_foundation() -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": []}
    })
}

fn item_by_text<'a>(f: &'a serde_json::Value, text: &str) -> &'a serde_json::Value {
    f["eventStorm"]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|i| i["text"].as_str() == Some(text))
        .unwrap_or_else(|| panic!("no item with text={text}"))
}

/// Foundation seeded with a 'Sales' bounded context plus two non-deleted
/// children carrying its boundedContextId.
fn foundation_with_sales_and_children() -> serde_json::Value {
    let mut f = base_foundation();
    f["eventStorm"] = serde_json::json!({
        "level": "big_picture",
        "items": [
            {"id": 1, "type": "bounded_context", "text": "Sales", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"},
            {"id": 2, "type": "aggregate", "text": "Order", "color": "yellow", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:01.000Z"},
            {"id": 3, "type": "event", "text": "OrderPlaced", "color": "orange", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:02.000Z"}
        ],
        "nextItemId": 4
    });
    f
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-foundation-bounded-context.txt");

#[test]
fn scenario_help_output_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec remove-foundation-bounded-context --help`
    let output = Command::new(fspec_bin())
        .arg("remove-foundation-bounded-context")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-foundation-bounded-context --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-foundation-bounded-context.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'REMOVE-FOUNDATION-BOUNDED-CONTEXT'
    assert!(
        stdout.starts_with("\nREMOVE-FOUNDATION-BOUNDED-CONTEXT"),
        "stdout must start with blank line + heading; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI soft-deletes a childless bounded context and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_soft_deletes_childless_context_and_prints_success_line() {
    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false and no children
    let ws = tempfile::tempdir().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = serde_json::json!({
        "level": "big_picture",
        "items": [{"id": 1, "type": "bounded_context", "text": "Identity", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}],
        "nextItemId": 2
    });
    write_foundation(ws.path(), &f);

    // @step When I run `fspec remove-foundation-bounded-context "Identity"` in that tempdir
    let (code, stdout, stderr) = run_remove(ws.path(), &["Identity"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Removed bounded context "Identity" from foundation Event Storm'
    assert!(
        stdout.contains("✓ Removed bounded context \"Identity\" from foundation Event Storm"),
        "missing success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows the 'Identity' bounded_context item has deleted=true
    let data = read_foundation(ws.path());
    assert_eq!(
        item_by_text(&data, "Identity")["deleted"].as_bool(),
        Some(true)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --cascade removes the context and prints the cascade success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_cascade_removes_context_and_prints_cascade_success_line() {
    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_sales_and_children());

    // @step When I run `fspec remove-foundation-bounded-context "Sales" --cascade` in that tempdir
    let (code, stdout, stderr) = run_remove(ws.path(), &["Sales", "--cascade"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring '✓ Removed bounded context "Sales" and all its children from foundation Event Storm'
    assert!(
        stdout.contains(
            "✓ Removed bounded context \"Sales\" and all its children from foundation Event Storm"
        ),
        "missing cascade success line; got:\n{stdout}"
    );

    // @step And spec/foundation.json on disk shows both child items have deleted=true
    let data = read_foundation(ws.path());
    assert_eq!(
        item_by_text(&data, "Order")["deleted"].as_bool(),
        Some(true)
    );
    assert_eq!(
        item_by_text(&data, "OrderPlaced")["deleted"].as_bool(),
        Some(true)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a non-empty context without --cascade with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_non_empty_context_without_cascade() {
    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_sales_and_children());
    let pre = read_foundation_raw(ws.path());

    // @step When I run `fspec remove-foundation-bounded-context "Sales"` in that tempdir
    let (code, _stdout, stderr) = run_remove(ws.path(), &["Sales"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."
    assert!(
        stderr.contains("Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."),
        "missing refusal text; got:\n{stderr}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_foundation_raw(ws.path()), pre);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/foundation.json containing eventStorm bounded_contexts text='C1' and text='C2' both deleted=false and childless
    let ws = tempfile::tempdir().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = serde_json::json!({
        "level": "big_picture",
        "items": [
            {"id": 1, "type": "bounded_context", "text": "C1", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"},
            {"id": 2, "type": "bounded_context", "text": "C2", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:01.000Z"}
        ],
        "nextItemId": 3
    });
    write_foundation(ws.path(), &f);

    // @step When I dispatch remove-foundation-bounded-context via fspec_core::dispatch::dispatch_command with contextName='C1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-foundation-bounded-context".to_string(),
        args_json: r#"{"contextName":"C1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec remove-foundation-bounded-context "C2"` afterwards exits 0
    let (code, stdout, stderr) = run_remove(ws.path(), &["C2"]);
    assert_eq!(
        code, 0,
        "CLI remove must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json on disk shows both 'C1' and 'C2' items have deleted=true
    let data = read_foundation(ws.path());
    assert_eq!(item_by_text(&data, "C1")["deleted"].as_bool(), Some(true));
    assert_eq!(item_by_text(&data, "C2")["deleted"].as_bool(), Some(true));

    // @step And the CLI bridge module codelet/fspec/src/remove_foundation_bounded_context.rs contains NO inline find, soft-delete, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_foundation_bounded_context.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "boundedContextId",
        "write_json_atomic",
        "ensure_foundation_file",
        "child items",
        "deleted",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
