#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-step-follows-gherkin-order-and-add-scenario-leaves-formatter-clean-files.feature
//
// BUG-620: add-step places a step where Gherkin order puts it (repeated
// keywords, And/But, data tables, a Background), and add-scenario leaves a
// file that fspec's own Gherkin formatter does not change.

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::io::gherkin::parse_feature_lenient;
use codelet_fspec_core::io::gherkin_format::format_feature;
use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

const FEATURE: &str = "spec/features/login.feature";
const WHEN_PLACEHOLDER: &str = "When [action]";
const THEN_PLACEHOLDER: &str = "Then [expected outcome]";

/// A project holding one feature file whose scenario 'Log in' has `steps`
/// (each a full line body such as "Given a"), after an optional Background.
fn project_with(background: Option<&str>, steps: &[&str]) -> (TempDir, PathBuf) {
    let mut content = String::from("@cli\nFeature: Login\n\n");
    if let Some(step) = background {
        content.push_str(&format!("  Background:\n    {step}\n\n"));
    }
    content.push_str("  Scenario: Log in\n");
    for step in steps {
        content.push_str(&format!("    {step}\n"));
    }
    project_with_content(&content)
}

fn project_with_content(content: &str) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join(FEATURE);
    fs::create_dir_all(path.parent().expect("features dir")).expect("mkdir features");
    fs::write(&path, content).expect("write feature");
    (tmp, path)
}

