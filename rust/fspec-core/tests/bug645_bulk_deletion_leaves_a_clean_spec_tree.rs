//! BUG-645 — bulk deletion leaves a clean spec tree: delete-features deletes
//! the deleted features' coverage files, and delete-scenarios removes whole
//! scenario blocks the way delete-scenario does (no trailing blank line, no
//! swept-away Rule).
//!
//! Feature: spec/features/bulk-deletion-leaves-a-clean-spec-tree.feature

use std::path::Path;

use codelet_fspec_core::io::gherkin::parse_feature_lenient;
use codelet_fspec_core::io::gherkin_format::format_feature;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const COVERAGE: &str = r#"{"scenarios":[{"name":"A","testMappings":[]}],"stats":{"totalScenarios":1,"coveredScenarios":0,"coveragePercent":0,"testFiles":[],"implFiles":[],"totalLinesCovered":0}}"#;

fn write(root: &Path, rel: &str, body: &str) -> std::io::Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
}

fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}

/// Feature `name` with feature-level `tags` and one scenario "A", and its
/// coverage file when `coverage` is set.
fn feature(root: &Path, name: &str, tags: &str, coverage: bool) -> std::io::Result<()> {
    let rel = format!("spec/features/{name}.feature");
    write(
        root,
        &rel,
        &format!("{tags}\nFeature: {name}\n\n  Scenario: A\n    Given x\n"),
    )?;
    if coverage {
        write(root, &format!("{rel}.coverage"), COVERAGE)?;
    }
    Ok(())
}

