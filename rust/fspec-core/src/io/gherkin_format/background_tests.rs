//! Feature: spec/features/gherkin-background-formatting.feature
//!
//! BUG-157 formatter tests, moved out of gherkin_format.rs's inline tests
//! (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use gherkin::GherkinEnv;

fn parse(src: &str) -> Feature {
    Feature::parse(src, GherkinEnv::default()).expect("parse")
}

// ====================================================================
// BUG-157: Background section must not duplicate scenarios
//
// Feature: spec/features/gherkin-background-formatting.feature
//
// The gherkin-0.16.0 parser stores Background prose in
// `Background.description`, so the formatter must NOT re-extract the
// Background description verbatim from raw source without bounding the
// extraction by the Background's own content. An unbounded extraction
// swallows the trailing top-level scenarios and re-emits them nested
// under the Background, duplicating every scenario block.
// ====================================================================

/// Feature: spec/features/gherkin-background-formatting.feature
#[test]
fn bug157_background_prose_does_not_duplicate_scenarios() {
    // @step Given a feature file with a Background section followed by two scenarios
    let src = "@test\nFeature: Formatter Background duplication repro\n\n  Background: User Story\n    As a user\n    I want to test\n    So that I can verify\n\n  Scenario: First scenario\n    Given a precondition\n    When I do the action\n    Then I see the result\n\n  Scenario: Second scenario\n    Given another precondition\n    When I do another action\n    Then I see another result\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the formatted output contains exactly two Scenario lines
    let scenario_count = out
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count();
    assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");

    // @step And every scenario is indented at the top level (two spaces)
    for line in out.lines() {
        if line.trim_start().starts_with("Scenario:") {
            assert_eq!(
                line,
                format!("  {}", line.trim_start()),
                "scenario must be 2-space top-level indented, got:\n{out}"
            );
        }
    }
}

/// Feature: spec/features/gherkin-background-formatting.feature
#[test]
fn bug157_background_prose_formatting_is_idempotent() {
    // @step Given a feature file with a Background section and two scenarios
    let src = "Feature: Background idempotent\n\n  Background: User Story\n    As a user\n    I want to test\n    So that I can verify\n\n  Scenario: First scenario\n    Given a precondition\n\n  Scenario: Second scenario\n    Given another precondition\n";
    let once = format_feature(&parse(src), src);

    // @step When the formatter formats the file twice
    let twice = format_feature(&parse(&once), &once);

    // @step Then the output of the second run is byte-identical to the output of the first run
    assert_eq!(
        once, twice,
        "formatting must be idempotent, got:\n{once}\n---\n{twice}"
    );
}

/// Feature: spec/features/gherkin-background-formatting.feature
#[test]
fn bug157_background_with_steps_is_formatted() {
    // @step Given a feature file with a Background section containing a Given step and one scenario
    let src = "Feature: Background with steps\n\n  Background: Setup\n    Given the app is loaded\n\n  Scenario: A scenario\n    Given a precondition\n    Then I see the result\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the Background step is emitted under the Background
    assert!(
        out.contains("  Background: Setup\n    Given the app is loaded"),
        "background step missing, got:\n{out}"
    );

    // @step And the scenario is emitted exactly once at the top level
    let scenario_count = out
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count();
    assert_eq!(scenario_count, 1, "scenario was duplicated, got:\n{out}");
    assert!(
        out.contains("  Scenario: A scenario"),
        "scenario must be 2-space indented, got:\n{out}"
    );
}

/// Feature: spec/features/gherkin-background-formatting.feature
#[test]
fn bug157_rule_with_background_is_formatted_without_duplication() {
    // @step Given a feature file with a Rule containing a Background and one scenario
    let src = "Feature: Rule with background\n\n  Rule: My rule\n    Background: Setup\n      Given the app is loaded\n\n    Scenario: A scenario\n      Given a precondition\n      Then I see the result\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the scenario is emitted exactly once
    let scenario_count = out
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count();
    assert_eq!(scenario_count, 1, "scenario was duplicated, got:\n{out}");

    // @step And the Background step is emitted under the Rule's Background
    assert!(
        out.contains("    Background: Setup\n      Given the app is loaded"),
        "rule background step missing, got:\n{out}"
    );
}
