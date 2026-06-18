#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/review-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `review` (RPC-295).
// Each scenario maps to exactly one #[test] function with @step comments
// mirroring the Gherkin steps verbatim.
//
// RED PHASE: the current core stub is 1-arg `run(args_json)` -> NotYetPorted,
// and `review` is NOT yet in PORTED_COMMANDS, so every dispatch of `review`
// routes to the stub and returns success=false with the NotYetPorted message.
// These tests assert the REAL ported behaviour, so they FAIL now — that is the
// correct red-phase state.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, work_unit_id: &str) -> DispatchRequest {
    DispatchRequest {
        command: "review".to_string(),
        args_json: json!({ "workUnitId": work_unit_id }).to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&path, content).unwrap_or_else(|e| panic!("write {rel}: {e}"));
}

fn write_work_units(root: &Path, doc: Value) {
    write_file(root, "spec/work-units.json", &doc.to_string());
}

fn write_feature(root: &Path, name: &str, content: &str) {
    write_file(root, &format!("spec/features/{name}.feature"), content);
}

fn write_coverage(root: &Path, name: &str, doc: Value) {
    write_file(
        root,
        &format!("spec/features/{name}.feature.coverage"),
        &doc.to_string(),
    );
}

/// A minimal work-units.json document carrying a single AUTH-001 unit with the
/// supplied fields merged onto a sensible default.
fn work_units_doc(unit: Value) -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": { "AUTH-001": unit },
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": ["AUTH-001"], "done": [], "blocked": []
        }
    })
}

/// A one-scenario feature tagged with the supplied work-unit id (so the
/// re-implemented linked-feature lookup resolves it).
fn feature_one_scenario(work_unit_id: &str) -> String {
    format!(
        "@{work_unit_id}\nFeature: User Login\n\n  Scenario: Login\n    Given a user\n    When they log in\n    Then they see the dashboard\n"
    )
}

// ---------- scenarios ----------

#[test]
fn reviewing_a_fully_compliant_work_unit_reports_pass() {
    // Scenario: Reviewing a fully compliant work unit reports an overall PASS

    // @step Given a work unit with Example Mapping rules
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "validating",
            "rules": [ { "id": 0, "text": "Password must be 8+ chars", "deleted": false } ],
            "examples": [ { "id": 0, "text": "valid login", "deleted": false } ],
            "stateHistory": [
                { "state": "specifying", "timestamp": "2026-06-01T00:00:00.000Z" },
                { "state": "validating", "timestamp": "2026-06-01T01:00:00.000Z" }
            ],
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );

    // @step And a linked feature file whose coverage is 100 percent
    write_feature(tmp.path(), "user-login", &feature_one_scenario("AUTH-001"));
    write_file(tmp.path(), "tests/login.test.ts", "const x: string = 'ok';\n");
    write_coverage(
        tmp.path(),
        "user-login",
        json!({
            "stats": { "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100 },
            "scenarios": [ { "name": "Login", "testMappings": [ { "file": "tests/login.test.ts", "lines": "1-1" } ] } ]
        }),
    );

    // @step And no linked test file contains coding-standards violations
    // (tests/login.test.ts above is clean.)

    // @step When I dispatch review for that work unit id
    let result = dispatch_command(req(tmp.path(), "AUTH-001"));

    // @step Then the result success flag is true
    assert!(result.success, "expected success=true; got {result:?}");
    let report = &result.data;

    // @step And the report header contains "REVIEW:" with the work unit id and title
    assert!(report.contains("REVIEW: AUTH-001 - Login"), "header missing; got:\n{report}");

    // @step And the report Issues Found section reports "No critical issues detected."
    assert!(report.contains("No critical issues detected."), "got:\n{report}");

    // @step And the report ACDD Compliance section lists "Example Mapping completed"
    assert!(report.contains("Example Mapping completed"), "got:\n{report}");

    // @step And the report Summary section contains "Overall Assessment: PASS"
    assert!(report.contains("**Overall Assessment:** PASS"), "got:\n{report}");
}

