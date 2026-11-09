//! COV-078 — fspec audit --fix relinks a scenario whose exact @step block is
//! found once.
//!
//! Feature: spec/features/audit-fix-relinks-a-scenario-to-its-exact-step-block.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str =
    "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const COVERAGE_FILE: &str = "spec/features/login.feature.coverage";

/// The five lines of the "Log in" @step block, code between the comments.
const BLOCK: [&str; 5] = [
    "    // @step Given a user",
    "    let user = user();",
    "    // @step When they log in",
    "    log_in(&user);",
    "    // @step Then they see the board",
];

/// A test file of `length` lines whose "Log in" block starts at each of
/// `starts` (1-based); every other line is code.
fn test_file(length: usize, starts: &[usize]) -> String {
    let mut lines: Vec<String> = (1..=length).map(|n| format!("    line_{n}();")).collect();
    for start in starts {
        for (offset, text) in BLOCK.iter().enumerate() {
            lines[start - 1 + offset] = (*text).to_string();
        }
    }
    format!("{}\n", lines.join("\n"))
}

struct Project(tempfile::TempDir);

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/features/login.feature", FEATURE)?;
        project.write("src/login.rs", "pub fn log_in(user: &User) {\n}\n")?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn write(&self, rel: &str, body: &str) -> std::io::Result<()> {
        let path = self.root().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }

    fn read(&self, rel: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root().join(rel))
    }

    /// `git init` the project and commit every file, so files written
    /// afterwards are added (untracked).
    fn git_commit_all(&self) -> TestResult {
        let git = |args: &[&str]| -> TestResult {
            let status = std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(args)
                .current_dir(self.root())
                .status()?;
            if !status.success() {
                return Err(format!("git {args:?} failed").into());
            }
            Ok(())
        };
        git(&["init", "-q"])?;
        git(&["add", "-A"])?;
        git(&["commit", "-q", "-m", "initial"])
    }

    /// A coverage file whose only scenario "Log in" maps
    /// tests/login_test.rs at `lines`, with an implementation mapping of
    /// src/login.rs that records its line text.
    fn coverage(&self, lines: &str) -> std::io::Result<()> {
        self.write(
            COVERAGE_FILE,
            &json!({"scenarios": [
                {"name": "Log in", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": lines, "implMappings": [
                        {"file": "src/login.rs", "lines": [1, 2],
                         "firstLine": "pub fn log_in(user: &User) {", "lastLine": "}"}
                    ]}
                ]}
            ]})
            .to_string(),
        )
    }

    fn sidecar(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&self.read(COVERAGE_FILE)?)?)
    }

    /// The (file, lines) of each test mapping of "Log in".
    fn test_mappings(&self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        Ok(self.sidecar()?["scenarios"][0]["testMappings"]
            .as_array()
            .ok_or("no testMappings")?
            .iter()
            .map(|row| {
                (
                    row["file"].as_str().unwrap_or_default().to_string(),
                    row["lines"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect())
    }
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// `fspec audit --feature <feature> --area <area>` with `extra` arguments:
/// the envelope.
fn audit(
    root: &Path,
    feature: &str,
    area: &str,
    extra: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut args = json!({"feature": [feature], "area": [area]});
    if let (Some(args), Some(extra)) = (args.as_object_mut(), extra.as_object()) {
        args.extend(extra.clone());
    }
    let result = dispatch(root, "audit", args);
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

/// `fspec audit --fix --feature login --area coverage`: its text output.
fn fix(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let envelope = audit(root, "login", "coverage", json!({"fix": true}))?;
    Ok(envelope["output"].as_str().unwrap_or_default().to_string())
}

/// `link-coverage login --scenario "Log in" --test-file tests/login_test.rs`.
fn link_without_lines(root: &Path) -> DispatchResult {
    dispatch(
        root,
        "link-coverage",
        json!({"featureName": "login", "scenario": "Log in", "testFile": "tests/login_test.rs"}),
    )
}

#[test]
fn a_drifted_mapping_is_relinked_to_the_block_found_once_in_its_file() -> TestResult {
    // @step Given scenario "Log in" maps tests/login_test.rs:10-14 with an implementation mapping of src/login.rs
    let project = Project::new()?;
    project.coverage("10-14")?;
    // @step And its @step comments moved to lines 22-26
    project.write("tests/login_test.rs", &test_file(40, &[22]))?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the mapping becomes tests/login_test.rs:22-26 and keeps its src/login.rs implementation mapping
    assert_eq!(
        project.test_mappings()?,
        vec![("tests/login_test.rs".to_string(), "22-26".to_string())]
    );
    let impls = project.sidecar()?["scenarios"][0]["testMappings"][0]["implMappings"].clone();
    assert_eq!(impls[0]["file"], "src/login.rs", "{impls}");
    assert_eq!(impls[0]["lines"], json!([1, 2]), "{impls}");
    // @step And it prints "tests/login_test.rs:10-14 -> tests/login_test.rs:22-26"
    assert!(
        output.contains("tests/login_test.rs:10-14 -> tests/login_test.rs:22-26"),
        "{output}"
    );
    assert!(output.contains("No problems found"), "{output}");
    // @step And a second fspec audit --fix changes nothing
    let before = project.read(COVERAGE_FILE)?;
    let output = fix(project.root())?;
    assert!(!output.contains(" -> "), "{output}");
    assert_eq!(project.read(COVERAGE_FILE)?, before);
    Ok(())
}

#[test]
fn a_relinkable_drift_names_the_fix_command() -> TestResult {
    // @step Given scenario "Log in" maps tests/login_test.rs:10-14 but its @step comments moved to lines 22-26
    let project = Project::new()?;
    project.coverage("10-14")?;
    project.write("tests/login_test.rs", &test_file(40, &[22]))?;
    // @step When fspec audit --json runs
    let envelope = audit(project.root(), "login", "coverage", json!({"json": true}))?;
    // @step Then the drifted problem has fix kind "mechanical" and command "fspec audit --fix --feature login"
    let report: Value = serde_json::from_str(envelope["output"].as_str().unwrap_or_default())?;
    let problems = report["problems"].as_array().ok_or("no problems array")?;
    let drifted = problems
        .iter()
        .find(|problem| problem["code"] == "drifted")
        .ok_or_else(|| format!("no drifted problem: {report}"))?;
    assert_eq!(drifted["fix"]["kind"], "mechanical", "{drifted}");
    assert_eq!(
        drifted["fix"]["command"], "fspec audit --fix --feature login",
        "{drifted}"
    );
    Ok(())
}

#[test]
fn a_test_moved_into_a_new_untracked_file_is_relinked_to_that_file() -> TestResult {
    // @step Given a git repository where scenario "Log in" maps tests/login_test.rs:10-14
    let project = Project::new()?;
    project.coverage("10-14")?;
    project.write("tests/login_test.rs", &test_file(20, &[10]))?;
    project.git_commit_all()?;
    // @step And the test moved into the new untracked file tests/login_api_test.rs at lines 3-7
    project.write("tests/login_test.rs", &test_file(20, &[]))?;
    project.write("tests/login_api_test.rs", &test_file(12, &[3]))?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the mapping becomes tests/login_api_test.rs:3-7
    assert_eq!(
        project.test_mappings()?,
        vec![("tests/login_api_test.rs".to_string(), "3-7".to_string())],
        "{output}"
    );
    assert!(
        output.contains("tests/login_test.rs:10-14 -> tests/login_api_test.rs:3-7"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_new_file_another_feature_maps_is_not_searched() -> TestResult {
    // @step Given a git repository where scenario "Log in" maps tests/login_test.rs:10-14 and its steps are gone from that file
    let project = Project::new()?;
    project.coverage("10-14")?;
    project.write("tests/login_test.rs", &test_file(20, &[10]))?;
    project.git_commit_all()?;
    project.write("tests/login_test.rs", &test_file(20, &[]))?;
    // @step And the new untracked file tests/shared_test.rs holds the block but feature "admin" maps it
    project.write("tests/shared_test.rs", &test_file(12, &[3]))?;
    project.write(
        "spec/features/admin.feature.coverage",
        &json!({"scenarios": [{"name": "Admin", "testMappings": [
            {"file": "tests/shared_test.rs", "lines": "1-2", "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the mapping stays tests/login_test.rs:10-14
    assert_eq!(
        project.test_mappings()?,
        vec![("tests/login_test.rs".to_string(), "10-14".to_string())],
        "{output}"
    );
    // @step And the problem says the exact @step block occurs 0 times
    assert!(
        output.contains("the exact @step block occurs 0 times"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_block_that_occurs_twice_is_not_relinked() -> TestResult {
    // @step Given scenario "Log in" maps tests/login_test.rs:1-5, which holds none of its steps
    let project = Project::new()?;
    project.coverage("1-5")?;
    // @step And its exact @step block occurs at lines 10-14 and 40-44
    project.write("tests/login_test.rs", &test_file(50, &[10, 40]))?;
    let before = project.read(COVERAGE_FILE)?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the coverage file is unchanged
    assert_eq!(project.read(COVERAGE_FILE)?, before, "{output}");
    // @step And the problem says the exact @step block occurs 2 times (tests/login_test.rs:10-14, tests/login_test.rs:40-44)
    assert!(
        output.contains(
            "the exact @step block occurs 2 times (tests/login_test.rs:10-14, tests/login_test.rs:40-44)"
        ),
        "{output}"
    );
    Ok(())
}

#[test]
fn steps_out_of_order_are_not_an_exact_block() -> TestResult {
    // @step Given scenario "Log in" maps tests/login_test.rs:1-5, which holds none of its steps
    let project = Project::new()?;
    project.coverage("1-5")?;
    // @step And the file holds its @step comments only in another order
    let reversed = test_file(30, &[20])
        .replace("// @step Given a user", "// @step GIVEN_SLOT")
        .replace("// @step Then they see the board", "// @step Given a user")
        .replace("// @step GIVEN_SLOT", "// @step Then they see the board");
    project.write("tests/login_test.rs", &reversed)?;
    let before = project.read(COVERAGE_FILE)?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the coverage file is unchanged
    assert_eq!(project.read(COVERAGE_FILE)?, before, "{output}");
    // @step And the problem says the exact @step block occurs 0 times
    assert!(
        output.contains("the exact @step block occurs 0 times"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_coverage_entry_for_a_scenario_the_feature_no_longer_has_is_removed() -> TestResult {
    // @step Given the coverage file lists scenario "Old" that the feature no longer has
    let project = Project::new()?;
    project.write("tests/login_test.rs", &test_file(20, &[3]))?;
    project.write(
        COVERAGE_FILE,
        &json!({"scenarios": [
            {"name": "Log in", "testMappings": [
                {"file": "tests/login_test.rs", "lines": "3-7", "implMappings": [
                    {"file": "src/login.rs", "lines": [1, 2],
                     "firstLine": "pub fn log_in(user: &User) {", "lastLine": "}"}
                ]}
            ]},
            {"name": "Old", "testMappings": []}
        ]})
        .to_string(),
    )?;
    // @step When fspec audit --fix runs
    let output = fix(project.root())?;
    // @step Then the "Old" entry is removed
    let names: Vec<Value> = project.sidecar()?["scenarios"]
        .as_array()
        .ok_or("no scenarios")?
        .iter()
        .map(|scenario| scenario["name"].clone())
        .collect();
    assert_eq!(names, vec![json!("Log in")], "{output}");
    // @step And it prints the removal
    assert!(
        output.contains("fixed coverage stale-entry login \"Old\": removed the coverage entry"),
        "{output}"
    );
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn a_coverage_file_without_its_feature_is_deleted() -> TestResult {
    // @step Given spec/features/old.feature.coverage exists without old.feature
    let project = Project::new()?;
    project.write(
        "spec/features/old.feature.coverage",
        &json!({"scenarios": []}).to_string(),
    )?;
    // @step When fspec audit --fix runs
    let envelope = audit(project.root(), "old", "spec", json!({"fix": true}))?;
    // @step Then spec/features/old.feature.coverage is deleted
    assert!(
        !project
            .root()
            .join("spec/features/old.feature.coverage")
            .exists(),
        "{envelope}"
    );
    let output = envelope["output"].as_str().unwrap_or_default();
    assert!(
        output
            .contains("fixed spec orphan-coverage old: deleted spec/features/old.feature.coverage"),
        "{output}"
    );
    Ok(())
}

#[test]
fn link_coverage_without_test_lines_links_the_block_found_once() -> TestResult {
    // @step Given the "Log in" @step block occurs once in tests/login_test.rs, at lines 22-26
    let project = Project::new()?;
    project.write("tests/login_test.rs", &test_file(40, &[22]))?;
    project.write(
        COVERAGE_FILE,
        &json!({"scenarios": [{"name": "Log in", "testMappings": []}]}).to_string(),
    )?;
    // @step When fspec link-coverage login --scenario "Log in" --test-file tests/login_test.rs runs without --test-lines
    let result = link_without_lines(project.root());
    assert!(result.success, "{result:?}");
    // @step Then the scenario maps tests/login_test.rs:22-26
    assert_eq!(
        project.test_mappings()?,
        vec![("tests/login_test.rs".to_string(), "22-26".to_string())]
    );
    Ok(())
}

#[test]
fn link_coverage_without_test_lines_refuses_an_ambiguous_block() -> TestResult {
    // @step Given the "Log in" @step block occurs twice in tests/login_test.rs
    let project = Project::new()?;
    project.write("tests/login_test.rs", &test_file(50, &[10, 40]))?;
    project.write(
        COVERAGE_FILE,
        &json!({"scenarios": [{"name": "Log in", "testMappings": []}]}).to_string(),
    )?;
    let before = project.read(COVERAGE_FILE)?;
    // @step When fspec link-coverage login --scenario "Log in" --test-file tests/login_test.rs runs without --test-lines
    let result = link_without_lines(project.root());
    // @step Then it fails saying the exact @step block occurs 2 times
    assert!(!result.success, "{result:?}");
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("the exact @step block occurs 2 times"),
        "{error}"
    );
    // @step And the coverage file is unchanged
    assert_eq!(project.read(COVERAGE_FILE)?, before);
    Ok(())
}
