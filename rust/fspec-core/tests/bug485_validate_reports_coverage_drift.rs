//! BUG-485 — fspec validate reports coverage files that drifted from their
//! feature's scenarios, so the card that adds or removes scenarios syncs the
//! coverage (exact scenario names only).
//!
//! Feature: spec/features/validate-reports-coverage-drift.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n\n  Scenario: Log out\n    Given a signed-in user\n";

/// A project whose login.feature has "Log in" and "Log out", and (unless
/// `None`) a coverage file listing `covered`.
fn project(covered: Option<&[&str]>) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let features = root.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    std::fs::write(features.join("login.feature"), LOGIN)?;
    if let Some(names) = covered {
        let scenarios: Vec<Value> = names
            .iter()
            .map(|name| json!({"name": name, "testMappings": []}))
            .collect();
        std::fs::write(
            features.join("login.feature.coverage"),
            json!({"scenarios": scenarios}).to_string(),
        )?;
    }
    Ok(root)
}

fn validate(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "validate".to_string(),
        args_json: json!({}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("validate failed to run: {result:?}").into());
    }
    let report: Value = serde_json::from_str(&result.data)?;
    let output = report["output"].as_str().unwrap_or_default().to_string();
    Ok((report, output))
}

#[test]
fn a_coverage_file_missing_a_scenario_is_reported() -> TestResult {
    // @step Given login.feature has the scenarios "Log in" and "Log out" and its coverage file lists only "Log in"
    let root = project(Some(&["Log in"]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports that spec/features/login.feature.coverage is missing the scenario "Log out" and exits 1
    assert!(
        output
            .contains("✗ spec/features/login.feature.coverage is missing the scenario \"Log out\""),
        "{output}"
    );
    assert_eq!(report["exitCode"], 1, "{output}");
    Ok(())
}

#[test]
fn a_coverage_file_listing_a_scenario_the_feature_no_longer_has_is_reported() -> TestResult {
    // @step Given login.feature.coverage lists "Reset password", which login.feature no longer has
    let root = project(Some(&["Log in", "Log out", "Reset password"]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports that spec/features/login.feature.coverage lists the scenario "Reset password" the feature does not have and exits 1
    assert!(
        output.contains(
            "✗ spec/features/login.feature.coverage lists the scenario \"Reset password\" the feature does not have"
        ),
        "{output}"
    );
    assert_eq!(report["exitCode"], 1, "{output}");
    Ok(())
}

#[test]
fn a_feature_without_a_coverage_file_yet_is_not_reported() -> TestResult {
    // @step Given login.feature has scenarios and no coverage file
    let root = project(None)?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports no coverage drift for it and exits 0, because a feature gets its coverage file when its coverage is first generated
    assert!(!output.contains("coverage"), "{output}");
    assert_eq!(report["exitCode"], 0, "{output}");
    Ok(())
}

#[test]
fn a_coverage_file_that_lists_exactly_the_features_scenarios_passes() -> TestResult {
    // @step Given login.feature.coverage lists exactly the scenario names of login.feature
    let root = project(Some(&["Log out", "Log in"]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports no coverage drift and exits 0
    assert!(!output.contains("coverage"), "{output}");
    assert_eq!(report["exitCode"], 0, "{output}");
    Ok(())
}

#[test]
fn the_repository_has_no_coverage_drift() -> TestResult {
    // @step Given the drifted coverage of structured-generation-aware-session-replay, typed-durable-summary-identity and client-mcp-read-only is synced with generate-coverage
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("repository root")?;
    // @step When fspec validate runs over this repository
    let (_, output) = validate(repository)?;
    // @step Then it reports no coverage drift
    let drift: Vec<&str> = output
        .lines()
        .filter(|line| {
            line.contains("is missing the scenario") || line.contains("the feature does not have")
        })
        .collect();
    assert!(drift.is_empty(), "coverage drift:\n{}", drift.join("\n"));
    Ok(())
}
