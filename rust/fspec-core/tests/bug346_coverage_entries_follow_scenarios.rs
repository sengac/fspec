//! BUG-346 — add-scenario adds a coverage entry, and coverage writes derive
//! totalScenarios from the scenario list.
//!
//! Feature: spec/features/coverage-entries-follow-scenarios.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

const FEATURE: &str = "@cli\nFeature: Login\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I can work\n\n  Scenario: Log in\n    Given a user\n";

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// `spec/features/login.feature` plus a coverage file listing `scenarios`
/// whose stats claim `claimed_total` scenarios.
fn login_feature(root: &Path, scenarios: &[&str], claimed_total: u64) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(root.join("spec/features/login.feature"), FEATURE)?;
    let coverage = json!({
        "scenarios": scenarios.iter().map(|name| json!({"name": name, "testMappings": []})).collect::<Vec<_>>(),
        "stats": {"totalScenarios": claimed_total, "coveredScenarios": 0, "coveragePercent": 0,
                  "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
    });
    fs::write(
        root.join("spec/features/login.feature.coverage"),
        serde_json::to_string_pretty(&coverage)?,
    )?;
    Ok(())
}

fn coverage(root: &Path) -> Result<Value, Box<dyn Error>> {
    let raw = fs::read_to_string(root.join("spec/features/login.feature.coverage"))?;
    Ok(serde_json::from_str(&raw)?)
}

#[test]
fn adding_a_scenario_adds_an_empty_coverage_entry() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a feature "login" with one scenario and a coverage file
    login_feature(root, &["Log in"], 1)?;
    // @step When the user adds the scenario "Reset password" to "login"
    let result = dispatch(
        root,
        "add-scenario",
        json!({"feature": "login", "scenario": "Reset password"}),
    );
    if !result.success {
        return Err(format!("add-scenario failed: {:?}", result.error).into());
    }
    // @step Then the coverage file lists "Reset password" with no test mappings
    let data = coverage(root)?;
    let added = data["scenarios"]
        .as_array()
        .and_then(|list| list.iter().find(|s| s["name"] == "Reset password"))
        .ok_or("Reset password has a coverage entry")?;
    assert_eq!(added["testMappings"], json!([]));
    // @step And the coverage stats report 2 total scenarios
    assert_eq!(data["stats"]["totalScenarios"], 2);
    Ok(())
}

#[test]
fn linking_coverage_recomputes_the_total_from_the_scenario_list() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a coverage file for "login" listing 2 scenarios whose stats claim 1 total scenario
    login_feature(root, &["Log in", "Reset password"], 1)?;
    fs::create_dir_all(root.join("tests"))?;
    fs::write(root.join("tests/login.rs"), "// @step Given a user\n")?;
    // @step When the user links a test to one of its scenarios
    let result = dispatch(
        root,
        "link-coverage",
        json!({"featureName": "login", "scenario": "Log in", "testFile": "tests/login.rs",
               "testLines": "1-1", "skipValidation": true}),
    );
    if !result.success {
        return Err(format!("link-coverage failed: {:?}", result.error).into());
    }
    // @step Then the coverage stats report 2 total scenarios and 50 percent coverage
    let data = coverage(root)?;
    assert_eq!(data["stats"]["totalScenarios"], 2);
    assert_eq!(data["stats"]["coveragePercent"], 50);
    Ok(())
}
