//! BUG-090 — Answered questions display as 'true' instead of actual answer text.
//!
//! Feature: spec/features/answered-questions-display-as-true-instead-of-actual-answer-text.feature
//!
//! answer-question stores the answer text in `question.answer` (with
//! `answered: true`), and generate-scenarios writes it to the example-mapping
//! comments as `#   A: <answer>`.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: &str = "AUTH-001";

/// The context-only feature file generate-scenarios writes for AUTH-001
/// ("Data Caching").
const GENERATED: &str = "spec/features/data-caching.feature";

/// A temporary project holding story AUTH-001 in specifying with a rule, an
/// example, a user story and one open question `question`.
struct Project(tempfile::TempDir);

impl Project {
    fn new(question: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        let mut states = serde_json::Map::new();
        for state in [
            "backlog",
            "specifying",
            "testing",
            "implementing",
            "validating",
            "done",
            "blocked",
        ] {
            let ids: Vec<&str> = if state == "specifying" {
                vec![ID]
            } else {
                vec![]
            };
            states.insert(state.to_string(), json!(ids));
        }
        let unit = json!({
            "id": ID, "title": "Data Caching", "type": "story", "status": "specifying",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z",
            "rules": [{"id": 0, "text": "Cached data expires", "deleted": false}],
            "examples": [{"id": 0, "text": "A dashboard reload reads the cache", "deleted": false}],
            "userStory": {"role": "dashboard user", "action": "see data quickly",
                          "benefit": "I wait less"},
            "questions": [{"id": 0, "text": question, "deleted": false,
                           "createdAt": "2026-06-01T00:00:00.000Z"}],
        });
        let path = project.0.path().join("spec/work-units.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            json!({"version": "0.7.1", "workUnits": {ID: unit}, "states": states}).to_string(),
        )?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn dispatch(&self, command: &str, args: Value) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: command.to_string(),
            args_json: args.to_string(),
            project_root: self.root().to_path_buf(),
        })
    }

    /// Answer question 0 with `answer`, adding it to no rule or assumption.
    fn answer(&self, answer: &str) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.dispatch(
            "answer-question",
            json!({"workUnitId": ID, "index": 0, "answer": answer, "addTo": "none"}),
        );
        if !result.success {
            return Err(format!("answer-question failed: {result:?}").into());
        }
        Ok(())
    }

    /// Run generate-scenarios; the generated feature file's text.
    fn generate(&self) -> Result<String, Box<dyn std::error::Error>> {
        let result = self.dispatch("generate-scenarios", json!({"workUnitId": ID}));
        if !result.success {
            return Err(format!("generate-scenarios failed: {result:?}").into());
        }
        Ok(std::fs::read_to_string(self.root().join(GENERATED))?)
    }

    /// Question 0 as stored in spec/work-units.json.
    fn question(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(self.root().join("spec/work-units.json"))?;
        let data: Value = serde_json::from_str(&raw)?;
        Ok(data["workUnits"][ID]["questions"][0].clone())
    }
}

#[test]
fn store_answer_text_in_question_answer_field() -> TestResult {
    // @step Given a work unit with a question "@human: When should data be cached?"
    let project = Project::new("@human: When should data be cached?")?;

    // @step When I answer the question with "Cache for 24 hours with file-based persistence"
    project.answer("Cache for 24 hours with file-based persistence")?;
    let question = project.question()?;

    // @step Then the question.answered field should be true
    assert_eq!(question["answered"], json!(true), "{question}");

    // @step And the question.answer field should contain "Cache for 24 hours with file-based persistence"
    assert_eq!(
        question["answer"],
        json!("Cache for 24 hours with file-based persistence")
    );

    // @step And the question.answer should NOT be the boolean true
    assert!(question["answer"].is_string(), "{question}");
    Ok(())
}

#[test]
fn display_answer_text_in_feature_file_comments() -> TestResult {
    // @step Given a work unit with an answered question
    // @step And the answer text is "Data should be cached immediately on first access"
    let project = Project::new("@human: When should data be cached?")?;
    project.answer("Data should be cached immediately on first access")?;

    // @step When I generate scenarios
    let generated = project.generate()?;

    // @step Then the feature file should contain "A: Data should be cached immediately on first access"
    assert!(
        generated.contains("  #   Q: When should data be cached?\n  #   A: Data should be cached immediately on first access\n"),
        "{generated}"
    );

    // @step And the feature file should NOT contain "A: true"
    assert!(!generated.contains("A: true"), "{generated}");
    Ok(())
}

const MULTILINE: &str = "Caching strategy:\n1. Cache on first access\n2. Use file-based persistence\n3. TTL of 24 hours";

#[test]
fn preserve_multiline_answer_text() -> TestResult {
    // @step Given a work unit with a question
    let project = Project::new("@human: What is the caching strategy?")?;

    // @step When I answer with multiline text:
    project.answer(MULTILINE)?;

    // @step Then the answer text should be stored exactly as provided
    assert_eq!(project.question()?["answer"], json!(MULTILINE));

    // @step And the generated feature file should contain every line of the answer
    let generated = project.generate()?;
    assert!(
        generated.contains("  #   A: Caching strategy:\n"),
        "{generated}"
    );
    for line in MULTILINE.lines() {
        assert!(
            generated
                .lines()
                .any(|generated_line| generated_line.ends_with(line)),
            "line '{line}' is missing: {generated}"
        );
    }
    Ok(())
}
