//! BUG-635 — fspec audit --fix carries implementation mappings through the
//! git line diff from the commit that linked them.
//!
//! Feature: spec/features/audit-fix-carries-implementation-mappings-through-the-git-line-diff.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str =
    "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const TEST_FILE: &str =
    "// @step Given a user\n// @step When they log in\n// @step Then they see the board\n";

const SIGNATURE: &str = "pub fn log_in(user: &User) -> Session {";

/// Lines 1-9 filler, 10 the signature, 11-21 steps, 22 `if done {`, 23 `}`
/// (closing the inner block), 24 `}` (closing log_in), then two helpers.
fn source() -> Vec<String> {
    let mut lines: Vec<String> = (1..=9).map(|n| format!("// filler {n}")).collect();
    lines.push(SIGNATURE.to_string());
    lines.extend((1..=11).map(|n| format!("    step_{n}(user);")));
    lines.push("    if done(user) {".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.extend(["fn helper_1() {", "}", "fn helper_2() {", "}"].map(String::from));
    lines
}

/// `lines` with `new` inserted after (1-based) line `after`.
fn insert_after(mut lines: Vec<String>, after: usize, new: &[&str]) -> Vec<String> {
    for (offset, line) in new.iter().enumerate() {
        lines.insert(after + offset, (*line).to_string());
    }
    lines
}

struct Project(tempfile::TempDir);

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/features/login.feature", FEATURE)?;
        project.write("tests/login_test.rs", TEST_FILE)?;
        project.write_source(&source())?;
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

    fn write_source(&self, lines: &[String]) -> std::io::Result<()> {
        self.write("src/login.rs", &format!("{}\n", lines.join("\n")))
    }

    fn read(&self, rel: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root().join(rel))
    }

    /// Scenario "Log in" maps tests/login_test.rs:1-3 to src/login.rs
    /// `from`-`to`, recording the signature and the closing brace.
    fn mapping(&self, from: i64, to: i64) -> std::io::Result<()> {
        self.write(
            "spec/features/login.feature.coverage",
            &json!({"scenarios": [
                {"name": "Log in", "testMappings": [
                    {"file": "tests/login_test.rs", "lines": "1-3", "implMappings": [
                        {"file": "src/login.rs", "lines": lines(from, to),
                         "firstLine": SIGNATURE, "lastLine": "}"}
                    ]}
                ]}
            ]})
            .to_string(),
        )
    }

    /// Commit every file in a new git repository.
    fn commit_all(&self) -> TestResult {
        for args in [
            &["init", "-q"][..],
            &["add", "-A"][..],
            &["commit", "-q", "-m", "linked"][..],
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

    /// A committed mapping src/login.rs 10-24 for log_in.
    fn committed() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project::new()?;
        project.mapping(10, 24)?;
        project.commit_all()?;
        Ok(project)
    }

    fn mapped_lines(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let sidecar: Value =
            serde_json::from_str(&self.read("spec/features/login.feature.coverage")?)?;
        Ok(sidecar["scenarios"][0]["testMappings"][0]["implMappings"][0]["lines"].clone())
    }
}

fn lines(from: i64, to: i64) -> Value {
    json!((from..=to).collect::<Vec<i64>>())
}

/// `fspec audit --feature login --area coverage` (with `--fix` when `fix`):
/// its output.
fn audit(root: &Path, fix: bool) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["login"], "area": ["coverage"], "fix": fix}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let envelope: Value = serde_json::from_str(&result.data)?;
    Ok(envelope["output"].as_str().unwrap_or_default().to_string())
}

const FILLER: [&str; 3] = ["// added 1", "// added 2", "// added 3"];

#[test]
fn lines_inserted_before_a_committed_mapping_shift_it() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 for the log_in function
    let project = Project::committed()?;

    // @step When 3 lines are inserted after line 4 and fspec audit --fix runs
    project.write_source(&insert_after(source(), 4, &FILLER))?;
    let output = audit(project.root(), true)?;

    // @step Then the mapping becomes src/login.rs 13-27
    assert_eq!(project.mapped_lines()?, lines(13, 27), "{output}");
    assert!(
        output.contains("src/login.rs:10-24 -> src/login.rs:13-27"),
        "{output}"
    );
    Ok(())
}

