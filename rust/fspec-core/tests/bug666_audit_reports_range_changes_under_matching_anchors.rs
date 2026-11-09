//! BUG-666 — fspec audit reports a mapping whose range changed under
//! anchor lines that still read their recorded text.
//!
//! Feature: spec/features/audit-reports-a-mapping-whose-range-changed-under-matching-anchor-lines.feature

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

fn problem_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| {
            line.starts_with("coverage impl-range-changed login \"Log in\" src/login.rs:10-24: ")
        })
        .collect()
}

#[test]
fn code_inserted_inside_a_range_whose_anchors_still_read_their_text_is_reported_and_carried(
) -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 whose lines 23 and 24 both read '}'
    let project = Project::committed()?;

    // @step When one line is inserted after line 15
    project.write_source(&insert_after(source(), 15, &["    log(user);"]))?;

    // @step Then fspec audit reports "coverage impl-range-changed" for src/login.rs:10-24 naming lines 10-25
    let output = audit(project.root(), false)?;
    let problems = problem_lines(&output);
    assert_eq!(problems.len(), 1, "{output}");
    assert!(problems[0].contains("lines 10-25"), "{output}");
    // @step And fspec audit --fix rewrites the mapping to src/login.rs 10-25
    let output = audit(project.root(), true)?;
    assert_eq!(project.mapped_lines()?, lines(10, 25), "{output}");
    assert!(
        output.contains("src/login.rs:10-24 -> src/login.rs:10-25"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_file_unchanged_inside_the_range_is_not_reported() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24
    let project = Project::committed()?;

    // @step When nothing changes, and then two lines are added after line 24
    let unchanged = audit(project.root(), false)?;
    project.write_source(&insert_after(source(), 24, &["// after 1", "// after 2"]))?;
    let after = audit(project.root(), false)?;

    // @step Then fspec audit reports no problem for the mapping either time
    for output in [&unchanged, &after] {
        assert!(!output.contains("src/login.rs"), "{output}");
    }
    Ok(())
}

#[test]
fn an_edit_to_the_last_line_under_matching_text_is_reported_not_fixed() -> TestResult {
    // @step Given a committed mapping src/login.rs 10-24 whose line 24 reads '}'
    let project = Project::committed()?;

    // @step When line 24 is re-indented and still reads '}' when trimmed
    let mut edited = source();
    edited[23] = "  }".to_string();
    project.write_source(&edited)?;

    // @step Then fspec audit reports "coverage impl-range-changed" saying an edit changed its last line 24
    let output = audit(project.root(), false)?;
    let problems = problem_lines(&output);
    assert_eq!(problems.len(), 1, "{output}");
    assert!(
        problems[0].contains("an edit changed its last line 24"),
        "{output}"
    );
    // @step And fspec audit --fix leaves the mapping at src/login.rs 10-24
    audit(project.root(), true)?;
    assert_eq!(project.mapped_lines()?, lines(10, 24));
    Ok(())
}

#[test]
fn an_uncommitted_mapping_keeps_the_text_check_only() -> TestResult {
    // @step Given an uncommitted mapping src/login.rs 10-24 whose lines 23 and 24 both read '}'
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

    // @step When one line is inserted after line 15
    project.write_source(&insert_after(source(), 15, &["    log(user);"]))?;

    // @step Then fspec audit reports no impl-range-changed problem for the mapping
    let output = audit(project.root(), false)?;
    assert!(!output.contains("impl-range-changed"), "{output}");
    Ok(())
}
