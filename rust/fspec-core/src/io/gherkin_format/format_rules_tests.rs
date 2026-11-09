//! Feature: spec/features/format-features.feature
//!
//! The formatting rules of `fspec format <file>` (COV-071): each test writes
//! a feature file into a temporary project, runs the `format` command through
//! the dispatcher (the entry the CLI subcommand calls) and reads the file
//! back. One test file per feature (BUG-357).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use serde_json::{json, Value};

use crate::{dispatch_command, DispatchRequest};

fn dispatch(root: &Path, command: &str, args: Value) -> Value {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    assert!(result.success, "{command} failed: {result:?}");
    serde_json::from_str(&result.data).expect("data json")
}

/// A temporary project holding `spec/features/<name>` with `source`.
fn project_with(name: &str, source: &str) -> (tempfile::TempDir, String) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let rel = format!("spec/features/{name}");
    let abs = tmp.path().join(&rel);
    std::fs::create_dir_all(abs.parent().expect("parent")).expect("mkdir");
    std::fs::write(&abs, source).expect("write feature");
    (tmp, rel)
}

/// Runs `format` on one file and returns its new content.
fn format_file(root: &Path, rel: &str) -> String {
    let data = dispatch(root, "format", json!({ "file": rel }));
    assert_eq!(data["formattedCount"].as_u64(), Some(1), "{data}");
    std::fs::read_to_string(root.join(rel)).expect("read formatted")
}

fn line_with<'a>(out: &'a str, trimmed: &str) -> &'a str {
    out.lines()
        .find(|l| l.trim() == trimmed)
        .unwrap_or_else(|| panic!("no line {trimmed:?} in:\n{out}"))
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[test]
fn apply_consistent_indentation() {
    // @step Given I have a feature file with mixed indentation
    let src = "Feature: Mixed\n      Scenario: First\n  Given a\n        When b\n Then c\n\n Scenario: Second\n            Given d\n";
    let (tmp, rel) = project_with("mixed.feature", src);

    // @step When I run `fspec format spec/features/mixed.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then all scenarios should be indented by 2 spaces
    assert_eq!(indent_of(line_with(&out, "Scenario: First")), 2, "{out}");
    assert_eq!(indent_of(line_with(&out, "Scenario: Second")), 2, "{out}");

    // @step And all steps should be indented by 4 spaces from feature level
    for step in ["Given a", "When b", "Then c", "Given d"] {
        assert_eq!(indent_of(line_with(&out, step)), 4, "{step}:\n{out}");
    }
    assert_eq!(indent_of(line_with(&out, "Feature: Mixed")), 0, "{out}");
}

#[test]
fn preserve_doc_strings() {
    // @step Given I have a feature file with doc strings
    let src = "Feature: Docs\n\n  Scenario: A\n    Given the text:\n      \"\"\"\n      first line\n        indented line\n\n      after a blank line\n      \"\"\"\n    Then ok\n";
    let (tmp, rel) = project_with("docs.feature", src);

    // @step When I run `fspec format spec/features/docs.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the doc strings should be preserved exactly
    assert!(
        out.contains("      first line\n        indented line\n\n      after a blank line\n"),
        "{out}"
    );

    // @step And the triple quotes (""") should remain on their own lines
    let delimiters: Vec<&str> = out.lines().filter(|l| l.contains("\"\"\"")).collect();
    assert_eq!(delimiters, vec!["      \"\"\"", "      \"\"\""], "{out}");
}

#[test]
fn preserve_data_tables() {
    // @step Given I have a feature file with data tables
    let src = "Feature: Tables\n\n  Scenario: A\n    Given the users:\n      | name | role |\n      |Alexandra|admin|\n      | Bo |   viewer   |\n    Then ok\n";
    let (tmp, rel) = project_with("tables.feature", src);

    // @step When I run `fspec format spec/features/tables.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the data table structure should be preserved
    // @step And columns should be aligned properly
    assert!(
        out.contains(
            "    Given the users:\n      | name      | role   |\n      | Alexandra | admin  |\n      | Bo        | viewer |\n"
        ),
        "{out}"
    );
}

