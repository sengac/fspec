#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/deprecated-scenario-gate-exemption.feature
//
// Validates the acceptance criteria for the @deprecated coverage-exemption
// mechanism (COV-056). Each scenario maps to exactly one #[test] function
// with @step comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: the coverage gate does not yet read @deprecated tags, so
// scenarios 1, 3, and 4 fail now — that is the correct red state.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, id: &str, status: &str, extra_fields: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let extra = if extra_fields.trim().is_empty() {
        String::new()
    } else {
        format!(", {extra_fields}")
    };
    let mut parts = Vec::new();
    for s in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        parts.push(if s == status {
            format!(r#""{s}": ["{id}"]"#)
        } else {
            format!(r#""{s}": []"#)
        });
    }
    let raw = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "T", "type": "story", "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"{extra}
    }}
  }},
  "states": {{ {} }}
}}"#,
        parts.join(", ")
    );
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Write spec/features/<name>.feature + <name>.feature.coverage. `feature_body`
/// is the scenario body of the feature file (tagged @DEP-001); `scenarios_json`
/// is the raw coverage scenarios array body (without brackets).
fn write_feature_with_coverage(project_root: &Path, name: &str, feature_body: &str, scenarios_json: &str) {
    let dir = project_root.join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    let content = format!("@DEP-001\nFeature: {name}\n\n{feature_body}\n");
    fs::write(dir.join(format!("{name}.feature")), content).expect("write feature");
    let body = format!(r#"{{ "scenarios": [{scenarios_json}] }}"#);
    fs::write(dir.join(format!("{name}.feature.coverage")), body).expect("write coverage");
}

fn status_of(project_root: &Path, id: &str) -> String {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json")).expect("read");
    let data: Value = serde_json::from_str(&raw).expect("valid JSON");
    data["workUnits"][id]["status"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// Combined failure text (error + system-reminder + data).
fn failure_text(result: &codelet_fspec_core::DispatchResult) -> String {
    format!(
        "{} {} {}",
        result.error.as_deref().unwrap_or(""),
        result
            .system_reminder
            .as_deref()
            .unwrap_or(""),
        result.data.as_str()
    )
}

#[test]
fn done_gate_exempts_scenario_level_deprecated_scenario_without_test_mapping() {
    // Scenario: Done-gate exempts a scenario-level @deprecated scenario without test mapping

    // @step Given a work unit with status "validating" linked to a feature file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "DEP-001", "validating", r#""linkedFeatures": ["dep-feature"]"#);

    // @step And the feature has one live scenario with test mapping and one scenario tagged @deprecated with no test mapping
    write_feature_with_coverage(
        tmp.path(),
        "dep-feature",
        "  Scenario: Live behavior\n    Given a live precondition\n    When the live action happens\n    Then the live outcome is observed\n\n  @deprecated\n  Scenario: Stale behavior\n    Given a stale precondition\n    When the stale action happens\n    Then the stale outcome is observed",
        r#"{ "name": "Live behavior", "testMappings": [{ "file": "t.rs", "lines": "1-2", "implMappings": [{ "file": "i.rs", "lines": "3-4" }] }] },
           { "name": "Stale behavior", "testMappings": [] }"#,
    );
    fs::write(tmp.path().join("t.rs"), "// @step Given a live precondition\n// @step When the live action happens\n// @step Then the live outcome is observed\n").expect("write test");
    fs::write(tmp.path().join("i.rs"), "fn live() {}\n").expect("write impl");

    // @step When the dispatcher moves the work unit to "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DEP-001", "status": "done", "skipTemporalValidation": true}),
    ));

    // @step Then the transition succeeds
    assert!(
        result.success,
        "deprecated scenario must not block done; got {result:?}"
    );

    // @step And the work unit status becomes "done"
    assert_eq!(status_of(tmp.path(), "DEP-001"), "done");

    // @step And no error mentions the deprecated scenario as uncovered
    let text = failure_text(&result);
    assert!(
        !text.contains("Stale behavior"),
        "deprecated scenario must not be listed as uncovered; got: {text}"
    );
}

#[test]
fn done_gate_still_blocks_on_an_uncovered_live_scenario() {
    // Scenario: Done-gate still blocks on an uncovered live scenario

    // @step Given a work unit with status "validating" linked to a feature file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "DEP-001", "validating", r#""linkedFeatures": ["dep-feature"]"#);

    // @step And the feature has one untagged scenario with no test mapping
    write_feature_with_coverage(
        tmp.path(),
        "dep-feature",
        "  Scenario: Live behavior\n    Given a live precondition\n    When the live action happens\n    Then the live outcome is observed",
        r#"{ "name": "Live behavior", "testMappings": [] }"#,
    );

    // @step When the dispatcher moves the work unit to "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DEP-001", "status": "done", "skipTemporalValidation": true}),
    ));

    // @step Then the transition fails
    assert!(!result.success, "uncovered live scenario must block done; got {result:?}");

    // @step And the error lists the untagged scenario as uncovered
    let text = failure_text(&result);
    assert!(
        text.contains("Live behavior") && text.to_lowercase().contains("uncovered"),
        "uncovered live scenario must be listed; got: {text}"
    );
}