#[test]
fn reviewing_with_no_linked_feature_emits_a_warning_and_no_coverage_data() {
    // Scenario: Reviewing a work unit with no linked feature emits a warning and no coverage data

    // @step Given a work unit in specifying status with no linked feature file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "specifying",
            "rules": [ { "id": 0, "text": "a rule", "deleted": false } ],
            "examples": [ { "id": 0, "text": "an example", "deleted": false } ],
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );
    // No feature file tagged @AUTH-001 exists, so linkedFeatures is empty.

    // @step When I dispatch review for that work unit id
    let result = dispatch_command(req(tmp.path(), "AUTH-001"));

    // @step Then the result success flag is true
    assert!(result.success, "expected success=true; got {result:?}");
    let report = &result.data;

    // @step And the report Warnings section contains "No linked feature files found"
    assert!(report.contains("No linked feature files found"), "got:\n{report}");

    // @step And the report Coverage Analysis section contains "No coverage data available"
    assert!(report.contains("No coverage data available"), "got:\n{report}");
}

#[test]
fn reviewing_with_partial_coverage_reports_needs_work_and_lists_uncovered() {
    // Scenario: Reviewing a work unit with partial coverage reports NEEDS WORK and lists uncovered scenarios

    // @step Given a work unit with a linked feature file whose coverage is 50 percent
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "implementing",
            "rules": [ { "id": 0, "text": "a rule", "deleted": false } ],
            "stateHistory": [ { "state": "specifying", "timestamp": "2026-06-01T00:00:00.000Z" } ],
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );
    write_feature(
        tmp.path(),
        "user-login",
        "@AUTH-001\nFeature: User Login\n\n  Scenario: Covered\n    Given a\n    When b\n    Then c\n\n  Scenario: Uncovered Scenario\n    Given a\n    When b\n    Then c\n",
    );
    write_file(tmp.path(), "tests/login.test.ts", "const x: string = 'ok';\n");

    // @step And the coverage file lists one uncovered scenario
    write_coverage(
        tmp.path(),
        "user-login",
        json!({
            "stats": { "totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50 },
            "scenarios": [
                { "name": "Covered", "testMappings": [ { "file": "tests/login.test.ts", "lines": "1-1" } ] },
                { "name": "Uncovered Scenario", "testMappings": [] }
            ]
        }),
    );

    // @step When I dispatch review for that work unit id
    let result = dispatch_command(req(tmp.path(), "AUTH-001"));
    assert!(result.success, "expected success=true; got {result:?}");
    let report = &result.data;

    // @step Then the report Summary section contains "Overall Assessment: NEEDS WORK"
    assert!(report.contains("**Overall Assessment:** NEEDS WORK"), "got:\n{report}");

    // @step And the report includes a recommendation to "Add tests for uncovered scenarios"
    assert!(report.contains("Add tests for uncovered scenarios"), "got:\n{report}");

    // @step And the report Coverage Analysis section lists the uncovered scenario name
    assert!(report.contains("Uncovered Scenario"), "got:\n{report}");
}

#[test]
fn reviewing_a_work_unit_whose_test_file_uses_any_reports_a_critical_issue() {
    // Scenario: Reviewing a work unit whose linked test file uses the any type reports a critical issue

    // @step Given a work unit with a linked feature file and 100 percent coverage
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "implementing",
            "rules": [ { "id": 0, "text": "a rule", "deleted": false } ],
            "stateHistory": [ { "state": "specifying", "timestamp": "2026-06-01T00:00:00.000Z" } ],
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );
    write_feature(tmp.path(), "user-login", &feature_one_scenario("AUTH-001"));

    // @step And a linked test file whose contents include ": any"
    write_file(tmp.path(), "tests/bad.test.ts", "const x: any = 1;\n");
    write_coverage(
        tmp.path(),
        "user-login",
        json!({
            "stats": { "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100 },
            "scenarios": [ { "name": "Login", "testMappings": [ { "file": "tests/bad.test.ts", "lines": "1-1" } ] } ]
        }),
    );

    // @step When I dispatch review for that work unit id
    let result = dispatch_command(req(tmp.path(), "AUTH-001"));
    assert!(result.success, "expected success=true; got {result:?}");
    let report = &result.data;

    // @step Then the report Critical Issues section contains "Use of `any` type detected"
    assert!(report.contains("Use of `any` type detected"), "got:\n{report}");

    // @step And the report Summary section contains "Overall Assessment: CRITICAL ISSUES"
    assert!(report.contains("**Overall Assessment:** CRITICAL ISSUES"), "got:\n{report}");
}

