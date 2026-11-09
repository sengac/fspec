//! BUG-591 — fspec audit --fix records line text only where git shows the
//! lines did not move.
//!
//! Feature: spec/features/audit-fix-records-line-text-only-where-git-shows-the-lines-did-not-move.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n\n  Scenario: Log out\n    Given a user\n    When they log out\n    Then they see the login page\n";

const TEST_FILE: &str = "// @step Given a user\n// @step When they log in\n// @step Then they see the board\n// @step Given a user\n// @step When they log out\n// @step Then they see the login page\n";

const SIGNATURE: &str = "pub fn log_in(user: &User) -> Session {";

/// `before` filler lines, the 15-line function (signature, 13 body lines,
/// closing brace) and `after` two-line helpers.
fn source(before: usize, after: usize) -> String {
    let mut lines: Vec<String> = (1..=before).map(|n| format!("// filler {n}")).collect();
    lines.push(SIGNATURE.to_string());
    lines.extend((1..=13).map(|n| format!("    step_{n}(user);")));
    lines.push("}".to_string());
    lines.extend((1..=after).map(|n| format!("fn helper_{n}() {{\n}}")));
    format!("{}\n", lines.join("\n"))
}

/// The legacy (no line text) implementation mapping of src/login.rs 10-24.
fn legacy() -> Value {
    json!({"file": "src/login.rs", "lines": (10..=24).collect::<Vec<i64>>()})
}

struct Project(tempfile::TempDir);

impl Project {
    /// A project with the login feature, its test file and src/login.rs with
    /// the function at lines 10-24.
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/features/login.feature", FEATURE)?;
        project.write("tests/login_test.rs", TEST_FILE)?;
        project.write("src/login.rs", &source(9, 2))?;
        Ok(project)
    }

    /// [`Project::new`] in a git working tree.
    fn git_repo() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project::new()?;
        project.git(&["init", "-q"])?;
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

    /// The coverage file with `log_in` as the implementation mappings of
    /// "Log in" and `log_out` as those of "Log out".
    fn coverage(&self, log_in: Value, log_out: Value) -> std::io::Result<()> {
        self.write(
            "spec/features/login.feature.coverage",
            &json!({"scenarios": [
                {"name": "Log in", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": "1-3", "implMappings": log_in}
                ]},
                {"name": "Log out", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": "4-6", "implMappings": log_out}
                ]}
            ]})
            .to_string(),
        )
    }

    fn git(&self, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let output = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_AUTHOR_DATE", "2026-11-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-11-01T00:00:00Z")
            .current_dir(self.root())
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Commit every file; the first 12 hex characters of the new commit.
    fn commit(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "-m", message])?;
        let hash = self.git(&["rev-parse", "HEAD"])?;
        Ok(hash[..12].to_string())
    }

    /// `fspec audit --area coverage --feature login [--fix]`: (exit code,
    /// problems).
    fn audit(&self, fix: bool) -> Result<(u64, Vec<Value>), Box<dyn std::error::Error>> {
        let result = dispatch_command(DispatchRequest {
            command: "audit".to_string(),
            args_json: json!({"feature": ["login"], "area": ["coverage"], "fix": fix}).to_string(),
            project_root: self.root().to_path_buf(),
        });
        if !result.success {
            return Err(format!("fspec audit failed: {result:?}").into());
        }
        let envelope: Value = serde_json::from_str(&result.data)?;
        Ok((
            envelope["exitCode"].as_u64().unwrap_or(99),
            envelope["problems"].as_array().cloned().unwrap_or_default(),
        ))
    }

    /// "Log in"'s implementation mapping of src/login.rs.
    fn mapping(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let coverage: Value = serde_json::from_str(&std::fs::read_to_string(
            self.root().join("spec/features/login.feature.coverage"),
        )?)?;
        Ok(coverage["scenarios"][0]["testMappings"][0]["implMappings"][0].clone())
    }

    /// The impl-text-unrecorded problem of src/login.rs, after an audit.
    fn unrecorded(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let (_, problems) = self.audit(false)?;
        problems
            .into_iter()
            .find(|problem| {
                problem["code"] == "impl-text-unrecorded" && problem["file"] == "src/login.rs"
            })
            .ok_or_else(|| "no impl-text-unrecorded problem for src/login.rs".into())
    }
}

#[test]
fn a_legacy_mapping_whose_file_is_unchanged_since_its_linking_commit_gets_its_text_recorded(
) -> TestResult {
    // @step Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    let project = Project::git_repo()?;
    project.coverage(json!([legacy()]), json!([]))?;
    project.commit("A")?;

    // @step And src/login.rs is unchanged since commit A
    assert_eq!(project.git(&["status", "--porcelain"])?, "");

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping records the text of lines 10 and 24 as firstLine and lastLine
    let mapping = project.mapping()?;
    assert_eq!(mapping["firstLine"], SIGNATURE, "{mapping}");
    assert_eq!(mapping["lastLine"], "}", "{mapping}");
    Ok(())
}

