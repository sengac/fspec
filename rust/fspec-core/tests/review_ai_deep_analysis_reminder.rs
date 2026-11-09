//! REV-004 — `fspec review` ends with an AI deep-review reminder: read the
//! implementation files, look for bugs, race conditions and anti-patterns,
//! check FOUNDATION.md and report conversationally.
//!
//! Feature: spec/features/ai-driven-deep-code-review.feature
//!
//! The reminder (`build_ai_analysis_reminder` in commands/review.rs) is
//! generic guidance: STEP 1 lists the implementation files from the
//! feature's coverage, STEPs 2-4 are a fixed checklist. fspec does not
//! analyse the code itself, so the steps assert the guidance the AI gets.
//! Every test sets FSPEC_AGENT=claude (the same value in every test, so
//! parallel tests agree), which wraps the reminder in `<system-reminder>`.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write(dir: &Path, rel: &str, content: &str) -> TestResult {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

/// A project with work unit `id` in validating, a feature tagged with it
/// and a coverage file mapping its one scenario to a test file and to the
/// implementation files `impl_files` (each written with `impl_body`).
fn project(dir: &Path, id: &str, impl_files: &[&str], impl_body: &str) -> TestResult {
    write(
        dir,
        "spec/work-units.json",
        &serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {id: {
                "id": id, "title": "Deep review", "type": "story", "status": "validating",
                "rules": [{"id": 0, "text": "Rule", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"}],
                "examples": [{"id": 0, "text": "Example", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"}],
                "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": {"backlog": [], "specifying": [], "testing": [], "implementing": [],
                       "validating": [id], "done": [], "blocked": []},
        }))?,
    )?;
    write(dir, "spec/fspec-config.json", r#"{"agent": "claude"}"#)?;
    write(
        dir,
        "spec/features/deep-review.feature",
        &format!("@{id}\nFeature: Deep review\n\n  Scenario: Works\n    Given a thing\n    When it runs\n    Then it works\n"),
    )?;
    write(
        dir,
        "tests/deep_review.rs",
        "// @step Given a thing\n// @step When it runs\n// @step Then it works\n",
    )?;
    let impl_mappings: Vec<Value> = impl_files
        .iter()
        .map(|file| json!({"file": file, "lines": [1]}))
        .collect();
    for file in impl_files {
        write(dir, file, impl_body)?;
    }
    write(
        dir,
        "spec/features/deep-review.feature.coverage",
        &serde_json::to_string_pretty(&json!({
            "scenarios": [{"name": "Works", "testMappings": [
                {"file": "tests/deep_review.rs", "lines": "1-3", "implMappings": impl_mappings}
            ]}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
                      "testFiles": ["tests/deep_review.rs"], "implFiles": impl_files, "totalLinesCovered": 3}
        }))?,
    )?;
    Ok(())
}

/// Run `fspec review <id>` and return its output.
fn review(dir: &Path, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::set_var("FSPEC_AGENT", "claude");
    let result = dispatch_command(DispatchRequest {
        command: "review".to_string(),
        args_json: json!({"workUnitId": id}).to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("review failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The content of the `<system-reminder>` block of `output`.
fn reminder(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let start = output
        .find("<system-reminder>\n")
        .ok_or_else(|| format!("no <system-reminder> tag in:\n{output}"))?;
    let end = output
        .find("\n</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{output}"))?;
    Ok(&output[start..end])
}

fn has(block: &str, text: &str) {
    assert!(block.contains(text), "missing {text:?} in:\n{block}");
}

#[test]
fn review_provides_system_reminder_with_ai_instructions_for_deep_analysis() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit "AUTH-001" with linked implementation files in coverage data
    // @step And the implementation file is "src/auth/login.ts"
    project(
        dir,
        "AUTH-001",
        &["src/auth/login.ts"],
        "export function login() {}\n",
    )?;

    // @step When I run "fspec review AUTH-001"
    let output = review(dir, "AUTH-001")?;

    // @step Then the output should contain a <system-reminder> tag
    let block = reminder(&output)?;
    has(block, "AI-DRIVEN DEEP CODE REVIEW");

    // @step And the system-reminder should instruct the AI to read "src/auth/login.ts"
    has(
        block,
        "STEP 1: Read Implementation Files\n\nUse the Read tool to examine the following implementation files:\n  - src/auth/login.ts\n",
    );

    // @step And the system-reminder should instruct the AI to analyze code for bugs and edge cases
    has(block, "STEP 2: Analyze Code for Quality Issues");
    has(block, "Bugs and Logic Errors:");
    has(block, "Incorrect edge case handling");

    // @step And the system-reminder should instruct the AI to check FOUNDATION.md alignment
    has(block, "STEP 3: Check FOUNDATION.md Alignment");

    // @step And the system-reminder should instruct the AI to report findings conversationally
    has(
        block,
        "After your analysis, report findings conversationally:",
    );
    Ok(())
}

#[test]
fn review_detects_duplicated_code_and_suggests_shared_utility() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();
    let files = [
        "src/validate/user.ts",
        "src/validate/order.ts",
        "src/validate/item.ts",
    ];

    // @step Given I have a work unit "VAL-001" with 3 implementation files
    // @step And all 3 files contain similar validation logic
    project(
        dir,
        "VAL-001",
        &files,
        "export function validate(x: string) { return x.length > 0; }\n",
    )?;

    // @step When I run "fspec review VAL-001"
    let output = review(dir, "VAL-001")?;

    // @step Then the output should contain a <system-reminder> tag
    let block = reminder(&output)?;

    // @step And the system-reminder should instruct the AI to detect duplicated validation logic
    has(block, "duplicated code across multiple files");
    has(block, "Repeated validation logic that could be DRY");

    // @step And the system-reminder should suggest extracting similar code to shared utilities
    has(
        block,
        "Similar code that could be extracted to shared utilities",
    );

    // @step And the system-reminder should list the 3 implementation files to read
    has(
        block,
        "following implementation files:\n  - src/validate/user.ts\n  - src/validate/order.ts\n  - src/validate/item.ts\n",
    );
    Ok(())
}

#[test]
fn review_finds_race_condition_and_suggests_foundation_md_pattern() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit "FILE-001" with async file operations
    // @step And the code has potential race conditions
    project(
        dir,
        "FILE-001",
        &["src/file-ops/save.ts"],
        "export async function save() {\n  await writeFile('a', '1');\n  await writeFile('a', '2');\n}\n",
    )?;

    // @step And FOUNDATION.md defines file locking patterns
    write(
        dir,
        "spec/FOUNDATION.md",
        "# Foundation\n\nFile writes take the spec file lock.\n",
    )?;

    // @step When I run "fspec review FILE-001"
    let output = review(dir, "FILE-001")?;

    // @step Then the output should contain a <system-reminder> tag
    let block = reminder(&output)?;

    // @step And the system-reminder should instruct the AI to analyze async operations
    has(block, "Async operations without proper locking");

    // @step And the system-reminder should suggest checking FOUNDATION.md for file locking pattern
    has(
        block,
        "file locking pattern or atomic writes as mentioned in FOUNDATION.md",
    );

    // @step And the system-reminder should guide the AI to detect race conditions
    has(block, "Race Conditions:");
    has(block, "Concurrent access to shared resources");
    Ok(())
}

#[test]
fn review_detects_god_function_anti_pattern() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit "PROC-001" with a 500-line function
    let mut body = String::from("export function process() {\n");
    for line in 0..498 {
        body.push_str(&format!("  step{line}();\n"));
    }
    body.push_str("}\n");
    project(dir, "PROC-001", &["src/process.ts"], &body)?;

    // @step When I run "fspec review PROC-001"
    let output = review(dir, "PROC-001")?;

    // @step Then the output should contain a <system-reminder> tag
    let block = reminder(&output)?;

    // @step And the system-reminder should instruct the AI to detect large functions
    has(block, "large functions that need refactoring");

    // @step And the system-reminder should suggest splitting large functions
    has(block, "large functions that should be split");

    // @step And the system-reminder should mention the God function anti-pattern
    has(block, "Anti-Patterns:\n    - God functions (>100 lines");
    Ok(())
}

#[test]
fn review_checks_foundation_md_file_size_principle() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit "BIG-001" with a file at 450 lines
    let body: String = (0..450)
        .map(|line| format!("const v{line} = {line};\n"))
        .collect();
    project(dir, "BIG-001", &["src/big.ts"], &body)?;

    // @step And FOUNDATION.md states "keep files under 300 lines"
    write(
        dir,
        "spec/FOUNDATION.md",
        "# Foundation\n\nkeep files under 300 lines\n",
    )?;

    // @step When I run "fspec review BIG-001"
    let output = review(dir, "BIG-001")?;

    // @step Then the output should contain a <system-reminder> tag
    let block = reminder(&output)?;

    // @step And the system-reminder should instruct the AI to check file sizes against FOUNDATION.md
    has(
        block,
        "Read FOUNDATION.md or AGENTS.md and verify code follows project principles:\n  - File size limits",
    );

    // @step And the system-reminder should name the 300-line file size limit as an example principle
    has(block, "File size limits (e.g., keep files under 300 lines)");

    // @step And the system-reminder should ask the AI to note FOUNDATION.md violations with the exact principle violated
    has(
        block,
        "Note FOUNDATION.md violations with exact principle violated",
    );
    Ok(())
}