#[test]
fn format_scenario_outline_with_examples() {
    // @step Given I have a feature file with Scenario Outline and Examples table
    let src = "Feature: Outline\n\nScenario Outline: Eat\nGiven there are <start> cucumbers\nThen I have <left>\nExamples:\n|start|left|\n|12|7|\n|200|1|\n";
    let (tmp, rel) = project_with("outline.feature", src);

    // @step When I run `fspec format spec/features/outline.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the Scenario Outline should be indented by 2 spaces
    assert_eq!(
        indent_of(line_with(&out, "Scenario Outline: Eat")),
        2,
        "{out}"
    );

    // @step And the Examples section should be indented by 4 spaces
    assert_eq!(indent_of(line_with(&out, "Examples:")), 4, "{out}");

    // @step And the Examples table should be indented by 6 spaces
    // @step And table columns should be aligned with padding
    assert!(
        out.contains("    Examples:\n      | start | left |\n      | 12    | 7    |\n      | 200   | 1    |\n"),
        "{out}"
    );
}

#[test]
fn format_rule_keyword() {
    // @step Given I have a feature file with Rule containing nested Background and Scenarios
    let src = "Feature: Rules\n\nRule: Business rule\nBackground:\nGiven a setup\nScenario: First\nGiven a\nThen b\nScenario: Second\nWhen c\n";
    let (tmp, rel) = project_with("rules.feature", src);

    // @step When I run `fspec format spec/features/rules.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the Rule should be indented by 2 spaces
    assert_eq!(
        indent_of(line_with(&out, "Rule: Business rule")),
        2,
        "{out}"
    );

    // @step And nested Background should be indented by 4 spaces
    assert_eq!(indent_of(line_with(&out, "Background:")), 4, "{out}");

    // @step And nested Scenarios should be indented by 4 spaces
    assert_eq!(indent_of(line_with(&out, "Scenario: First")), 4, "{out}");
    assert_eq!(indent_of(line_with(&out, "Scenario: Second")), 4, "{out}");

    // @step And steps should be indented by 6 spaces
    for step in ["Given a setup", "Given a", "Then b", "When c"] {
        assert_eq!(indent_of(line_with(&out, step)), 6, "{step}:\n{out}");
    }
}

#[test]
fn format_doc_strings_with_content_types() {
    // @step Given I have a feature file with DocStrings marked as """ json
    let src = "Feature: Typed docs\n\n  Scenario: Quotes\n    Given the payload:\n      \"\"\"json\n      {\"a\": 1}\n      \"\"\"\n\n  Scenario: Backticks\n    Given the payload:\n      ```json\n      {\"b\": 2}\n      ```\n";
    let (tmp, rel) = project_with("docstrings.feature", src);

    // @step When I run `fspec format spec/features/docstrings.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the DocString delimiter should include the content type
    let opening: Vec<&str> = out.lines().filter(|l| l.ends_with("json")).collect();
    assert_eq!(
        opening,
        vec!["      \"\"\"json", "      \"\"\"json"],
        "{out}"
    );

    // @step And the content should be preserved exactly
    assert!(
        out.contains("      \"\"\"json\n      {\"a\": 1}\n      \"\"\"\n"),
        "{out}"
    );

    // @step And both """ and ``` delimiters should be supported
    // A ``` doc string is accepted and written in the canonical """ form.
    assert!(
        out.contains("      \"\"\"json\n      {\"b\": 2}\n      \"\"\"\n"),
        "{out}"
    );
    assert!(!out.contains("```"), "{out}");
}

#[test]
fn format_multiple_feature_level_tags() {
    // @step Given I have a feature with tags @cli @formatter
    let src = "  @cli @formatter\nFeature: Tags\n\n  Scenario: A\n    Given a\n";
    let (tmp, rel) = project_with("tags.feature", src);

    // @step When I run `fspec format spec/features/tags.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then each tag should be on its own line
    // @step And tags should have zero indentation
    // @step And tag order should be preserved
    assert!(
        out.starts_with("@cli\n@formatter\nFeature: Tags\n"),
        "{out}"
    );
}

#[test]
fn format_scenario_level_tags() {
    // @step Given I have a feature file with scenario-level tags:
    let src = "Feature: User Authentication\n  @smoke @regression\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should be logged in\n";
    let (tmp, rel) = project_with("authentication.feature", src);

    // @step When I run `fspec format spec/features/authentication.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then scenario tags should be on separate lines
    // @step And scenario tags should be indented by 2 spaces
    // @step And tag order should be preserved
    // @step And the formatted output should be:
    assert_eq!(
        out,
        "Feature: User Authentication\n\n  @smoke\n  @regression\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should be logged in\n"
    );
}