/// Run `command` through the dispatcher; its inner JSON payload.
fn dispatch(root: &Path, command: &str, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("{command} failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

/// The formatter's canonical form of a feature source.
fn canonical(body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let feature = parse_feature_lenient(body).map_err(|error| format!("{error:?}"))?;
    Ok(format_feature(&feature, body))
}

/// Write the canonical form of `body` as spec/features/x.feature (it must
/// already be canonical), delete the scenarios tagged @drop and return the
/// file.
fn delete_dropped(body: &str) -> Result<String, Box<dyn std::error::Error>> {
    if canonical(body)? != body {
        return Err(format!("the fixture is not formatter-clean:\n{body}").into());
    }
    let root = tempfile::tempdir()?;
    write(root.path(), "spec/features/x.feature", body)?;
    let data = dispatch(root.path(), "delete-scenarios", json!({"tags": ["@drop"]}))?;
    if data["success"] != json!(true) {
        return Err(format!("delete-scenarios failed: {data}").into());
    }
    Ok(std::fs::read_to_string(
        root.path().join("spec/features/x.feature"),
    )?)
}

#[test]
fn deleting_features_deletes_their_coverage_files() -> TestResult {
    // @step Given feature "a" with a coverage file and feature "b" without one are tagged @gone, and feature "c" with a coverage file is not
    let root = tempfile::tempdir()?;
    feature(root.path(), "a", "@gone", true)?;
    feature(root.path(), "b", "@gone", false)?;
    feature(root.path(), "c", "@kept", true)?;
    // @step When I delete the features tagged @gone
    let data = dispatch(root.path(), "delete-features", json!({"tags": ["@gone"]}))?;
    // @step Then a.feature, its coverage file and b.feature are gone
    assert!(!exists(root.path(), "spec/features/a.feature"), "{data}");
    assert!(
        !exists(root.path(), "spec/features/a.feature.coverage"),
        "{data}"
    );
    assert!(!exists(root.path(), "spec/features/b.feature"), "{data}");
    // @step And c.feature and its coverage file are untouched
    assert!(exists(root.path(), "spec/features/c.feature"));
    assert_eq!(
        std::fs::read_to_string(root.path().join("spec/features/c.feature.coverage"))?,
        COVERAGE
    );
    // @step And the result lists spec/features/a.feature.coverage as the only deleted coverage file
    assert_eq!(
        data["coverageFiles"],
        json!(["spec/features/a.feature.coverage"]),
        "{data}"
    );
    assert_eq!(
        data["files"],
        json!(["spec/features/a.feature", "spec/features/b.feature"]),
        "{data}"
    );
    Ok(())
}

#[test]
fn a_dry_run_lists_the_coverage_files_it_would_delete() -> TestResult {
    // @step Given feature "a" with a coverage file is tagged @gone
    let root = tempfile::tempdir()?;
    feature(root.path(), "a", "@gone", true)?;
    // @step When I delete the features tagged @gone as a dry run
    let data = dispatch(
        root.path(),
        "delete-features",
        json!({"tags": ["@gone"], "dryRun": true}),
    )?;
    // @step Then the result lists spec/features/a.feature.coverage as a coverage file it would delete
    assert_eq!(
        data["coverageFiles"],
        json!(["spec/features/a.feature.coverage"]),
        "{data}"
    );
    // @step And a.feature and its coverage file are still on disk
    assert!(exists(root.path(), "spec/features/a.feature"));
    assert!(exists(root.path(), "spec/features/a.feature.coverage"));
    Ok(())
}

#[test]
fn deleting_the_last_scenario_leaves_a_formatter_clean_file() -> TestResult {
    // @step Given a formatter-clean feature whose last scenario "Two" is tagged @drop
    let body =
        "Feature: X\n\n  Scenario: One\n    Given a\n\n  @drop\n  Scenario: Two\n    Given b\n";
    // @step When I delete the scenarios tagged @drop
    let after = delete_dropped(body)?;
    // @step Then the file ends with the last step of scenario "One" followed by one newline
    assert_eq!(after, "Feature: X\n\n  Scenario: One\n    Given a\n");
    // @step And the file equals the formatter's output
    assert_eq!(canonical(&after)?, after);
    Ok(())
}

#[test]
fn deleting_the_last_two_scenarios_leaves_a_formatter_clean_file() -> TestResult {
    // @step Given a formatter-clean feature whose scenarios "Two" and "Three" after scenario "One" are tagged @drop
    let body = "Feature: X\n\n  Scenario: One\n    Given a\n    Then b\n\n  @drop\n  Scenario: Two\n    Given c\n\n  @drop\n  Scenario: Three\n    Given d\n";
    // @step When I delete the scenarios tagged @drop
    let after = delete_dropped(body)?;
    // @step Then the file ends with the last step of scenario "One" followed by one newline
    assert_eq!(
        after,
        "Feature: X\n\n  Scenario: One\n    Given a\n    Then b\n"
    );
    // @step And the file equals the formatter's output
    assert_eq!(canonical(&after)?, after);
    Ok(())
}

#[test]
fn deleting_every_scenario_leaves_a_formatter_clean_file() -> TestResult {
    // @step Given a formatter-clean feature with a Background whose only scenarios are tagged @drop
    let body = "Feature: X\n\n  Background: User Story\n    Given a user\n\n  @drop\n  Scenario: One\n    Given a\n\n  @drop\n  Scenario: Two\n    Given b\n";
    // @step When I delete the scenarios tagged @drop
    let after = delete_dropped(body)?;
    // @step Then the file ends with the last step of the Background followed by one newline
    assert_eq!(
        after,
        "Feature: X\n\n  Background: User Story\n    Given a user\n"
    );
    // @step And the file equals the formatter's output
    assert_eq!(canonical(&after)?, after);
    Ok(())
}

#[test]
fn a_rule_after_a_deleted_scenario_is_kept() -> TestResult {
    // @step Given a feature with scenario "One", scenario "Two" tagged @drop, then Rule "R" holding scenario "Three"
    let body = "Feature: X\n\n  Scenario: One\n    Given a\n\n  @drop\n  Scenario: Two\n    Given b\n\n  Rule: R\n\n    Scenario: Three\n      Given c\n";
    let root = tempfile::tempdir()?;
    write(root.path(), "spec/features/x.feature", body)?;
    // @step When I delete the scenarios tagged @drop
    let data = dispatch(root.path(), "delete-scenarios", json!({"tags": ["@drop"]}))?;
    let after = std::fs::read_to_string(root.path().join("spec/features/x.feature"))?;
    // @step Then scenario "Two" is gone
    assert_eq!(data["deletedCount"], json!(1), "{data}");
    assert!(!after.contains("Scenario: Two"), "{after}");
    // @step And scenario "One", Rule "R" and scenario "Three" are unchanged
    assert_eq!(
        after,
        "Feature: X\n\n  Scenario: One\n    Given a\n\n  Rule: R\n\n    Scenario: Three\n      Given c\n"
    );
    Ok(())
}