#[test]
fn validating_gate_exempts_deprecated_scenario_without_implementation_mapping() {
    // Scenario: Validating gate exempts a @deprecated scenario without implementation mapping

    // @step Given a work unit with status "implementing" linked to a feature file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "DEP-001", "implementing", r#""linkedFeatures": ["dep-feature"]"#);

    // @step And the feature has one scenario tagged @deprecated whose test mapping has no impl mapping
    write_feature_with_coverage(
        tmp.path(),
        "dep-feature",
        "  @deprecated\n  Scenario: Stale behavior\n    Given a stale precondition\n    When the stale action happens\n    Then the stale outcome is observed",
        r#"{ "name": "Stale behavior", "testMappings": [{ "file": "t.rs", "lines": "1-2", "implMappings": [] }] }"#,
    );
    fs::write(tmp.path().join("t.rs"), "// @step Given a stale precondition\n// @step When the stale action happens\n// @step Then the stale outcome is observed\n").expect("write test");

    // @step When the dispatcher moves the work unit to "validating"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DEP-001", "status": "validating", "skipTemporalValidation": true}),
    ));

    // @step Then the transition succeeds
    assert!(
        result.success,
        "deprecated scenario must not block validating; got {result:?}"
    );

    // @step And the work unit status becomes "validating"
    assert_eq!(status_of(tmp.path(), "DEP-001"), "validating");
}

#[test]
fn feature_level_deprecated_deprecates_every_scenario_in_the_feature() {
    // Scenario: Feature-level @deprecated deprecates every scenario in the feature

    // @step Given a work unit with status "validating" linked to a feature file tagged @deprecated on the Feature header
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "DEP-001", "validating", r#""linkedFeatures": ["dep-feature"]"#);

    // @step And the feature has three scenarios, none with test mappings
    let dir = tmp.path().join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    fs::write(
        dir.join("dep-feature.feature"),
        "@DEP-001\n@deprecated\nFeature: dep-feature\n\n  Scenario: One\n    Given a\n    When b\n    Then c\n\n  Scenario: Two\n    Given a\n    When b\n    Then c\n\n  Scenario: Three\n    Given a\n    When b\n    Then c\n",
    )
    .expect("write feature");
    fs::write(
        dir.join("dep-feature.feature.coverage"),
        r#"{ "scenarios": [{ "name": "One", "testMappings": [] }, { "name": "Two", "testMappings": [] }, { "name": "Three", "testMappings": [] }] }"#,
    )
    .expect("write coverage");

    // @step When the dispatcher moves the work unit to "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DEP-001", "status": "done", "skipTemporalValidation": true}),
    ));

    // @step Then the transition succeeds
    assert!(
        result.success,
        "feature-level @deprecated must exempt all scenarios; got {result:?}"
    );

    // @step And no scenario in the feature is reported as uncovered
    let text = failure_text(&result);
    assert!(!text.contains("uncovered"), "no scenario may be reported uncovered; got: {text}");
}
