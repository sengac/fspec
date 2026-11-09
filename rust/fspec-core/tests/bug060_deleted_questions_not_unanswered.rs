//! BUG-060 — generate-scenarios counts deleted questions as unanswered.
//!
//! Feature: spec/features/generate-scenarios-counts-deleted-questions-as-unanswered.feature
//!
//! A question is unanswered when it is neither deleted nor selected (answered).
//! generate-scenarios and the specifying -> testing move of
//! update-work-unit-status both apply that filter.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: &str = "AUTH-001";

/// The context-only feature file generate-scenarios writes for AUTH-001
/// ("User Authentication").
const GENERATED: &str = "spec/features/user-authentication.feature";

/// A temporary project holding story AUTH-001 in specifying, ready for
/// generate-scenarios and for the move to testing apart from `questions`.
struct Project(tempfile::TempDir);

impl Project {
    fn new(questions: Vec<Value>) -> Result<Self, Box<dyn std::error::Error>> {
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
            "id": ID, "title": "User Authentication", "type": "story", "status": "specifying",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z",
            "estimate": 3,
            "rules": [{"id": 0, "text": "Password must be 8+ characters", "deleted": false}],
            "examples": [{"id": 0, "text": "User views the account settings page", "deleted": false}],
            "userStory": {"role": "registered user", "action": "log in securely",
                          "benefit": "access my account"},
            "architectureNotes": [{"id": 0, "text": "Session tokens live in memory", "deleted": false}],
            "attachments": ["spec/attachments/AUTH-001/ast-research-login.md"],
            "questions": questions,
        });
        project.write(
            "spec/work-units.json",
            &json!({
                "version": "0.7.1",
                "workUnits": {ID: unit},
                "states": states,
            })
            .to_string(),
        )?;
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

    /// The feature file tagged @AUTH-001 that the move to testing needs.
    fn tagged_feature(&self) -> std::io::Result<()> {
        self.write(
            "spec/features/login.feature",
            "@AUTH-001\nFeature: Login\n\n  Scenario: Log in\n    Given a registered user\n    When they log in\n    Then they see their account\n",
        )
    }

    fn dispatch(&self, command: &str, args: Value) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: command.to_string(),
            args_json: args.to_string(),
            project_root: self.root().to_path_buf(),
        })
    }

    fn generate_scenarios(&self) -> DispatchResult {
        self.dispatch("generate-scenarios", json!({"workUnitId": ID}))
    }

    fn move_to_testing(&self) -> DispatchResult {
        self.dispatch(
            "update-work-unit-status",
            json!({"workUnitId": ID, "status": "testing", "skipTemporalValidation": true}),
        )
    }

    fn status(&self) -> Result<String, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(self.root().join("spec/work-units.json"))?;
        let data: Value = serde_json::from_str(&raw)?;
        Ok(data["workUnits"][ID]["status"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

/// `count` questions starting at index `first`, deleted or answered as
/// given; neither makes an unanswered question.
fn questions(first: usize, count: usize, deleted: bool, selected: bool) -> Vec<Value> {
    (first..first + count)
        .map(|index| {
            let mut question = json!({
                "id": index, "text": format!("@human: Question {index}?"),
                "deleted": deleted, "selected": selected,
                "createdAt": "2026-06-01T00:00:00.000Z",
            });
            if selected {
                question["answered"] = json!(true);
                question["answer"] = json!(format!("Answer {index}"));
            }
            question
        })
        .collect()
}

/// 3 deleted questions then 5 answered ones.
fn deleted_and_answered() -> Vec<Value> {
    let mut all = questions(0, 3, true, false);
    all.extend(questions(3, 5, false, true));
    all
}

/// The whole text of a dispatch result.
fn text(result: &DispatchResult) -> String {
    format!(
        "{}\n{}\n{}",
        result.data,
        result.error.clone().unwrap_or_default(),
        result.system_reminder.clone().unwrap_or_default()
    )
}

#[test]
fn deleted_questions_should_not_be_counted_as_unanswered() -> TestResult {
    // @step Given a work unit has 3 deleted questions with deleted=true and selected=false
    // @step And the work unit has 5 answered questions with deleted=false and selected=true
    let project = Project::new(deleted_and_answered())?;

    // @step When I run "fspec generate-scenarios <work-unit-id>"
    let result = project.generate_scenarios();

    // @step Then the validation should count 0 unanswered questions
    assert!(!text(&result).contains("unanswered question"), "{result:?}");

    // @step And the command should succeed and generate scenarios
    assert!(result.success, "{result:?}");
    let generated = std::fs::read_to_string(project.root().join(GENERATED))?;
    assert!(generated.contains("#   A: Answer 7"), "{generated}");

    // @step And the command should not throw "unanswered questions found" error
    assert!(result.error.is_none(), "{result:?}");
    Ok(())
}

#[test]
fn unanswered_non_deleted_questions_should_block_generation() -> TestResult {
    // @step Given a work unit has 2 unanswered questions with deleted=false and selected=false
    let mut all = questions(0, 2, false, false);
    all.extend(questions(2, 1, true, false));
    let project = Project::new(all)?;

    // @step When I run "fspec generate-scenarios <work-unit-id>"
    let result = project.generate_scenarios();

    // @step Then the validation should count 2 unanswered questions
    let error = result.error.clone().unwrap_or_default();
    assert!(
        error.contains("Work unit AUTH-001 has 2 unanswered questions."),
        "{result:?}"
    );

    // @step And the command should fail with "2 unanswered questions found" error
    assert!(!result.success, "{result:?}");
    assert!(error.contains("2 unanswered questions found"), "{error}");

    // @step And the command should not generate scenarios
    assert!(!project.root().join(GENERATED).exists());
    Ok(())
}

#[test]
fn work_unit_with_no_questions_should_allow_generation() -> TestResult {
    // @step Given a work unit has no questions
    let project = Project::new(Vec::new())?;

    // @step When I run "fspec generate-scenarios <work-unit-id>"
    let result = project.generate_scenarios();

    // @step Then the validation should count 0 unanswered questions
    assert!(!text(&result).contains("unanswered question"), "{result:?}");

    // @step And the command should succeed and generate scenarios
    assert!(result.success, "{result:?}");
    assert!(project.root().join(GENERATED).exists());
    Ok(())
}

#[test]
fn deleted_questions_should_not_block_status_transition_in_update_work_unit_status() -> TestResult {
    // @step Given a work unit in "specifying" status has 3 deleted questions with deleted=true and selected=false
    // @step And the work unit has 5 answered questions with deleted=false and selected=true
    let project = Project::new(deleted_and_answered())?;
    project.tagged_feature()?;

    // @step When I run "fspec update-work-unit-status <work-unit-id> testing"
    let result = project.move_to_testing();

    // @step Then the validation should count 0 unanswered questions
    assert!(!text(&result).contains("Unanswered question"), "{result:?}");

    // @step And the command should succeed and update status to "testing"
    assert!(result.success, "{result:?}");
    assert_eq!(project.status()?, "testing");

    // @step And the command should not throw "unanswered questions" error
    assert!(result.error.is_none(), "{result:?}");
    Ok(())
}

#[test]
fn unanswered_non_deleted_questions_should_block_status_transition() -> TestResult {
    // @step Given a work unit in "specifying" status has 2 unanswered questions with deleted=false and selected=false
    let mut all = questions(0, 1, true, false);
    all.extend(questions(1, 2, false, false));
    let project = Project::new(all)?;
    project.tagged_feature()?;

    // @step When I run "fspec update-work-unit-status <work-unit-id> testing"
    let result = project.move_to_testing();

    // @step Then the validation should count 2 unanswered questions
    let error = result.error.clone().unwrap_or_default();
    let listed: Vec<&str> = error
        .lines()
        .filter(|line| line.starts_with("  - ["))
        .collect();
    assert_eq!(
        listed,
        ["  - [1] @human: Question 1?", "  - [2] @human: Question 2?"],
        "{error}"
    );

    // @step And the command should fail with "Unanswered questions prevent state transition" error
    assert!(!result.success, "{result:?}");
    assert!(
        error.contains(
            "Unanswered questions prevent state transition from 'specifying' to 'testing'"
        ),
        "{error}"
    );

    // @step And the status should remain "specifying"
    assert_eq!(project.status()?, "specifying");
    Ok(())
}
