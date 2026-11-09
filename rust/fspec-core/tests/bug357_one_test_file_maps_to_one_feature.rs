//! BUG-357 — one test file maps to one feature.
//!
//! Feature: spec/features/multiple-test-files-mapped-to-single-feature-file-causes-validation-errors.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN_FEATURE: &str = "@AUTH-001\nFeature: Login\n\n  Scenario: Log in through the UI\n    Given a user\n    Then they see the board\n\n  Scenario: Log in through the API\n    Given a token\n    Then the API answers\n";

const UI_TEST: &str =
    "fn ui() {\n    // @step Given a user\n    // @step Then they see the board\n}\n";
const API_TEST: &str =
    "fn api() {\n    // @step Given a token\n    // @step Then the API answers\n}\n";

fn write(root: &Path, rel: &str, body: &str) -> std::io::Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
}

fn work_units() -> String {
    json!({
        "version": "0.7.1",
        "meta": {"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"},
        "workUnits": {"AUTH-001": {
            "id": "AUTH-001", "title": "Login", "type": "story", "status": "testing",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
        }},
        "states": {"backlog": [], "specifying": [], "testing": ["AUTH-001"], "implementing": [],
            "validating": [], "done": [], "blocked": []}
    })
    .to_string()
}

/// A project whose login feature is covered by tests/login_ui.rs and
/// tests/login_api.rs; with `shared`, signup.feature also maps login_api.rs.
fn project(shared: bool) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let r = root.path();
    write(r, "spec/work-units.json", &work_units())?;
    write(r, "spec/features/login.feature", LOGIN_FEATURE)?;
    write(r, "tests/login_ui.rs", UI_TEST)?;
    write(r, "tests/login_api.rs", API_TEST)?;
    write(
        r,
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [
            {"name": "Log in through the UI", "testMappings": [{"file": "tests/login_ui.rs", "lines": "1-4", "implMappings": []}]},
            {"name": "Log in through the API", "testMappings": [{"file": "tests/login_api.rs", "lines": "1-4", "implMappings": []}]}
        ]})
        .to_string(),
    )?;
    if shared {
        write(
            r,
            "spec/features/signup.feature",
            "Feature: Signup\n\n  Scenario: Sign up\n    Given a token\n    Then the API answers\n",
        )?;
        write(
            r,
            "spec/features/signup.feature.coverage",
            &json!({"scenarios": [{"name": "Sign up", "testMappings": [
                {"file": "tests/login_api.rs", "lines": "1-4", "implMappings": []}
            ]}]})
            .to_string(),
        )?;
    }
    Ok(root)
}

fn to_implementing(root: &Path) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "status": "implementing", "skipTemporalValidation": true}).to_string(),
        project_root: root.to_path_buf(),
    })
}

fn status(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let data: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("spec/work-units.json"))?)?;
    Ok(data["workUnits"]["AUTH-001"]["status"]
        .as_str()
        .unwrap_or_default()
        .to_owned())
}

#[test]
fn a_feature_covered_by_several_test_files_moves_on() -> TestResult {
    // @step Given a work unit's feature is covered by two test files that no other feature maps
    let root = project(false)?;
    // @step When the work unit moves from testing to implementing
    let result = to_implementing(root.path());
    // @step Then the transition succeeds
    assert!(result.success, "{result:?}");
    assert_eq!(status(root.path())?, "implementing");
    Ok(())
}

#[test]
fn a_test_file_shared_with_another_feature_blocks_the_transition() -> TestResult {
    // @step Given a work unit's feature maps a test file that another feature also maps
    let root = project(true)?;
    // @step When the work unit moves from testing to implementing
    let result = to_implementing(root.path());
    // @step Then the transition is refused
    assert!(!result.success, "{result:?}");
    assert_eq!(status(root.path())?, "testing");
    // @step And the error names the shared test file and the other feature
    let message = format!("{:?} {:?}", result.error, result.system_reminder);
    assert!(message.contains("tests/login_api.rs"), "{message}");
    assert!(message.contains("signup.feature"), "{message}");
    Ok(())
}

#[test]
fn fspec_audit_reports_a_test_file_shared_with_another_feature() -> TestResult {
    // @step Given a feature maps a test file that another feature also maps
    let root = project(true)?;
    // @step When its coverage is audited
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["login"], "area": ["coverage"]}).to_string(),
        project_root: root.path().to_path_buf(),
    });
    assert!(result.success, "{result:?}");
    let report: Value = serde_json::from_str(&result.data)?;
    // @step Then the audit fails naming the shared test file and the other feature
    let output = report["output"].as_str().unwrap_or_default();
    assert_eq!(report["exitCode"], 1, "{output}");
    assert!(
        output.contains("tests/login_api.rs") && output.contains("signup.feature"),
        "{output}"
    );
    Ok(())
}
