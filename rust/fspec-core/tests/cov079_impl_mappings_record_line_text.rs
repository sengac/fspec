//! COV-079 — implementation mappings record their first and last line text.
//!
//! Feature: spec/features/implementation-mappings-record-their-first-and-last-line-text.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n\n  Scenario: Log out\n    Given a user\n    When they log out\n    Then they see the login page\n";

const TEST_FILE: &str = "// @step Given a user\n// @step When they log in\n// @step Then they see the board\n// @step Given a user\n// @step When they log out\n// @step Then they see the login page\n";

const SIGNATURE: &str = "pub fn log_in(user: &User) -> Session {";

/// `before` filler lines, then the 15-line log_in function (signature, 13
/// body lines, closing brace), then `after` filler lines.
fn source(before: usize, body: usize, after: usize) -> String {
    let mut lines: Vec<String> = (1..=before).map(|n| format!("// filler {n}")).collect();
    lines.push(SIGNATURE.to_string());
    lines.extend((1..=body).map(|n| format!("    step_{n}(user);")));
    lines.push("}".to_string());
    lines.extend((1..=after).map(|n| format!("fn helper_{n}() {{\n}}")));
    format!("{}\n", lines.join("\n"))
}

struct Project(tempfile::TempDir);

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/features/login.feature", FEATURE)?;
        project.write("tests/login_test.rs", TEST_FILE)?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    /// BUG-591: commit every file in a new git repository, so the legacy
    /// mappings' lines are verified by git before --fix records their text.
    fn commit_all(&self) -> TestResult {
        for args in [
            &["init", "-q"][..],
            &["add", "-A"][..],
            &["commit", "-q", "-m", "legacy mappings"][..],
        ] {
            let status = std::process::Command::new("git")
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
                .status()?;
            if !status.success() {
                return Err(format!("git {args:?} failed").into());
            }
        }
        Ok(())
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

    /// A coverage file whose scenario "Log in" maps tests/login_test.rs:1-3
    /// with `impls`, and "Log out" maps 4-6 without implementations.
    fn coverage(&self, impls: Value) -> std::io::Result<()> {
        self.write(
            "spec/features/login.feature.coverage",
            &json!({"scenarios": [
                {"name": "Log in", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": "1-3", "implMappings": impls}
                ]},
                {"name": "Log out", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": "4-6", "implMappings": []}
                ]}
            ]})
            .to_string(),
        )
    }

    fn sidecar(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(
            &self.read("spec/features/login.feature.coverage")?,
        )?)
    }
}

fn dispatch(root: &Path, command: &str, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn link_impl(root: &Path, file: &str, lines: &str) -> codelet_fspec_core::DispatchResult {
    dispatch(
        root,
        "link-coverage",
        json!({"featureName": "login", "scenario": "Log in", "testFile": "tests/login_test.rs",
               "implFile": file, "implLines": lines}),
    )
}

/// `fspec audit --area coverage` (with `--fix` when `fix`): exit code and
/// output.
fn audit(root: &Path, fix: bool) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch(
        root,
        "audit",
        json!({"feature": ["login"], "area": ["coverage"], "fix": fix}),
    );
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let envelope: Value = serde_json::from_str(&result.data)?;
    Ok((
        envelope["exitCode"].clone(),
        envelope["output"].as_str().unwrap_or_default().to_string(),
    ))
}

fn impl_mapping(sidecar: &Value, index: usize) -> Value {
    sidecar["scenarios"][0]["testMappings"][0]["implMappings"][index].clone()
}

fn lines(from: i64, to: i64) -> Value {
    json!((from..=to).collect::<Vec<i64>>())
}

