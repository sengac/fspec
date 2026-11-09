//! CLI-018 — list-work-units --tag matches a tag on the Feature line or on
//! any scenario of a card's feature files.
//!
//! Feature: spec/features/list-work-units-tag-matches-scenario-tags.feature
//!
//! Every query runs the core command on a fixture board with a fixed clock
//! and a named zone (`run_at`), never the wall clock (TEST-042).

#[path = "cli017_board_queries/support.rs"]
#[allow(dead_code)]
mod support;

use serde_json::json;
use support::*;

/// A board holding one backlog story `id` and the feature file `name` with
/// `content`.
fn board_with_feature(id: &str, name: &str, content: &str) -> Result<tempfile::TempDir, Failure> {
    let dir = board(&[card(id, "story", "backlog", json!({}))])?;
    let features = dir.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    std::fs::write(features.join(name), content)?;
    Ok(dir)
}

/// `--tag @cli --format ids` on the board in `dir`.
fn tag_cli(dir: &tempfile::TempDir) -> Result<String, Failure> {
    ok(
        dir.path(),
        json!({"tag": "@cli", "format": "ids"}),
        &brisbane()?,
    )
}

#[test]
fn a_tag_on_one_scenario_of_a_cards_feature_file_selects_the_card() -> TestResult {
    // @step Given spec/features/a.feature carries @CLI-001 on its Feature line and @cli only on one scenario
    let dir = board_with_feature(
        "CLI-001",
        "a.feature",
        "@CLI-001\nFeature: A\n\n  Scenario: Plain\n    Given x\n\n  @cli\n  Scenario: Tagged\n    Given y\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let out = tag_cli(&dir)?;
    // @step Then the output is the ids CLI-001
    assert_eq!(ids(&out), strings(&["CLI-001"]), "{out}");
    Ok(())
}

#[test]
fn a_tag_that_is_only_text_in_the_feature_file_does_not_select_the_card() -> TestResult {
    // @step Given spec/features/b.feature carries @CLI-002 and @tui on its Feature line and @cli only in its description and in a step
    let dir = board_with_feature(
        "CLI-002",
        "b.feature",
        "@CLI-002 @tui\nFeature: B\n  \"\"\"\n  Cards tagged @cli are CLI cards.\n  \"\"\"\n\n  # @cli\n  Scenario: S\n    Given the tag @cli\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let out = tag_cli(&dir)?;
    // @step Then the output is no ids
    assert_eq!(ids(&out), Vec::<String>::new(), "{out}");
    Ok(())
}

#[test]
fn tags_match_exactly() -> TestResult {
    // @step Given spec/features/c.feature carries @CLI-003 on its Feature line and @cli-tools on a scenario
    let dir = board_with_feature(
        "CLI-003",
        "c.feature",
        "@CLI-003\nFeature: C\n\n  @cli-tools\n  Scenario: S\n    Given x\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let out = tag_cli(&dir)?;
    // @step Then the output is no ids
    assert_eq!(ids(&out), Vec::<String>::new(), "{out}");
    Ok(())
}

#[test]
fn a_tag_on_a_scenario_outline_inside_a_rule_selects_the_card() -> TestResult {
    // @step Given spec/features/d.feature carries @CLI-004 on its Feature line and @cli on a Scenario Outline inside a Rule
    let dir = board_with_feature(
        "CLI-004",
        "d.feature",
        "@CLI-004\nFeature: D\n\n  Rule: R\n\n    @cli\n    Scenario Outline: O\n      Given <n>\n\n      Examples:\n        | n |\n        | 1 |\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let out = tag_cli(&dir)?;
    // @step Then the output is the ids CLI-004
    assert_eq!(ids(&out), strings(&["CLI-004"]), "{out}");
    Ok(())
}

#[test]
fn a_feature_file_that_does_not_parse_is_an_error_naming_it() -> TestResult {
    // @step Given spec/features/e.feature carries @CLI-005 on its Feature line and does not parse as Gherkin
    let dir = board_with_feature(
        "CLI-005",
        "e.feature",
        "@CLI-005\nFeature: E\n\n  Scenario: S\n    Given x\n  Scenario S2 without a colon\n    Given y\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let error = err(
        dir.path(),
        json!({"tag": "@cli", "format": "ids"}),
        &brisbane()?,
    )?;
    // @step Then the query fails with an error naming spec/features/e.feature
    assert!(error.contains("spec/features/e.feature"), "{error}");
    Ok(())
}
