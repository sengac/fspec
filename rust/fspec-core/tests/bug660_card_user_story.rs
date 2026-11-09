#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/show-work-unit-and-the-implementing-reminder-show-the-card-s-own-user-story.feature
//
// BUG-660: show-work-unit's JSON and text output carry the card's stored user
// story, a malformed stored story is an exact error, and the implementing
// reminder shows the moved card's own story.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

const DEVELOPER_STORY: &str =
    r#"{"role": "developer", "action": "log in", "benefit": "I reach my board"}"#;
const REVIEWER_STORY: &str =
    r#"{"role": "reviewer", "action": "approve a change", "benefit": "only reviewed code ships"}"#;
const ONE_RULE: &str = r#"[{"id": 0, "text": "a rule", "deleted": false}]"#;

/// One card: (id, type, status, extra raw JSON fields without a leading comma).
struct Card<'a> {
    id: &'a str,
    work_type: &'a str,
    status: &'a str,
    extra: &'a str,
}

fn write_board(project_root: &Path, cards: &[Card<'_>]) {
    let units = cards
        .iter()
        .map(|c| {
            let extra = if c.extra.is_empty() {
                String::new()
            } else {
                format!(", {}", c.extra)
            };
            format!(
                r#""{id}": {{ "id": "{id}", "title": "Card {id}", "type": "{t}", "status": "{s}", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"{extra} }}"#,
                id = c.id,
                t = c.work_type,
                s = c.status,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let states = [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ]
    .iter()
    .map(|st| {
        let ids = cards
            .iter()
            .filter(|c| c.status == *st)
            .map(|c| format!("\"{}\"", c.id))
            .collect::<Vec<_>>()
            .join(", ");
        format!("\"{st}\": [{ids}]")
    })
    .collect::<Vec<_>>()
    .join(", ");
    let raw = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
{units}
  }},
  "states": {{ {states} }}
}}"#
    );
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn dispatch(project_root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn show_json(project_root: &Path, id: &str) -> (String, Value) {
    let result = dispatch(
        project_root,
        "show-work-unit",
        json!({"workUnitId": id, "format": "json"}),
    );
    assert!(result.success, "show-work-unit failed: {result:?}");
    let value: Value = serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("show-work-unit JSON did not parse: {e}\n{}", result.data));
    (result.data, value)
}

fn key_pos(raw: &str, key: &str) -> usize {
    raw.find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("key {key} missing from:\n{raw}"))
}

