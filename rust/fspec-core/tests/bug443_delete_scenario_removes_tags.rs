//! BUG-443 — deleting a scenario removes the tag lines directly above it.
//!
//! Feature: spec/features/delete-scenario-removes-its-tags.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn write_login(root: &Path, body: &str) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(root.join("spec/features/login.feature"), body)?;
    Ok(())
}

fn delete_old(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "delete-scenario".to_string(),
        args_json: json!({"feature": "login", "scenario": "Old"}).to_string(),
        project_root: root.to_path_buf(),
    })
}

fn login(root: &Path) -> Result<String, std::io::Error> {
    fs::read_to_string(root.join("spec/features/login.feature"))
}

#[test]
fn deleting_a_tagged_last_scenario_removes_its_tag_line() -> TestResult {
    let tmp = setup_temp_directory()?;
    // @step Given a feature "login" whose last scenario "Old" is tagged "@legacy"
    write_login(
        tmp.path(),
        "@cli\nFeature: Login\n\n  Scenario: Keep\n    Given a user\n\n  @legacy\n  Scenario: Old\n    Given an old user\n",
    )?;
    // @step When the user deletes the scenario "Old" from "login"
    let result = delete_old(tmp.path());
    // @step Then the deletion succeeds
    if !result.success {
        return Err(format!("delete-scenario failed: {:?}", result.error).into());
    }
    // @step And "login" no longer contains "@legacy"
    let content = login(tmp.path())?;
    assert!(!content.contains("@legacy"), "{content}");
    Ok(())
}

#[test]
fn deleting_a_scenario_keeps_the_next_scenario_s_tags() -> TestResult {
    let tmp = setup_temp_directory()?;
    // @step Given a feature "login" with scenario "Old" followed by scenario "Keep" tagged "@smoke"
    write_login(
        tmp.path(),
        "@cli\nFeature: Login\n\n  Scenario: Old\n    Given an old user\n\n  @smoke\n  Scenario: Keep\n    Given a user\n",
    )?;
    // @step When the user deletes the scenario "Old" from "login"
    let result = delete_old(tmp.path());
    // @step Then the deletion succeeds
    if !result.success {
        return Err(format!("delete-scenario failed: {:?}", result.error).into());
    }
    // @step And "login" still tags "Keep" with "@smoke"
    let content = login(tmp.path())?;
    assert!(content.contains("  @smoke\n  Scenario: Keep"), "{content}");
    assert!(content.starts_with("@cli\nFeature: Login"), "{content}");
    Ok(())
}
