//! BUG-551 — coverage writes leave no lock files beside the coverage files.
//!
//! Every coverage sidecar write holds the project's spec write lock
//! (`spec/.fspec-write.lock`, BUG-477): one persistent lock per spec root
//! instead of a `<name>.feature.coverage.lock` per feature.
//!
//! Feature: spec/features/coverage-writes-leave-no-lock-files.feature

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use codelet_common::spec_lock::SPEC_LOCK_TIMEOUT_ENV;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use fs2::FileExt;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";
const SIGNUP: &str = "Feature: Signup\n\n  Scenario: Sign up\n    Given a visitor\n    When they sign up\n    Then they have an account\n";
const TEST_FILE: &str = "tests/login.rs";

fn write(root: &Path, relative: &str, body: &str) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
}

/// A project with a covered "login" feature and an uncovered "signup" one.
fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    write(root.path(), "spec/features/login.feature", LOGIN)?;
    write(root.path(), "spec/features/signup.feature", SIGNUP)?;
    write(
        root.path(),
        "spec/features/login.feature.coverage",
        &json!({
            "scenarios": [{"name": "Log in", "testMappings": []}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
                "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
        })
        .to_string(),
    )?;
    write(
        root.path(),
        TEST_FILE,
        "fn log_in() {\n    // @step Given a user\n    // @step When they log in\n    // @step Then they see the board\n}\n",
    )?;
    Ok(root)
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn succeed(root: &Path, command: &str, args: Value) -> TestResult {
    let result = dispatch(root, command, args);
    if result.success {
        Ok(())
    } else {
        Err(format!("{command} failed: {:?}", result.error).into())
    }
}

fn link_args() -> Value {
    json!({"featureName": "login", "scenario": "Log in", "testFile": TEST_FILE,
        "testLines": "1-5"})
}

/// Every file under `directory` whose name ends in `.lock`, relative to `root`.
fn lock_files(root: &Path, directory: &Path, found: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path: PathBuf = entry?.path();
        if path.is_dir() {
            lock_files(root, &path, found)?;
        } else if path.to_string_lossy().ends_with(".lock") {
            found.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn test_mappings(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(root.join("spec/features/login.feature.coverage"))?;
    let coverage: Value = serde_json::from_str(&raw)?;
    Ok(coverage["scenarios"][0]["testMappings"].clone())
}

#[test]
fn coverage_commands_leave_no_lock_file_beside_the_coverage_files() -> TestResult {
    // @step Given a project with a feature and its coverage file
    let root = project()?;
    let root = root.path();

    // @step When add-scenario, link-coverage, unlink-coverage and generate-coverage rewrite the coverage file
    succeed(
        root,
        "add-scenario",
        json!({"feature": "login", "scenario": "Log out"}),
    )?;
    succeed(root, "link-coverage", link_args())?;
    succeed(
        root,
        "unlink-coverage",
        json!({"featureName": "login", "scenario": "Log in", "testFile": TEST_FILE}),
    )?;
    succeed(root, "generate-coverage", json!({}))?;
    assert!(
        root.join("spec/features/signup.feature.coverage").is_file(),
        "generate-coverage wrote no coverage file for signup"
    );

    // BUG-597: the spec write lock is removed when the write finishes.
    // @step Then spec/features and spec contain no lock file
    let mut found = Vec::new();
    lock_files(root, &root.join("spec"), &mut found)?;
    assert_eq!(found, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_coverage_write_waits_for_the_spec_write_lock() -> TestResult {
    let root = project()?;
    let path = root.path().to_path_buf();

    // @step Given another process holds spec/.fspec-write.lock
    // A separate open file description conflicts with the command's own
    // lock exactly as another process's would.
    let holder = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.join("spec/.fspec-write.lock"))?;
    holder.lock_exclusive()?;

    // @step When link-coverage runs
    // TEST-042: no waiting window. With a lock budget of 0 ms the command
    // gives up at once instead of waiting, and the coverage file is checked
    // after it has returned: a command that skipped the lock would have
    // written by then.
    std::env::set_var(SPEC_LOCK_TIMEOUT_ENV, "0");
    let writer_root = path.clone();
    let locked =
        std::thread::spawn(move || dispatch(&writer_root, "link-coverage", link_args())).join();
    std::env::remove_var(SPEC_LOCK_TIMEOUT_ENV);
    let locked = locked.map_err(|_| "link-coverage thread panicked")?;

    // @step Then it writes the coverage file only after that process releases the lock
    assert!(
        !locked.success
            && locked
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Failed to acquire file lock")),
        "link-coverage ran while the spec write lock was held: {locked:?}"
    );
    assert_eq!(test_mappings(&path)?, json!([]));
    FileExt::unlock(&holder)?;
    let result = dispatch(&path, "link-coverage", link_args());
    assert!(result.success, "link-coverage failed: {:?}", result.error);
    assert_eq!(test_mappings(&path)?[0]["lines"], "1-5");
    Ok(())
}

#[test]
fn the_cleanup_script_removes_leftover_coverage_lock_files() -> TestResult {
    // @step Given a project whose spec/features holds leftover .feature.coverage.lock files beside features, coverage files and other files
    let root = project()?;
    let root = root.path();
    write(root, "spec/features/login.feature.coverage.lock", "")?;
    write(root, "spec/features/nested/deep.feature.coverage.lock", "")?;
    write(root, "spec/features/nested/deep.feature", SIGNUP)?;
    write(root, "spec/features/notes.lock", "kept")?;
    write(root, "spec/.fspec-write.lock", "")?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/remove-coverage-lock-files.sh")
        .canonicalize()?;

    // @step When scripts/remove-coverage-lock-files.sh runs in the project
    let status = std::process::Command::new("sh")
        .arg(&script)
        .current_dir(root)
        .status()?;
    assert!(status.success(), "the cleanup script failed: {status}");

    // @step Then every .feature.coverage.lock file is gone and every other file is kept
    let mut found = Vec::new();
    lock_files(root, &root.join("spec"), &mut found)?;
    found.sort();
    assert_eq!(
        found,
        vec![
            "spec/.fspec-write.lock".to_string(),
            "spec/features/notes.lock".to_string()
        ]
    );
    for kept in [
        "spec/features/login.feature",
        "spec/features/login.feature.coverage",
        "spec/features/nested/deep.feature",
        TEST_FILE,
    ] {
        assert!(root.join(kept).is_file(), "{kept} was removed");
    }
    Ok(())
}
