//! BUG-374 — deleting scenarios recomputes every coverage stat, including
//! the test and implementation file lists.
//!
//! Feature: spec/features/coverage-stats-after-scenario-deletion.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// login.feature with "Keep" and "Old"; only "Old" maps tests/old.rs and
/// src/old.rs. `old_tag` tags the Old scenario (empty for none).
fn login_feature(root: &Path, old_tag: &str) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features/login.feature"),
        format!(
            "@cli\nFeature: Login\n\n  Scenario: Keep\n    Given a user\n\n  {old_tag}\n  Scenario: Old\n    Given an old user\n"
        ),
    )?;
    let coverage = json!({
        "scenarios": [
            {"name": "Keep", "testMappings": [{"file": "tests/keep.rs", "lines": "1-2",
                "implMappings": [{"file": "src/keep.rs", "lines": [1]}]}]},
            {"name": "Old", "testMappings": [{"file": "tests/old.rs", "lines": "1-2",
                "implMappings": [{"file": "src/old.rs", "lines": [1]}]}]}
        ],
        "stats": {"totalScenarios": 2, "coveredScenarios": 2, "coveragePercent": 100,
                  "testFiles": ["tests/keep.rs", "tests/old.rs"],
                  "implFiles": ["src/keep.rs", "src/old.rs"], "totalLinesCovered": 6}
    });
    fs::write(
        root.join("spec/features/login.feature.coverage"),
        serde_json::to_string_pretty(&coverage)?,
    )?;
    Ok(())
}

/// Then the coverage stats no longer list tests/old.rs or src/old.rs.
fn expect_old_files_dropped(root: &Path) -> TestResult {
    let raw = fs::read_to_string(root.join("spec/features/login.feature.coverage"))?;
    let stats = serde_json::from_str::<Value>(&raw)?["stats"].clone();
    assert_eq!(stats["testFiles"], json!(["tests/keep.rs"]), "{stats}");
    assert_eq!(stats["implFiles"], json!(["src/keep.rs"]), "{stats}");
    assert_eq!(stats["totalScenarios"], 1, "{stats}");
    Ok(())
}

#[test]
fn deleting_a_scenario_drops_its_test_and_implementation_files_from_the_stats() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a feature "login" whose scenario "Old" alone maps "tests/old.rs" and "src/old.rs"
    login_feature(root, "")?;
    // @step When the user deletes the scenario "Old" from "login"
    let result = dispatch(
        root,
        "delete-scenario",
        json!({"feature": "login", "scenario": "Old"}),
    );
    if !result.success {
        return Err(format!("delete-scenario failed: {:?}", result.error).into());
    }
    // @step Then the coverage stats no longer list "tests/old.rs" or "src/old.rs"
    expect_old_files_dropped(root)
}

#[test]
fn deleting_scenarios_by_tag_drops_their_files_from_the_stats() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a feature "login" whose scenario "Old" alone maps "tests/old.rs" and "src/old.rs" and is tagged "@legacy"
    login_feature(root, "@legacy")?;
    // @step When the user deletes the scenarios tagged "@legacy"
    let result = dispatch(root, "delete-scenarios", json!({"tags": ["@legacy"]}));
    if !result.success {
        return Err(format!("delete-scenarios failed: {:?}", result.error).into());
    }
    // @step Then the coverage stats no longer list "tests/old.rs" or "src/old.rs"
    expect_old_files_dropped(root)
}