#[test]
fn show_work_unit_json_carries_the_card_user_story_before_its_rules() {
    // Scenario: show-work-unit JSON carries the card's user story before its rules
    let tmp = TempDir::new().expect("tempdir");
    // @step Given work unit "AUTH-001" has the user story role "developer", action "log in" and benefit "I reach my board", and one rule
    let extra = format!(r#""userStory": {DEVELOPER_STORY}, "rules": {ONE_RULE}"#);
    write_board(
        tmp.path(),
        &[Card {
            id: "AUTH-001",
            work_type: "story",
            status: "specifying",
            extra: &extra,
        }],
    );

    // @step When I run show-work-unit for "AUTH-001" with format "json"
    let (raw, value) = show_json(tmp.path(), "AUTH-001");

    // @step Then the JSON field "userStory" is exactly {"role": "developer", "action": "log in", "benefit": "I reach my board"}
    assert_eq!(
        value["userStory"],
        json!({"role": "developer", "action": "log in", "benefit": "I reach my board"})
    );

    // @step And the key "userStory" comes after "status" and before "rules" in the JSON object
    let story = key_pos(&raw, "userStory");
    assert!(
        key_pos(&raw, "status") < story,
        "userStory before status:\n{raw}"
    );
    assert!(
        story < key_pos(&raw, "rules"),
        "userStory after rules:\n{raw}"
    );
}

#[test]
fn show_work_unit_json_has_no_user_story_key_without_a_user_story() {
    // Scenario: show-work-unit JSON has no userStory key for a card without a user story
    let tmp = TempDir::new().expect("tempdir");
    // @step Given work unit "AUTH-002" has no user story
    write_board(
        tmp.path(),
        &[Card {
            id: "AUTH-002",
            work_type: "story",
            status: "specifying",
            extra: "",
        }],
    );

    // @step When I run show-work-unit for "AUTH-002" with format "json"
    let (raw, value) = show_json(tmp.path(), "AUTH-002");

    // @step Then the JSON object has no "userStory" key
    let object = value.as_object().expect("JSON object");
    assert!(
        !object.contains_key("userStory"),
        "unexpected userStory:\n{raw}"
    );
}

#[test]
fn show_work_unit_text_prints_the_user_story_line_before_the_rules() {
    // Scenario: show-work-unit text output prints the user story line before the rules
    let tmp = TempDir::new().expect("tempdir");
    // @step Given work unit "AUTH-001" has the user story role "developer", action "log in" and benefit "I reach my board", and one rule
    let extra = format!(r#""userStory": {DEVELOPER_STORY}, "rules": {ONE_RULE}"#);
    write_board(
        tmp.path(),
        &[Card {
            id: "AUTH-001",
            work_type: "story",
            status: "specifying",
            extra: &extra,
        }],
    );

    // @step When I run show-work-unit for "AUTH-001" with the default text format
    let result = dispatch(
        tmp.path(),
        "show-work-unit",
        json!({"workUnitId": "AUTH-001"}),
    );
    assert!(result.success, "show-work-unit failed: {result:?}");
    let lines: Vec<&str> = result.data.lines().collect();

    // @step Then the output has the line "User Story: As a developer, I want to log in, so that I reach my board"
    let story = lines
        .iter()
        .position(|l| {
            *l == "User Story: As a developer, I want to log in, so that I reach my board"
        })
        .unwrap_or_else(|| panic!("no user story line in:\n{}", result.data));

    // @step And that line comes before the "Rules:" line
    let rules = lines
        .iter()
        .position(|l| *l == "Rules:")
        .unwrap_or_else(|| panic!("no Rules: line in:\n{}", result.data));
    assert!(story < rules, "user story after rules:\n{}", result.data);
}

#[test]
fn show_work_unit_fails_with_an_exact_error_for_a_malformed_user_story() {
    // Scenario: show-work-unit fails with an exact error for a malformed stored user story
    let tmp = TempDir::new().expect("tempdir");
    // @step Given work unit "AUTH-003" has the user story stored as the string "legacy"
    write_board(
        tmp.path(),
        &[Card {
            id: "AUTH-003",
            work_type: "story",
            status: "specifying",
            extra: r#""userStory": "legacy""#,
        }],
    );

    // @step When I run show-work-unit for "AUTH-003"
    let result = dispatch(
        tmp.path(),
        "show-work-unit",
        json!({"workUnitId": "AUTH-003"}),
    );

    // @step Then the command fails with "Work unit AUTH-003 has a malformed userStory field: expected an object with string role, action and benefit. Fix spec/work-units.json."
    assert!(!result.success, "expected failure, got {result:?}");
    let error = result.error.expect("error message");
    assert!(
        error.contains(
            "Work unit AUTH-003 has a malformed userStory field: expected an object with string role, action and benefit. Fix spec/work-units.json."
        ),
        "unexpected error: {error}"
    );
}

#[test]
fn the_implementing_reminder_shows_the_moved_card_own_user_story() {
    // Scenario: The implementing reminder shows the moved card's own user story
    let tmp = TempDir::new().expect("tempdir");
    // @step Given task "AUTH-001" in testing has the user story role "developer", action "log in" and benefit "I reach my board"
    let developer = format!(r#""userStory": {DEVELOPER_STORY}"#);
    // @step And task "AUTH-002" in testing has the user story role "reviewer", action "approve a change" and benefit "only reviewed code ships"
    let reviewer = format!(r#""userStory": {REVIEWER_STORY}"#);
    write_board(
        tmp.path(),
        &[
            Card {
                id: "AUTH-001",
                work_type: "task",
                status: "testing",
                extra: &developer,
            },
            Card {
                id: "AUTH-002",
                work_type: "task",
                status: "testing",
                extra: &reviewer,
            },
        ],
    );

    // @step When I move "AUTH-002" to implementing
    let result = dispatch(
        tmp.path(),
        "update-work-unit-status",
        json!({"workUnitId": "AUTH-002", "status": "implementing"}),
    );
    assert!(result.success, "move failed: {result:?}");
    let output = format!(
        "{}\n{}",
        result.data,
        result.system_reminder.clone().unwrap_or_default()
    );

    // @step Then the output has the line 'USER STORY: "As a reviewer, I want to approve a change, so that only reviewed code ships"'
    assert!(
        output.lines().any(|l| l
            == r#"USER STORY: "As a reviewer, I want to approve a change, so that only reviewed code ships""#),
        "no reviewer story line in:\n{output}"
    );

    // @step And the output does not contain "As a developer, I want to log in"
    assert!(
        !output.contains("As a developer, I want to log in"),
        "another card's story leaked into:\n{output}"
    );
}