#[test]
fn link_coverage_records_the_first_and_last_line_text() -> TestResult {
    // @step Given src/login.rs whose lines 10 to 24 hold the log_in function
    let project = Project::new()?;
    project.write("src/login.rs", &source(9, 13, 2))?;
    project.coverage(json!([]))?;
    // @step When the implementation lines 10-24 are linked to scenario "Log in"
    let result = link_impl(project.root(), "src/login.rs", "10-24");
    assert!(result.success, "{result:?}");
    // @step Then the mapping records firstLine "pub fn log_in(user: &User) -> Session {" and lastLine "}"
    let mapping = impl_mapping(&project.sidecar()?, 0);
    assert_eq!(mapping["firstLine"], SIGNATURE, "{mapping}");
    assert_eq!(mapping["lastLine"], "}", "{mapping}");
    assert_eq!(mapping["lines"], lines(10, 24), "{mapping}");
    Ok(())
}

#[test]
fn a_blank_first_line_is_refused() -> TestResult {
    // @step Given src/login.rs whose line 9 is blank
    let project = Project::new()?;
    project.write(
        "src/login.rs",
        &source(9, 13, 2).replace("// filler 9\n", "\n"),
    )?;
    project.coverage(json!([]))?;
    let before = project.read("spec/features/login.feature.coverage")?;
    // @step When the implementation lines 9-24 are linked to scenario "Log in"
    let result = link_impl(project.root(), "src/login.rs", "9-24");
    // @step Then link-coverage fails naming the blank first line
    assert!(!result.success, "{result:?}");
    let error = result.error.unwrap_or_default();
    assert!(error.contains("src/login.rs line 9 is blank"), "{error}");
    // @step And the coverage file is unchanged
    assert_eq!(
        project.read("spec/features/login.feature.coverage")?,
        before
    );
    Ok(())
}

#[test]
fn a_first_line_that_is_not_unique_is_refused() -> TestResult {
    // @step Given src/login.rs whose line 24 reads "}" like other lines of the file
    let project = Project::new()?;
    project.write("src/login.rs", &source(9, 13, 3))?;
    project.coverage(json!([]))?;
    let before = project.read("spec/features/login.feature.coverage")?;
    // @step When the implementation lines 24-30 are linked to scenario "Log in"
    let result = link_impl(project.root(), "src/login.rs", "24-30");
    // @step Then link-coverage fails naming how often the first line occurs
    assert!(!result.success, "{result:?}");
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("src/login.rs line 24 ('}') occurs 4 times"),
        "{error}"
    );
    // @step And the coverage file is unchanged
    assert_eq!(
        project.read("spec/features/login.feature.coverage")?,
        before
    );
    Ok(())
}

#[test]
fn a_mapping_whose_code_moved_unchanged_is_shifted_by_fix() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 whose function moved unchanged to lines 30-44
    let project = Project::new()?;
    project.write("src/login.rs", &source(9, 13, 2))?;
    project.coverage(json!([{"file": "src/login.rs", "lines": lines(10, 24),
        "firstLine": SIGNATURE, "lastLine": "}"}]))?;
    // BUG-635: --fix carries a mapping through the git line diff from the
    // commit that linked it.
    project.commit_all()?;
    project.write("src/login.rs", &source(29, 13, 2))?;
    // @step When fspec audit runs
    let (exit, output) = audit(project.root(), false)?;
    // @step Then it reports "coverage impl-text-changed" for src/login.rs:10-24
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("coverage impl-text-changed login \"Log in\" src/login.rs:10-24: "),
        "{output}"
    );
    // @step When fspec audit --fix runs
    let (exit, output) = audit(project.root(), true)?;
    // @step Then the mapping becomes src/login.rs 30-44
    let mapping = impl_mapping(&project.sidecar()?, 0);
    assert_eq!(mapping["lines"], lines(30, 44), "{mapping}");
    // @step And it prints "src/login.rs:10-24 -> src/login.rs:30-44"
    assert!(
        output.contains("src/login.rs:10-24 -> src/login.rs:30-44"),
        "{output}"
    );
    assert_eq!(exit, 0, "{output}");
    // @step And a second fspec audit --fix changes nothing
    let before = project.read("spec/features/login.feature.coverage")?;
    let (exit, output) = audit(project.root(), true)?;
    assert_eq!(exit, 0, "{output}");
    assert!(!output.contains(" -> src/"), "{output}");
    assert_eq!(
        project.read("spec/features/login.feature.coverage")?,
        before
    );
    Ok(())
}

