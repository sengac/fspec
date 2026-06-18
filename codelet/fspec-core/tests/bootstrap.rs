#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/bootstrap-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `bootstrap` (RPC-200).
// Each scenario maps to exactly one #[test] with @step comments mirroring the
// Gherkin steps verbatim.
//
// RED PHASE: the current core stub is 1-arg `run(args_json)` -> NotYetPorted,
// so every dispatch of `bootstrap` returns success=false with the NotYetPorted
// message. These tests assert the REAL ported behaviour, so they FAIL now —
// that is the correct red-phase state. They go green once
// commands::bootstrap::run(args_json, project_root) is ported and wired into
// the dispatcher AND the byte-exact bootstrap_doc.txt asset is captured.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path) -> DispatchRequest {
    DispatchRequest {
        command: "bootstrap".to_string(),
        args_json: "{}".to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_config(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-config.json"), body).expect("write fspec-config.json");
}

fn write_foundation(project_root: &Path, data: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(data).expect("serialize foundation"),
    )
    .expect("write foundation.json");
}

fn write_work_units(project_root: &Path, data: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(data).expect("serialize work-units"),
    )
    .expect("write work-units.json");
}

// ---------- scenarios ----------

#[test]
fn dispatch_returns_the_complete_documentation_for_an_empty_project() {
    // Scenario: Dispatch returns the complete documentation for an empty project

    // @step Given a project root tempdir with no fspec-config.json and no foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output is longer than 10000 characters
    assert!(
        result.data.len() > 10000,
        "expected >10000 chars; got {} chars",
        result.data.len()
    );

    // @step Then the rendered output contains the substring "<test-command>"
    assert!(
        result.data.contains("<test-command>"),
        "missing <test-command> placeholder"
    );

    // @step Then the rendered output contains the substring "<quality-check-commands>"
    assert!(
        result.data.contains("<quality-check-commands>"),
        "missing <quality-check-commands> placeholder"
    );

    // @step Then the rendered output does not contain the substring "BIG PICTURE EVENT STORMING NEEDED"
    assert!(
        !result.data.contains("BIG PICTURE EVENT STORMING NEEDED"),
        "must not append event-storm reminder when no foundation.json"
    );
}

#[test]
fn output_contains_the_header_marker_and_core_workflow_strings() {
    // Scenario: Output contains the header marker and core workflow strings

    // @step Given a project root tempdir with no fspec-config.json and no foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "# fspec Command - Kanban-Based Project Management"
    assert!(
        result
            .data
            .contains("# fspec Command - Kanban-Based Project Management"),
        "missing header marker"
    );

    // @step Then the rendered output contains the substring "ACDD"
    assert!(result.data.contains("ACDD"), "missing ACDD");

    // @step Then the rendered output contains the substring "Example Mapping"
    assert!(
        result.data.contains("Example Mapping"),
        "missing Example Mapping"
    );

    // @step Then the rendered output contains the substring "Story Point Estimation"
    assert!(
        result.data.contains("Story Point Estimation"),
        "missing Story Point Estimation"
    );

    // @step Then the rendered output contains the substring "Kanban"
    assert!(result.data.contains("Kanban"), "missing Kanban");
}

#[test]
fn output_contains_all_six_help_section_markers_in_order() {
    // Scenario: Output contains all six help-section markers in order

    // @step Given a project root tempdir with no fspec-config.json and no foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "GHERKIN SPECIFICATIONS"
    assert!(
        result.data.contains("GHERKIN SPECIFICATIONS"),
        "missing GHERKIN SPECIFICATIONS"
    );

    // @step Then the rendered output contains the substring "create-story"
    assert!(result.data.contains("create-story"), "missing create-story");

    // @step Then the rendered output contains the substring "add-rule"
    assert!(result.data.contains("add-rule"), "missing add-rule");

    // @step Then the rendered output contains the substring "query-metrics"
    assert!(
        result.data.contains("query-metrics"),
        "missing query-metrics"
    );

    // @step Then the rendered output contains the substring "discover-foundation"
    assert!(
        result.data.contains("discover-foundation"),
        "missing discover-foundation"
    );

    // @step Then the rendered output contains the substring "LIFECYCLE HOOKS"
    assert!(
        result.data.contains("LIFECYCLE HOOKS"),
        "missing LIFECYCLE HOOKS"
    );
}

#[test]
fn config_test_command_placeholder_is_replaced() {
    // Scenario: Config test-command placeholder is replaced

    // @step Given a project root tempdir whose spec/fspec-config.json sets tools.test.command to "cargo test"
    let tmp = TempDir::new().expect("tempdir");
    write_config(
        tmp.path(),
        &json!({ "tools": { "test": { "command": "cargo test" } } }).to_string(),
    );

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "cargo test"
    assert!(result.data.contains("cargo test"), "test command not injected");

    // @step Then the rendered output does not contain the substring "<test-command>"
    assert!(
        !result.data.contains("<test-command>"),
        "test-command placeholder still present"
    );
}

