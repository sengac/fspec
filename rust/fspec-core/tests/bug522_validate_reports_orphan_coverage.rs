//! BUG-522 — fspec validate reports orphan coverage files.
//!
//! Feature: spec/features/fspec-validate-reports-orphan-coverage-files.feature

use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n";

fn project(files: &[&str]) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let features = root.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    for file in files {
        let body = if file.ends_with(".coverage") {
            r#"{"scenarios": [{"name": "Log in", "testMappings": []}]}"#
        } else {
            FEATURE
        };
        std::fs::write(features.join(file), body)?;
    }
    Ok(root)
}

fn validate(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "validate".to_string(),
        args_json: json!({}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("validate failed to run: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

#[test]
fn a_coverage_file_without_its_feature_fails_validation() -> TestResult {
    // @step Given a project with a valid feature and a coverage file "gone.feature.coverage" with no "gone.feature"
    let root = project(&[
        "login.feature",
        "login.feature.coverage",
        "gone.feature.coverage",
    ])?;
    // @step When I validate all feature files
    let report = validate(root.path())?;
    // @step Then validation fails naming "spec/features/gone.feature.coverage" as having no feature file
    let output = report["output"].as_str().unwrap_or_default();
    assert_eq!(report["exitCode"], 1, "{output}");
    assert!(
        output.contains("✗ spec/features/gone.feature.coverage has no feature file"),
        "{output}"
    );
    Ok(())
}

#[test]
fn coverage_files_that_match_their_features_pass_validation() -> TestResult {
    // @step Given a project with a valid feature and its coverage file
    let root = project(&["login.feature", "login.feature.coverage"])?;
    // @step When I validate all feature files
    let report = validate(root.path())?;
    // @step Then validation passes
    assert_eq!(report["exitCode"], 0, "{report}");
    Ok(())
}

#[test]
fn the_repository_has_no_orphan_coverage_files() -> TestResult {
    // @step Given the repository's spec/features directory
    let features: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("repository root")?
        .join("spec/features");
    // @step When I list the coverage files
    let mut orphans = Vec::new();
    for entry in std::fs::read_dir(&features)? {
        let path = entry?.path();
        let name = path.to_string_lossy().to_string();
        // @step Then every coverage file has its feature file
        if let Some(feature) = name.strip_suffix(".coverage") {
            if name.ends_with(".feature.coverage") && !Path::new(feature).exists() {
                orphans.push(name);
            }
        }
    }
    assert!(orphans.is_empty(), "orphan coverage files: {orphans:?}");
    Ok(())
}