#[test]
fn lines_inserted_inside_a_committed_mapping_extend_it() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 for the log_in function, whose line 23 closes an inner block with '}'
    let project = Project::committed()?;

    // @step When 3 lines are inserted after line 4 and 1 line after line 15, and fspec audit --fix runs
    let edited = insert_after(source(), 15, &["    log(user);"]);
    project.write_source(&insert_after(edited, 4, &FILLER))?;
    let output = audit(project.root(), true)?;

    // @step Then the mapping becomes src/login.rs 13-28, ending at the function's closing line, not 13-27
    assert_eq!(project.mapped_lines()?, lines(13, 28), "{output}");
    assert!(
        output.contains("src/login.rs:10-24 -> src/login.rs:13-28"),
        "{output}"
    );
    Ok(())
}

#[test]
fn lines_inserted_directly_before_the_first_line_stay_outside_the_mapping() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 for the log_in function
    let project = Project::committed()?;

    // @step When 2 lines are inserted directly before line 10 and fspec audit --fix runs
    project.write_source(&insert_after(source(), 9, &FILLER[..2]))?;
    let output = audit(project.root(), true)?;

    // @step Then the mapping becomes src/login.rs 12-26
    assert_eq!(project.mapped_lines()?, lines(12, 26), "{output}");
    Ok(())
}

#[test]
fn lines_inserted_directly_after_the_last_line_leave_the_mapping_as_it_is() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 for the log_in function
    let project = Project::committed()?;

    // @step When 2 lines are inserted directly after line 24
    project.write_source(&insert_after(source(), 24, &FILLER[..2]))?;

    // @step Then fspec audit reports no problem for the mapping
    let output = audit(project.root(), false)?;
    assert!(!output.contains("src/login.rs"), "{output}");
    assert_eq!(project.mapped_lines()?, lines(10, 24));
    Ok(())
}

#[test]
fn an_edit_to_the_mapping_s_first_line_is_reported_not_fixed() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 for the log_in function
    let project = Project::committed()?;

    // @step When line 4 gains a line after it and the signature on line 10 is edited, and fspec audit --fix runs
    let mut edited = source();
    edited[9] = "pub fn log_in(user: &User, at: Instant) -> Session {".to_string();
    project.write_source(&insert_after(edited, 4, &FILLER[..1]))?;
    audit(project.root(), true)?;

    // @step Then the mapping stays src/login.rs 10-24
    assert_eq!(project.mapped_lines()?, lines(10, 24));
    // @step And fspec audit reports it as impl-text-changed saying an edit changed its first line 10
    let output = audit(project.root(), false)?;
    let line = output
        .lines()
        .find(|line| {
            line.starts_with("coverage impl-text-changed login \"Log in\" src/login.rs:10-24: ")
        })
        .ok_or_else(|| format!("no impl-text-changed line:\n{output}"))?;
    assert!(line.contains("an edit changed its first line 10"), "{line}");
    Ok(())
}

#[test]
fn a_mapping_that_is_not_committed_is_reported_not_shifted() -> TestResult {
    // @step Given an uncommitted mapping src/login.rs 10-24 whose function moved unchanged to lines 30-44
    let project = Project::new()?;
    project.write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": [
            {"file": "tests/login_test.rs", "lines": "1-3", "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    project.commit_all()?;
    project.mapping(10, 24)?;
    let moved: Vec<String> = (1..=20).map(|n| format!("// moved {n}")).collect();
    let moved: Vec<&str> = moved.iter().map(String::as_str).collect();
    project.write_source(&insert_after(source(), 9, &moved))?;

    // @step When fspec audit --fix runs
    audit(project.root(), true)?;

    // @step Then the mapping stays src/login.rs 10-24
    assert_eq!(project.mapped_lines()?, lines(10, 24));
    // @step And fspec audit reports it saying the mapping is not committed, so git cannot show how its lines moved
    let output = audit(project.root(), false)?;
    assert!(
        output.contains("the mapping is not committed, so git cannot show how its lines moved"),
        "{output}"
    );
    Ok(())
}