#[test]
fn config_quality_check_commands_placeholder_is_replaced_with_the_joined_string() {
    // Scenario: Config quality-check-commands placeholder is replaced with the joined string

    // @step Given a project root tempdir whose spec/fspec-config.json sets tools.qualityCheck.commands to ["cargo clippy", "cargo fmt --check"]
    let tmp = TempDir::new().expect("tempdir");
    write_config(
        tmp.path(),
        &json!({
            "tools": { "qualityCheck": { "commands": ["cargo clippy", "cargo fmt --check"] } }
        })
        .to_string(),
    );

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "cargo clippy && cargo fmt --check"
    assert!(
        result.data.contains("cargo clippy && cargo fmt --check"),
        "quality commands not joined/injected"
    );

    // @step Then the rendered output does not contain the substring "<quality-check-commands>"
    assert!(
        !result.data.contains("<quality-check-commands>"),
        "quality-check-commands placeholder still present"
    );
}

#[test]
fn malformed_config_leaves_placeholders_intact_and_still_succeeds() {
    // Scenario: Malformed config leaves placeholders intact and still succeeds

    // @step Given a project root tempdir whose spec/fspec-config.json contains invalid JSON
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "{ this is not valid json ");

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "<test-command>"
    assert!(
        result.data.contains("<test-command>"),
        "placeholder must remain intact on malformed config"
    );

    // @step Then the rendered output contains the substring "<quality-check-commands>"
    assert!(
        result.data.contains("<quality-check-commands>"),
        "placeholder must remain intact on malformed config"
    );
}

#[test]
fn event_storm_reminder_names_the_matching_found_work_unit() {
    // Scenario: Event Storm reminder names the matching FOUND work unit

    // @step Given a project root tempdir whose foundation.json has an empty eventStorm and a non-done FOUND-001 work unit titled "Conduct Foundation Event Storm"
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &json!({ "eventStorm": { "items": [] } }));
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "FOUND-001": {
                    "id": "FOUND-001",
                    "title": "Conduct Foundation Event Storm",
                    "type": "task",
                    "status": "specifying",
                    "createdAt": "2026-06-01T00:00:00.000Z",
                    "updatedAt": "2026-06-01T00:00:00.000Z"
                }
            },
            "states": {
                "backlog": [], "specifying": ["FOUND-001"], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "BIG PICTURE EVENT STORMING NEEDED"
    assert!(
        result.data.contains("BIG PICTURE EVENT STORMING NEEDED"),
        "missing event-storm reminder"
    );

    // @step Then the rendered output contains the substring "FOUND-001"
    assert!(
        result.data.contains("FOUND-001"),
        "reminder must name the matching work unit"
    );
}

#[test]
fn event_storm_reminder_suggests_creating_a_work_unit_when_none_matches() {
    // Scenario: Event Storm reminder suggests creating a work unit when none matches

    // @step Given a project root tempdir whose foundation.json has an empty eventStorm and no matching FOUND work unit
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &json!({ "eventStorm": { "items": [] } }));

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output contains the substring "BIG PICTURE EVENT STORMING NEEDED"
    assert!(
        result.data.contains("BIG PICTURE EVENT STORMING NEEDED"),
        "missing event-storm reminder"
    );

    // @step Then the rendered output contains the substring "fspec create-task FOUND"
    assert!(
        result.data.contains("fspec create-task FOUND"),
        "no-work-unit reminder must suggest create-task FOUND"
    );
}

#[test]
fn no_reminder_when_the_event_storm_is_already_populated() {
    // Scenario: No reminder when the event storm is already populated

    // @step Given a project root tempdir whose foundation.json eventStorm.items already has entries
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &json!({ "eventStorm": { "items": [{ "type": "event", "name": "WorkUnitCreated" }] } }),
    );

    // @step When I dispatch bootstrap
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output does not contain the substring "BIG PICTURE EVENT STORMING NEEDED"
    assert!(
        !result.data.contains("BIG PICTURE EVENT STORMING NEEDED"),
        "must not append reminder when event storm is populated"
    );
}

#[test]
fn cli_and_dispatcher_converge_on_the_same_fspec_core_run_function() {
    // Scenario: CLI and dispatcher converge on the same fspec_core run function

    // @step Given a project root tempdir with no fspec-config.json and no foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch bootstrap and also run the CLI subcommand fspec bootstrap against an equivalent project root
    let result = dispatch_command(req(tmp.path()));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then both paths produce output containing "# fspec Command - Kanban-Based Project Management"
    assert!(
        result
            .data
            .contains("# fspec Command - Kanban-Based Project Management"),
        "dispatcher output missing header marker"
    );

    // @step Then the CLI bridge module codelet/fspec/src/bootstrap.rs contains no documentation-building or transform logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fspec/src/bootstrap.rs");
    let bridge_src = fs::read_to_string(&bridge_path).expect("CLI bridge bootstrap.rs readable");
    assert!(
        bridge_src.contains("bootstrap::run") || bridge_src.contains("commands::bootstrap"),
        "bridge must delegate to bootstrap::run"
    );
    for forbidden in [
        "include_str!",
        "<test-command>",
        "<quality-check-commands>",
        "BIG PICTURE EVENT STORMING NEEDED",
        "# fspec Command - Kanban-Based Project Management",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