#[test]
fn format_both_feature_level_and_scenario_level_tags() {
    // @step Given I have a feature file with tags at both levels:
    let src = "@authentication\nFeature: User Authentication\n  @smoke\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should be logged in\n  @regression @edge-case\n  Scenario: Login with expired session\n    Given I have an expired session\n    When I attempt to login\n    Then I should be prompted to re-authenticate\n";
    let (tmp, rel) = project_with("authentication.feature", src);

    // @step When I run `fspec format spec/features/authentication.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then feature-level tags should have zero indentation
    assert!(
        out.starts_with("@authentication\nFeature: User Authentication\n"),
        "{out}"
    );

    // @step And scenario-level tags should be indented by 2 spaces
    // @step And each tag should be on its own line
    // @step And the formatted output should preserve tag hierarchy
    assert!(
        out.contains("\n  @smoke\n  Scenario: Login with valid credentials\n"),
        "{out}"
    );
    assert!(
        out.contains("\n  @regression\n  @edge-case\n  Scenario: Login with expired session\n"),
        "{out}"
    );
}

#[test]
fn format_multiple_scenario_tags_on_same_line() {
    // @step Given I have a feature file with multiple scenario tags on one line:
    let src = "Feature: Notifications\n\n  @email @sms @push\n  Scenario: Send multi-channel notification\n    Given I have a message\n    When I send notification\n    Then user receives it via all channels\n";
    let (tmp, rel) = project_with("notifications.feature", src);

    // @step When I run `fspec format spec/features/notifications.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then each scenario tag should be separated onto its own line
    // @step And all scenario tags should be indented by 2 spaces
    // @step And the formatted output should be:
    assert_eq!(
        out,
        "Feature: Notifications\n\n  @email\n  @sms\n  @push\n  Scenario: Send multi-channel notification\n    Given I have a message\n    When I send notification\n    Then user receives it via all channels\n"
    );
}

#[test]
fn format_and_but_step_keywords() {
    // @step Given I have a feature file with And and But steps
    let src = "Feature: Steps\n\n  Scenario: A\n Given a\n        And b\n   When c\n  Then d\n          But e\n";
    let (tmp, rel) = project_with("steps.feature", src);

    // @step When I run `fspec format spec/features/steps.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then And steps should be formatted like other steps
    assert_eq!(line_with(&out, "And b"), "    And b", "{out}");

    // @step And But steps should be formatted like other steps
    assert_eq!(line_with(&out, "But e"), "    But e", "{out}");

    // @step And all steps should be indented by 4 spaces
    assert!(
        out.contains("    Given a\n    And b\n    When c\n    Then d\n    But e\n"),
        "{out}"
    );
}

#[test]
fn format_wildcard_step_keyword() {
    // @step Given I have a feature file with * step keyword
    let src = "Feature: Wildcard\n\n  Scenario: A\n    Given a\n        * b\n * c\n";
    let (tmp, rel) = project_with("wildcard.feature", src);

    // @step When I run `fspec format spec/features/wildcard.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then the * steps should be formatted correctly
    // @step And indentation should match other steps
    assert!(out.contains("    Given a\n    * b\n    * c\n"), "{out}");
}

#[test]
fn limit_excessive_blank_lines() {
    // @step Given I have a feature file with 6 consecutive blank lines in description
    let src = "Feature: Excessive\n  First paragraph.\n\n\n\n\n\n\n  Second paragraph.\n\n  Scenario: A\n    Given a\n";
    let (tmp, rel) = project_with("excessive.feature", src);

    // @step When I run `fspec format spec/features/excessive.feature`
    let out = format_file(tmp.path(), &rel);

    // @step Then consecutive blank lines should be limited to maximum 2
    assert!(!out.contains("\n\n\n\n"), "more than 2 blank lines:\n{out}");
    assert!(out.contains("First paragraph.\n"), "{out}");
    assert!(out.contains("Second paragraph.\n"), "{out}");

    // @step And the file should pass validation
    let data = dispatch(tmp.path(), "validate", json!({ "file": rel }));
    assert_eq!(data["invalidCount"].as_u64(), Some(0), "{data}");
}