#[test]
fn a_mapping_whose_code_moved_and_grew_is_not_shifted() -> TestResult {
    // @step Given a mapping src/login.rs 10-24 whose function moved to lines 30-47 and grew by 3 lines
    let project = Project::new()?;
    project.write("src/login.rs", &source(29, 16, 2))?;
    project.coverage(json!([{"file": "src/login.rs", "lines": lines(10, 24),
        "firstLine": SIGNATURE, "lastLine": "}"}]))?;
    // @step When fspec audit --fix runs
    audit(project.root(), true)?;
    // @step Then the mapping stays src/login.rs 10-24
    let mapping = impl_mapping(&project.sidecar()?, 0);
    assert_eq!(mapping["lines"], lines(10, 24), "{mapping}");
    // @step And fspec audit still reports "coverage impl-text-changed" naming the link-coverage command
    let (exit, output) = audit(project.root(), false)?;
    assert_eq!(exit, 1, "{output}");
    let line = output
        .lines()
        .find(|line| {
            line.starts_with("coverage impl-text-changed login \"Log in\" src/login.rs:10-24: ")
        })
        .ok_or_else(|| format!("no impl-text-changed line:\n{output}"))?;
    // AUDIT-004: choosing the new lines needs judgment, so the problem is
    // guided: the line names the audit session, whose step (the JSON
    // fix.step) names the link-coverage command.
    assert!(
        line.ends_with("-> fspec audit --start --feature login"),
        "{line}"
    );
    let result = dispatch(
        project.root(),
        "audit",
        json!({"feature": ["login"], "area": ["coverage"]}),
    );
    let envelope: Value = serde_json::from_str(&result.data)?;
    let problem = envelope["problems"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|problem| problem["code"] == "impl-text-changed")
        .ok_or_else(|| format!("no impl-text-changed problem: {envelope}"))?;
    assert_eq!(problem["fix"]["kind"], "guided", "{problem}");
    assert_eq!(
        problem["fix"]["step"],
        "fspec link-coverage login --scenario \"Log in\" --test-file tests/login_test.rs --impl-file src/login.rs --impl-lines <lines>",
        "{problem}"
    );
    Ok(())
}

#[test]
fn legacy_mappings_get_their_text_recorded_only_when_the_first_line_is_unique() -> TestResult {
    // @step Given a legacy mapping src/login.rs 10-24 without text whose line 10 is unique
    let project = Project::new()?;
    project.write("src/login.rs", &source(9, 13, 2))?;
    // @step And a legacy mapping src/login.rs 9-24 without text whose line 9 is blank
    project.write(
        "src/blank.rs",
        &source(9, 13, 2).replace("// filler 9\n", "\n"),
    )?;
    project.coverage(json!([
        {"file": "src/login.rs", "lines": lines(10, 24)},
        {"file": "src/blank.rs", "lines": lines(9, 24)}
    ]))?;
    project.commit_all()?;
    // @step When fspec audit --fix runs
    audit(project.root(), true)?;
    // @step Then the first mapping records its firstLine and lastLine
    let sidecar = project.sidecar()?;
    let first = impl_mapping(&sidecar, 0);
    assert_eq!(first["firstLine"], SIGNATURE, "{first}");
    assert_eq!(first["lastLine"], "}", "{first}");
    assert!(impl_mapping(&sidecar, 1).get("firstLine").is_none());
    // @step And fspec audit reports only the second as "coverage impl-text-unrecorded"
    let (exit, output) = audit(project.root(), false)?;
    assert_eq!(exit, 1, "{output}");
    let unrecorded: Vec<&str> = output
        .lines()
        .filter(|line| line.starts_with("coverage impl-text-unrecorded "))
        .collect();
    assert_eq!(unrecorded.len(), 1, "{output}");
    assert!(unrecorded[0].contains(" src/blank.rs:9-24: "), "{output}");
    Ok(())
}
