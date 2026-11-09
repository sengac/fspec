//! Feature: spec/features/gherkin-stepless-scenario-formatting.feature
//!
//! BUG-158 formatter tests, moved out of gherkin_format.rs's inline tests
//! (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use gherkin::GherkinEnv;

fn parse(src: &str) -> Feature {
    Feature::parse(src, GherkinEnv::default()).expect("parse")
}

/// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
#[test]
fn bug158_stepless_scenario_does_not_duplicate_scenarios() {
    // @step Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario with steps
    let src = "Feature: Step-less scenario\n\n  Scenario: First\n    just some prose here\n\n  Scenario: Second\n    Given a precondition\n    When I do the action\n    Then I see the result\n";

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

/// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
#[test]
fn bug158_stepless_scenario_formatting_is_idempotent() {
    // @step Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario
    let src = "Feature: Step-less idempotent\n\n  Scenario: First\n    just some prose here\n\n  Scenario: Second\n    Given a precondition\n    When I do the action\n    Then I see the result\n";
    let once = format_feature(&parse(src), src);

    // @step When the formatter formats the file twice
    let twice = format_feature(&parse(&once), &once);

    // @step Then the output of the second run is byte-identical to the output of the first run
    assert_eq!(
        once, twice,
        "formatting must be idempotent, got:\n{once}\n---\n{twice}"
    );
}

/// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
#[test]
fn bug158_lowercase_step_keywords_are_formatted_without_duplication() {
    // @step Given a feature file whose scenarios use lowercase step keywords (given, when, then)
    let src = "Feature: Lowercase keywords\n\n  Scenario: One\n    given a precondition\n    when I do the action\n    then I see the result\n\n  Scenario: Two\n    given another precondition\n    when I do another action\n    then I see another result\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the formatted output contains exactly the original number of Scenario lines
    let scenario_count = out
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count();
    assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");

    // @step And the lowercase step lines are preserved verbatim under their scenario
    assert!(
        out.contains("    given a precondition"),
        "lowercase step line was lost or rewritten, got:\n{out}"
    );
    assert!(
        out.contains("    then I see another result"),
        "lowercase step line was lost or rewritten, got:\n{out}"
    );
}

/// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
#[test]
fn bug158_rule_with_stepless_scenario_is_formatted_without_duplication() {
    // @step Given a feature file with a Rule containing a prose-only scenario followed by a second scenario
    let src = "Feature: Rule with step-less scenario\n\n  Rule: My rule\n    Scenario: First\n      just some prose here\n\n    Scenario: Second\n      Given a precondition\n      Then I see the result\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then both scenarios are emitted exactly once at the Rule nesting level
    let scenario_count = out
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count();
    assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");
    assert!(
        out.contains("    Scenario: First"),
        "first scenario must be 4-space Rule-nested, got:\n{out}"
    );
    assert!(
        out.contains("    Scenario: Second"),
        "second scenario must be 4-space Rule-nested, got:\n{out}"
    );
}
