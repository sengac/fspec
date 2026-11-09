//! FEAT-012 — generate-scenarios writes the work unit's architecture notes
//! into the feature docstring, grouped by their prefix, instead of the TODO
//! placeholders.
//!
//! Feature: spec/features/architecture-notes-in-example-mapping.feature
//!
//! Code: rust/fspec-core/src/commands/generate_scenarios.rs
//! (build_architecture_docstring, categorize_architecture_notes).

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A board whose story WORK-001 ("Gherkin Validation") passes every
/// generate-scenarios gate and carries the architecture notes `notes`.
fn board(dir: &Path, notes: &[&str]) -> TestResult {
    let notes: Vec<Value> = notes
        .iter()
        .enumerate()
        .map(|(id, text)| json!({"id": id, "text": text, "deleted": false}))
        .collect();
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "workUnits": {"WORK-001": {
                "id": "WORK-001", "title": "Gherkin Validation", "type": "story",
                "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z",
                "rules": [{"id": 0, "text": "Invalid Gherkin is reported", "deleted": false}],
                "examples": [{"id": 0, "text": "A missing Feature line is reported", "deleted": false}],
                "userStory": {"role": "developer", "action": "validate feature files", "benefit": "catch syntax errors"},
                "architectureNotes": notes,
            }},
            "states": {
                "backlog": [], "specifying": ["WORK-001"], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            },
        }))?,
    )?;
    Ok(())
}

/// Run generate-scenarios for WORK-001; the docstring lines (fences
/// included) of the feature file it created.
fn generate(dir: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "generate-scenarios".to_string(),
        args_json: json!({"workUnitId": "WORK-001"}).to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("generate-scenarios failed: {result:?}").into());
    }
    let content = std::fs::read_to_string(dir.join("spec/features/gherkin-validation.feature"))?;
    let mut docstring = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if line.trim() == "\"\"\"" {
            docstring.push(line.to_string());
            if inside {
                return Ok(docstring);
            }
            inside = true;
        } else if inside {
            docstring.push(line.to_string());
        }
    }
    Err(format!("no closed docstring in:\n{content}").into())
}

#[test]
fn generate_scenarios_populates_docstring_with_architecture_notes() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit with architecture notes captured
    // @step And the work unit has the note "Uses @cucumber/gherkin parser"
    // @step And the work unit has the note "Must complete validation within 2 seconds"
    board(
        dir,
        &[
            "Uses @cucumber/gherkin parser",
            "Must complete validation within 2 seconds",
        ],
    )?;

    // @step When I run "fspec generate-scenarios WORK-001"
    let docstring = generate(dir)?;

    // @step Then the generated feature file should have a docstring
    assert_eq!(docstring.first().map(String::as_str), Some("  \"\"\""));
    assert_eq!(docstring.last().map(String::as_str), Some("  \"\"\""));

    // @step And the docstring should contain "Uses @cucumber/gherkin parser"
    assert!(
        docstring.contains(&"  Uses @cucumber/gherkin parser".to_string()),
        "{docstring:?}"
    );

    // @step And the docstring should contain "Must complete validation within 2 seconds"
    assert!(
        docstring.contains(&"  Must complete validation within 2 seconds".to_string()),
        "{docstring:?}"
    );

    // @step And the docstring should NOT contain placeholder text
    assert!(
        docstring.iter().all(|line| !line.contains("TODO")),
        "{docstring:?}"
    );
    assert_eq!(
        docstring.len(),
        4,
        "only the two notes between the fences: {docstring:?}"
    );
    Ok(())
}

#[test]
fn generated_docstring_organizes_notes_by_category() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have architecture notes with natural prefixes
    // @step And I have note "Dependency: @cucumber/gherkin parser"
    // @step And I have note "Performance: Must complete within 2 seconds"
    // @step And I have note "Refactoring: Share validation logic with formatter"
    board(
        dir,
        &[
            "Dependency: @cucumber/gherkin parser",
            "Performance: Must complete within 2 seconds",
            "Refactoring: Share validation logic with formatter",
        ],
    )?;

    // @step When I run "fspec generate-scenarios WORK-001"
    let docstring = generate(dir)?;

    // @step Then the docstring should group notes by detected prefix
    // @step And dependency notes should appear first
    // @step And performance notes should appear in their own section
    assert_eq!(
        docstring,
        vec![
            "  \"\"\"",
            "  Dependency:",
            "  - @cucumber/gherkin parser",
            "  Performance:",
            "  - Must complete within 2 seconds",
            "  Refactoring:",
            "  - Share validation logic with formatter",
            "  \"\"\"",
        ]
    );
    Ok(())
}
