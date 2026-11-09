//! BUG-502 — link-coverage replaces a test mapping in place.
//!
//! Feature: spec/features/link-coverage-replaces-a-test-mapping-in-place.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

/// The @step comments are on lines 4 to 6.
const TEST_FILE: &str = "\
// helper line 1
// helper line 2
fn log_in() {
    // @step Given a user
    // @step When they log in
    // @step Then they see the board
}
";

const TEST: &str = "tests/login_test.rs";
const IMPL: &str = "src/login.rs";

fn project(mappings: Value) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/login.feature", FEATURE)?;
    write(TEST, TEST_FILE)?;
    write(
        IMPL,
        &(1..=20)
            .map(|n| format!("// impl {n}\n"))
            .collect::<String>(),
    )?;
    write(
        "spec/features/login.feature.coverage",
        &json!({
            "scenarios": [{"name": "Log in", "testMappings": mappings}],
            // unlink-coverage refuses a coverage file without stats.
            "stats": {"totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
                "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
        })
        .to_string(),
    )?;
    Ok(root)
}

fn run(root: &Path, command: &str, args: Value) -> DispatchResult {
    let mut args = args;
    args["featureName"] = json!("login");
    args["scenario"] = json!("Log in");
    args["testFile"] = json!(TEST);
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn rows(root: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(root.join("spec/features/login.feature.coverage"))?;
    let coverage: Value = serde_json::from_str(&raw)?;
    coverage["scenarios"][0]["testMappings"]
        .as_array()
        .cloned()
        .ok_or_else(|| "no testMappings array".into())
}

#[test]
fn replacing_a_test_mapping_keeps_its_implementation_mappings() -> TestResult {
    // @step Given scenario "Log in" maps test lines 1-3 carrying an implementation mapping
    let root = project(json!([{"file": TEST, "lines": "1-3",
        "implMappings": [{"file": IMPL, "lines": [5, 6]}]}]))?;
    // @step When link-coverage is run with --replace and test lines 4-6
    let result = run(
        root.path(),
        "link-coverage",
        json!({"testLines": "4-6", "replace": true}),
    );
    assert!(result.success, "{result:?}");
    // @step Then the scenario has one test mapping for lines 4-6
    let rows = rows(root.path())?;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["lines"], "4-6", "{rows:?}");
    // @step And that mapping still carries the implementation mapping
    assert_eq!(
        rows[0]["implMappings"],
        json!([{"file": IMPL, "lines": [5, 6]}]),
        "{rows:?}"
    );
    Ok(())
}

#[test]
fn a_replacement_range_that_lacks_the_steps_is_refused() -> TestResult {
    // @step Given scenario "Log in" maps test lines 4-6 holding all its @step comments
    let root = project(json!([{"file": TEST, "lines": "4-6", "implMappings": []}]))?;
    // @step When link-coverage is run with --replace and test lines 1-2
    let result = run(
        root.path(),
        "link-coverage",
        json!({"testLines": "1-2", "replace": true}),
    );
    // @step Then link-coverage fails naming the missing steps
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(text.contains("Given a user"), "{text}");
    // @step And the mapping for lines 4-6 is unchanged
    let rows = rows(root.path())?;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["lines"], "4-6", "{rows:?}");
    Ok(())
}

#[test]
fn replacing_a_test_file_the_scenario_does_not_map_is_refused() -> TestResult {
    // @step Given scenario "Log in" has no test mapping
    let root = project(json!([]))?;
    // @step When link-coverage is run with --replace and test lines 4-6
    let result = run(
        root.path(),
        "link-coverage",
        json!({"testLines": "4-6", "replace": true}),
    );
    // @step Then link-coverage fails saying there is no mapping of that test file to replace
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(
        text.contains("no mapping of tests/login_test.rs to replace"),
        "{text}"
    );
    assert!(rows(root.path())?.is_empty());
    Ok(())
}

#[test]
fn linking_without_replace_still_adds_a_second_range() -> TestResult {
    // @step Given scenario "Log in" maps test lines 4-4 holding its Given step
    let root = project(json!([{"file": TEST, "lines": "4-4", "implMappings": []}]))?;
    // @step When link-coverage is run without --replace for test lines 5-6
    let result = run(root.path(), "link-coverage", json!({"testLines": "5-6"}));
    assert!(result.success, "{result:?}");
    // @step Then the scenario has two test mappings for that test file
    let rows = rows(root.path())?;
    let lines: Vec<&Value> = rows.iter().map(|row| &row["lines"]).collect();
    assert_eq!(lines, [&json!("4-4"), &json!("5-6")], "{rows:?}");
    Ok(())
}

#[test]
fn relinking_an_implementation_file_replaces_its_lines() -> TestResult {
    // @step Given scenario "Log in" maps test lines 4-6 carrying an implementation mapping of lines 5 and 6
    let root = project(json!([{"file": TEST, "lines": "4-6",
        "implMappings": [{"file": IMPL, "lines": [5, 6]}]}]))?;
    // @step When the same implementation file is linked with lines 10-11
    let result = run(
        root.path(),
        "link-coverage",
        json!({"implFile": IMPL, "implLines": "10-11"}),
    );
    assert!(result.success, "{result:?}");
    // @step Then the test mapping carries that implementation file once with lines 10 and 11
    let rows = rows(root.path())?;
    assert_eq!(
        rows[0]["implMappings"],
        json!([{"file": IMPL, "lines": [10, 11], "firstLine": "// impl 10", "lastLine": "// impl 11"}]),
        "{rows:?}"
    );
    Ok(())
}

#[test]
fn unlinking_an_implementation_mapping_from_the_second_mapping_of_a_test_file() -> TestResult {
    // @step Given scenario "Log in" has two mappings of one test file and only the second carries an implementation mapping
    let root = project(json!([
        {"file": TEST, "lines": "4-4", "implMappings": []},
        {"file": TEST, "lines": "5-6", "implMappings": [{"file": IMPL, "lines": [5, 6]}]}
    ]))?;
    // @step When unlink-coverage is run with that test file and implementation file
    let result = run(root.path(), "unlink-coverage", json!({"implFile": IMPL}));
    assert!(result.success, "{result:?}");
    // @step Then the implementation mapping is removed
    let rows = rows(root.path())?;
    assert!(
        rows.iter()
            .all(|row| row["implMappings"].as_array().is_some_and(Vec::is_empty)),
        "{rows:?}"
    );
    // @step And both test mappings remain
    let lines: Vec<&Value> = rows.iter().map(|row| &row["lines"]).collect();
    assert_eq!(lines, [&json!("4-4"), &json!("5-6")], "{rows:?}");
    Ok(())
}