#[test]
fn a_legacy_mapping_whose_file_changed_above_it_is_reported_not_recorded() -> TestResult {
    // @step Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    let project = Project::git_repo()?;
    project.coverage(json!([legacy()]), json!([]))?;
    let a = project.commit("A")?;

    // @step And two lines were added above line 10 of src/login.rs after commit A
    project.write("src/login.rs", &source(11, 2))?;

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping still records no line text
    let mapping = project.mapping()?;
    assert!(mapping.get("firstLine").is_none(), "{mapping}");
    assert!(mapping.get("lastLine").is_none(), "{mapping}");

    // @step And fspec audit reports it as impl-text-unrecorded with fix kind "guided"
    let problem = project.unrecorded()?;
    assert_eq!(problem["fix"]["kind"], "guided", "{problem}");

    // @step And its fact says src/login.rs changed since the mapping was linked in commit A
    let fact = problem["fact"].as_str().unwrap_or_default();
    assert!(
        fact.contains(&format!(
            "src/login.rs lines 1-24 changed since the mapping was linked in commit {a}"
        )),
        "{fact}"
    );
    Ok(())
}

#[test]
fn lines_added_only_after_the_mapping_do_not_stop_the_recording() -> TestResult {
    // @step Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    let project = Project::git_repo()?;
    project.coverage(json!([legacy()]), json!([]))?;
    project.commit("A")?;

    // @step And lines were added only after line 24 of src/login.rs since commit A
    project.write("src/login.rs", &source(9, 5))?;

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping records the text of lines 10 and 24 as firstLine and lastLine
    let mapping = project.mapping()?;
    assert_eq!(mapping["firstLine"], SIGNATURE, "{mapping}");
    assert_eq!(mapping["lastLine"], "}", "{mapping}");
    Ok(())
}

#[test]
fn a_later_commit_of_the_coverage_file_for_another_scenario_keeps_the_linking_commit() -> TestResult
{
    // @step Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    let project = Project::git_repo()?;
    project.coverage(json!([legacy()]), json!([]))?;
    let a = project.commit("A")?;

    // @step And two lines were added above line 10 of src/login.rs after commit A
    project.write("src/login.rs", &source(11, 2))?;

    // @step And commit B changes only the mapping of scenario "Log out" in the same coverage file
    project.write("src/logout.rs", "pub fn log_out() {\n}\n")?;
    project.coverage(
        json!([legacy()]),
        json!([{"file": "src/logout.rs", "lines": [1, 2],
                "firstLine": "pub fn log_out() {", "lastLine": "}"}]),
    )?;
    project.commit("B")?;

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping still records no line text
    let mapping = project.mapping()?;
    assert!(mapping.get("firstLine").is_none(), "{mapping}");

    // @step And its fact names commit A as the linking commit
    let problem = project.unrecorded()?;
    let fact = problem["fact"].as_str().unwrap_or_default();
    assert!(fact.contains(&format!("linked in commit {a}")), "{fact}");
    Ok(())
}

#[test]
fn an_uncommitted_legacy_mapping_is_reported_not_recorded() -> TestResult {
    // @step Given the coverage file links "Log in" to src/login.rs lines 10-24 without line text only in the working tree
    let project = Project::git_repo()?;
    project.coverage(json!([]), json!([]))?;
    project.commit("A")?;
    project.coverage(json!([legacy()]), json!([]))?;

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping still records no line text
    let mapping = project.mapping()?;
    assert!(mapping.get("firstLine").is_none(), "{mapping}");

    // @step And its fact says the mapping is not committed
    let problem = project.unrecorded()?;
    let fact = problem["fact"].as_str().unwrap_or_default();
    assert!(fact.contains("the mapping is not committed"), "{fact}");
    assert_eq!(problem["fix"]["kind"], "guided", "{problem}");
    Ok(())
}

#[test]
fn outside_a_git_working_tree_a_legacy_mapping_is_never_recorded() -> TestResult {
    // @step Given a project that is not a git working tree whose coverage links "Log in" to src/login.rs lines 10-24 without line text
    let project = Project::new()?;
    project.coverage(json!([legacy()]), json!([]))?;

    // @step When fspec audit --fix runs
    project.audit(true)?;

    // @step Then the mapping still records no line text
    let mapping = project.mapping()?;
    assert!(mapping.get("firstLine").is_none(), "{mapping}");

    // @step And its fact says the project is not a git working tree
    let problem = project.unrecorded()?;
    let fact = problem["fact"].as_str().unwrap_or_default();
    assert!(
        fact.contains("the project is not a git working tree"),
        "{fact}"
    );
    Ok(())
}
