//! BUG-645 — `fspec delete-features` prints the coverage files it deletes
//! with the features, and those a dry run would delete.
//!
//! Feature: spec/features/bulk-deletion-leaves-a-clean-spec-tree.feature

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const COVERAGE: &str = "spec/features/a.feature.coverage";

/// A project whose feature "a", tagged @gone, has a coverage file.
fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let features = root.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    std::fs::write(
        features.join("a.feature"),
        "@gone\nFeature: A\n\n  Scenario: A\n    Given x\n",
    )?;
    std::fs::write(
        features.join("a.feature.coverage"),
        r#"{"scenarios":[{"name":"A","testMappings":[]}]}"#,
    )?;
    Ok(root)
}

/// Run `fspec delete-features <args>` in `root`; exit code and stdout.
fn delete_features(
    root: &Path,
    args: &[&str],
) -> Result<(i32, String), Box<dyn std::error::Error>> {
    let output = Command::new(fspec_bin())
        .arg("delete-features")
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "fspec delete-features failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok((
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout)?,
    ))
}

#[test]
fn the_cli_prints_the_deleted_coverage_files() -> TestResult {
    // @step Given feature "a" with a coverage file is tagged @gone
    let root = project()?;
    // @step When I run fspec delete-features --tag @gone
    let (code, stdout) = delete_features(root.path(), &["--tag", "@gone"])?;
    // @step Then stdout lists spec/features/a.feature.coverage under "Deleted coverage files:"
    assert_eq!(code, 0);
    assert!(
        stdout.contains(&format!("\nDeleted coverage files:\n  - {COVERAGE}\n")),
        "{stdout}"
    );
    // @step And the coverage file is gone
    assert!(!root.path().join(COVERAGE).exists());
    Ok(())
}

#[test]
fn the_cli_dry_run_prints_the_coverage_files_it_would_delete() -> TestResult {
    // @step Given feature "a" with a coverage file is tagged @gone
    let root = project()?;
    // @step When I run fspec delete-features --tag @gone --dry-run
    let (code, stdout) = delete_features(root.path(), &["--tag", "@gone", "--dry-run"])?;
    // @step Then stdout lists spec/features/a.feature.coverage under "Would delete 1 coverage file(s):"
    assert_eq!(code, 0);
    assert!(
        stdout.contains(&format!(
            "\nWould delete 1 coverage file(s):\n\n  - {COVERAGE}\n"
        )),
        "{stdout}"
    );
    // @step And the coverage file is still on disk
    assert!(root.path().join(COVERAGE).exists());
    Ok(())
}