#[test]
fn reviewing_a_missing_work_unit_returns_an_error() {
    // Scenario: Reviewing a work unit id that does not exist returns an error

    // @step Given a work units store that does not contain the id "BOGUS-999"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "validating",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );

    // @step When I dispatch review for the work unit id "BOGUS-999"
    let result = dispatch_command(req(tmp.path(), "BOGUS-999"));

    // @step Then the dispatch returns an error whose message is "Work unit 'BOGUS-999' does not exist"
    assert!(!result.success, "expected failure; got {result:?}");
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'BOGUS-999' does not exist"),
        "missing canonical not-found text; got: {msg}"
    );

    // @step And no report is produced
    assert!(result.data.is_empty(), "no report should be produced on error; got: {}", result.data);
}

#[test]
fn the_ai_reminder_is_wrapped_according_to_the_configured_agent() {
    // Scenario: The AI deep-review reminder is wrapped according to the configured agent

    // @step Given a work unit with a linked feature file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "validating",
            "rules": [ { "id": 0, "text": "a rule", "deleted": false } ],
            "stateHistory": [ { "state": "specifying", "timestamp": "2026-06-01T00:00:00.000Z" } ],
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );
    write_feature(tmp.path(), "user-login", &feature_one_scenario("AUTH-001"));
    write_file(tmp.path(), "tests/login.test.ts", "const x: string = 'ok';\n");
    write_coverage(
        tmp.path(),
        "user-login",
        json!({
            "stats": { "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100 },
            "scenarios": [ { "name": "Login", "testMappings": [ { "file": "tests/login.test.ts", "lines": "1-1" } ] } ]
        }),
    );

    // The detected agent honours FSPEC_AGENT env > spec/fspec-config.json > default.
    // Remove FSPEC_AGENT for the duration so the config file drives detection,
    // then restore it. This test is the ONLY one that asserts on the reminder
    // wrapper, so env mutation cannot perturb the other tests' assertions.
    let prev = std::env::var_os("FSPEC_AGENT");
    std::env::remove_var("FSPEC_AGENT");

    // @step And spec/fspec-config.json selects the agent "claude"
    write_file(tmp.path(), "spec/fspec-config.json", &json!({ "agent": "claude" }).to_string());

    // @step When I dispatch review for that work unit id
    let claude = dispatch_command(req(tmp.path(), "AUTH-001"));

    // @step Then the final AI deep-review reminder is wrapped in "<system-reminder>" tags
    let claude_ok = claude.success && claude.data.contains("<system-reminder>");

    // @step When spec/fspec-config.json selects the cli agent "aider"
    write_file(tmp.path(), "spec/fspec-config.json", &json!({ "agent": "aider" }).to_string());

    // @step And I dispatch review for that work unit id again
    let aider = dispatch_command(req(tmp.path(), "AUTH-001"));

    // Restore env before asserting so a failure cannot leak into other tests.
    match prev {
        Some(v) => std::env::set_var("FSPEC_AGENT", v),
        None => std::env::remove_var("FSPEC_AGENT"),
    }

    assert!(claude_ok, "claude agent must wrap reminder in <system-reminder>; got:\n{}", claude.data);

    // @step Then the final AI deep-review reminder is prefixed with "**IMPORTANT:**"
    assert!(aider.success, "expected success=true; got {aider:?}");
    assert!(
        aider.data.contains("**IMPORTANT:**"),
        "cli agent must prefix reminder with **IMPORTANT:**; got:\n{}",
        aider.data
    );
    assert!(
        !aider.data.contains("<system-reminder>"),
        "cli agent must NOT use system-reminder tags; got:\n{}",
        aider.data
    );
}

#[test]
fn reviewing_a_non_backlog_unit_with_no_example_mapping_reports_an_acdd_failure() {
    // Scenario: Reviewing a non-backlog work unit with no Example Mapping reports an ACDD failure

    // @step Given a work unit in specifying status with no rules or examples
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        work_units_doc(json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "specifying",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        })),
    );

    // @step When I dispatch review for that work unit id
    let result = dispatch_command(req(tmp.path(), "AUTH-001"));
    assert!(result.success, "expected success=true; got {result:?}");
    let report = &result.data;

    // @step Then the report ACDD Compliance section lists the failure "No Example Mapping data found"
    assert!(report.contains("No Example Mapping data found"), "got:\n{report}");

    // @step And the report includes a recommendation to "Complete Example Mapping before specifying"
    assert!(report.contains("Complete Example Mapping before specifying"), "got:\n{report}");
}