fn dispatch(project_root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

/// Run add-step and return its JSON envelope.
fn add_step(project_root: &Path, step_type: &str, text: &str) -> Value {
    let result = dispatch(
        project_root,
        "add-step",
        json!({"feature": FEATURE, "scenario": "Log in", "type": step_type, "text": text}),
    );
    assert!(result.success, "add-step dispatch failed: {result:?}");
    serde_json::from_str(&result.data).expect("add-step data is JSON")
}

fn add_step_ok(project_root: &Path, step_type: &str, text: &str) {
    let envelope = add_step(project_root, step_type, text);
    assert_eq!(envelope["success"], true, "add-step failed: {envelope}");
    assert_eq!(
        envelope["valid"], true,
        "add-step left invalid Gherkin: {envelope}"
    );
}

/// The steps of scenario `name` as "Keyword text".
fn steps_of(path: &Path, name: &str) -> Vec<String> {
    let content = fs::read_to_string(path).expect("read feature");
    let feature = parse_feature_lenient(&content).expect("feature parses");
    let scenario = feature
        .scenarios
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no scenario {name} in:\n{content}"));
    scenario
        .steps
        .iter()
        .map(|s| format!("{} {}", s.keyword.trim(), s.value))
        .collect()
}

fn assert_formatter_clean(path: &Path) {
    let content = fs::read_to_string(path).expect("read feature");
    let feature = parse_feature_lenient(&content).expect("feature parses");
    assert_eq!(
        format_feature(&feature, &content),
        content,
        "not formatter-clean"
    );
}

#[test]
fn a_second_given_step_goes_after_the_first_and_before_the_when_block() {
    // Scenario: A second Given step goes after the first one and before the When block
    // @step Given a feature whose Background has the step 'Given the app is running' and whose scenario 'Log in' has the three placeholder steps
    let (tmp, path) = project_with(
        Some("Given the app is running"),
        &["Given [precondition]", WHEN_PLACEHOLDER, THEN_PLACEHOLDER],
    );

    // @step When I add the given step 'I am on the login page' and then the given step 'I have an account'
    add_step_ok(tmp.path(), "given", "I am on the login page");
    add_step_ok(tmp.path(), "given", "I have an account");

    // @step Then the steps of 'Log in' are 'Given I am on the login page', 'Given I have an account', the When placeholder, the Then placeholder
    assert_eq!(
        steps_of(&path, "Log in"),
        [
            "Given I am on the login page",
            "Given I have an account",
            WHEN_PLACEHOLDER,
            THEN_PLACEHOLDER
        ]
    );

    // @step And the Background still has only the step 'Given the app is running'
    let content = fs::read_to_string(&path).expect("read feature");
    let feature = parse_feature_lenient(&content).expect("feature parses");
    let background = feature.background.expect("background");
    let steps: Vec<String> = background
        .steps
        .iter()
        .map(|s| format!("{} {}", s.keyword.trim(), s.value))
        .collect();
    assert_eq!(steps, ["Given the app is running"]);
}

#[test]
fn an_and_step_continues_the_last_real_step_not_the_placeholder_then() {
    // Scenario: An And step continues the last real step, not the placeholder Then
    // @step Given the scenario 'Log in' has the steps 'Given I am on the login page', the When placeholder, the Then placeholder
    let (tmp, path) = project_with(
        None,
        &[
            "Given I am on the login page",
            WHEN_PLACEHOLDER,
            THEN_PLACEHOLDER,
        ],
    );

    // @step When I add the and step 'I have an account'
    add_step_ok(tmp.path(), "and", "I have an account");

    // @step Then the steps of 'Log in' are 'Given I am on the login page', 'And I have an account', the When placeholder, the Then placeholder
    assert_eq!(
        steps_of(&path, "Log in"),
        [
            "Given I am on the login page",
            "And I have an account",
            WHEN_PLACEHOLDER,
            THEN_PLACEHOLDER
        ]
    );
}

#[test]
fn a_when_step_goes_after_the_when_blocks_last_step_even_a_but() {
    // Scenario: A When step goes after the When block's last step even when that step is a But
    // @step Given the scenario 'Log in' has the steps 'Given a', 'When b', 'But c', 'Then d'
    let (tmp, path) = project_with(None, &["Given a", "When b", "But c", "Then d"]);

    // @step When I add the when step 'e'
    add_step_ok(tmp.path(), "when", "e");

    // @step Then the steps of 'Log in' are 'Given a', 'When b', 'But c', 'When e', 'Then d'
    assert_eq!(
        steps_of(&path, "Log in"),
        ["Given a", "When b", "But c", "When e", "Then d"]
    );
}

#[test]
fn a_step_never_lands_between_a_step_and_its_data_table() {
    // Scenario: A step never lands between a step and its data table
    // @step Given the scenario 'Log in' has the step 'Given these users:' with a data table, then the Then placeholder
    let (tmp, path) = project_with(
        None,
        &[
            "Given these users:",
            "  | name  |",
            "  | alice |",
            THEN_PLACEHOLDER,
        ],
    );

    // @step When I add the given step 'I am an admin'
    add_step_ok(tmp.path(), "given", "I am an admin");

    // @step Then the steps of 'Log in' are 'Given these users:', 'Given I am an admin', the Then placeholder
    assert_eq!(
        steps_of(&path, "Log in"),
        [
            "Given these users:",
            "Given I am an admin",
            THEN_PLACEHOLDER
        ]
    );

    // @step And 'Given these users:' still has its data table
    let content = fs::read_to_string(&path).expect("read feature");
    let feature = parse_feature_lenient(&content).expect("feature parses");
    let table = feature.scenarios[0].steps[0]
        .table
        .as_ref()
        .expect("the first step keeps its table");
    assert_eq!(table.rows, [["name"], ["alice"]]);
    assert!(feature.scenarios[0].steps[1].table.is_none());
}

#[test]
fn a_given_step_goes_before_the_when_block_without_a_given_step() {
    // Scenario: A Given step goes before the When block when the scenario has no Given step
    // @step Given the scenario 'Log in' has the steps 'When b', 'Then d'
    let (tmp, path) = project_with(None, &["When b", "Then d"]);

    // @step When I add the given step 'a'
    add_step_ok(tmp.path(), "given", "a");

    // @step Then the steps of 'Log in' are 'Given a', 'When b', 'Then d'
    assert_eq!(steps_of(&path, "Log in"), ["Given a", "When b", "Then d"]);
}

#[test]
fn an_and_step_with_no_real_step_fails_with_an_exact_error() {
    // Scenario: An And step with no real step to continue fails with an exact error
    // @step Given the scenario 'Log in' has the three placeholder steps
    let (tmp, path) = project_with(
        None,
        &["Given [precondition]", WHEN_PLACEHOLDER, THEN_PLACEHOLDER],
    );
    let before = fs::read_to_string(&path).expect("read feature");

    // @step When I add the and step 'x'
    let envelope = add_step(tmp.path(), "and", "x");

    // @step Then add-step fails with 'Cannot add an And step to scenario "Log in": it has no real step to continue. Add a given, when or then step first.'
    assert_eq!(envelope["success"], false, "expected failure: {envelope}");
    assert_eq!(
        envelope["error"],
        "Cannot add an And step to scenario \"Log in\": it has no real step to continue. Add a given, when or then step first."
    );

    // @step And the feature file is unchanged
    assert_eq!(fs::read_to_string(&path).expect("read feature"), before);
}

#[test]
fn add_scenario_at_the_end_of_a_file_leaves_it_formatter_clean() {
    // Scenario: add-scenario at the end of a file leaves it formatter-clean
    // @step Given a formatter-clean feature file that ends with one newline
    let (tmp, path) = project_with(None, &["Given a", "When b", "Then c"]);
    assert_formatter_clean(&path);

    // @step When I add the scenario 'New one'
    let result = dispatch(
        tmp.path(),
        "add-scenario",
        json!({"feature": FEATURE, "scenario": "New one"}),
    );
    assert!(result.success, "add-scenario failed: {result:?}");

    // @step Then the file equals fspec's formatted form of it
    assert_formatter_clean(&path);

    // @step And the file ends with exactly one newline
    let content = fs::read_to_string(&path).expect("read feature");
    assert!(
        content.ends_with("]\n") && !content.ends_with("\n\n"),
        "bad file end: {content:?}"
    );
}

#[test]
fn add_scenario_before_a_tagged_outline_keeps_the_outline_tags() {
    // Scenario: add-scenario before a tagged Scenario Outline keeps the outline's tags
    // @step Given a formatter-clean feature file whose last scenario is a Scenario Outline tagged @slow
    let (tmp, path) = project_with_content(
        "@cli\nFeature: Login\n\n  Scenario: Log in\n    Given a\n\n  @slow\n  Scenario Outline: Many\n    Given <n>\n\n    Examples:\n      | n |\n      | 1 |\n",
    );
    assert_formatter_clean(&path);

    // @step When I add the scenario 'New one'
    let result = dispatch(
        tmp.path(),
        "add-scenario",
        json!({"feature": FEATURE, "scenario": "New one"}),
    );
    assert!(result.success, "add-scenario failed: {result:?}");

    // @step Then the scenario 'New one' has no tags and the Scenario Outline still has the tag @slow
    let content = fs::read_to_string(&path).expect("read feature");
    let feature = parse_feature_lenient(&content).expect("feature parses");
    let tags_of = |name: &str| {
        feature
            .scenarios
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.tags.clone())
            .unwrap_or_else(|| panic!("no scenario {name} in:\n{content}"))
    };
    assert!(tags_of("New one").is_empty(), "file:\n{content}");
    assert_eq!(tags_of("Many"), ["slow"], "file:\n{content}");

    // @step And the file equals fspec's formatted form of it
    assert_formatter_clean(&path);
}
