//! BUG-017 — update-foundation chains to the next field during discovery.
//!
//! Feature: spec/features/update-foundation-doesn-t-chain-to-next-field-during-discovery.feature
//!
//! On the draft path `update-foundation` returns the next unfilled field's
//! `<system-reminder>` (`systemReminder`) and the progress trailer
//! (`nextSteps`); once every field is filled there is no reminder and the
//! trailer names `fspec discover-foundation --finalize`.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The canonical 8-field placeholder draft `discover-foundation` writes.
fn placeholder_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "[QUESTION: What problem does this solve?]",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

fn write_draft(root: &Path, draft: &Value) -> TestResult {
    std::fs::create_dir_all(root.join("spec"))?;
    std::fs::write(
        root.join("spec/foundation.json.draft"),
        serde_json::to_string_pretty(draft)?,
    )?;
    Ok(())
}

fn update_foundation(root: &Path, section: &str, content: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-foundation".to_string(),
        args_json: json!({ "section": section, "content": content }).to_string(),
        project_root: root.to_path_buf(),
    })
}

/// The success envelope of a dispatch.
fn envelope(result: &DispatchResult) -> Result<Value, Box<dyn std::error::Error>> {
    if !result.success {
        return Err(format!("update-foundation failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

fn text<'a>(data: &'a Value, key: &str) -> &'a str {
    data[key].as_str().unwrap_or_default()
}

#[test]
fn chain_to_next_field_after_updating_first_field() -> TestResult {
    // @step Given I have a foundation.json.draft with placeholder fields
    let tmp = tempfile::tempdir()?;
    write_draft(tmp.path(), &placeholder_draft())?;

    // @step When I run `fspec update-foundation projectName "MyProject"`
    let result = update_foundation(tmp.path(), "projectName", "MyProject");

    // @step Then the command should exit with code 0
    let data = envelope(&result)?;

    // @step And the output should contain "Updated \"projectName\" in foundation.json.draft"
    assert_eq!(
        text(&data, "message"),
        "Updated \"projectName\" in foundation.json.draft"
    );

    // @step And the output should contain a system-reminder for Field 2/8
    let reminder = text(&data, "systemReminder");
    assert!(reminder.starts_with("<system-reminder>\n"), "{reminder}");
    assert!(reminder.ends_with("\n</system-reminder>"), "{reminder}");
    assert!(reminder.contains("Field 2/8: "), "{reminder}");

    // @step And the system-reminder should mention "project.vision"
    assert!(
        reminder.contains("Field 2/8: project.vision (elevator pitch)"),
        "{reminder}"
    );

    // @step And the system-reminder should include the command to run next
    assert!(
        reminder.contains("Run: fspec update-foundation projectVision \"your vision\""),
        "{reminder}"
    );
    Ok(())
}

#[test]
fn chain_to_next_field_after_updating_middle_field() -> TestResult {
    // @step Given I have a foundation.json.draft with first field filled
    let tmp = tempfile::tempdir()?;
    let mut draft = placeholder_draft();
    draft["project"]["name"] = json!("MyProject");
    write_draft(tmp.path(), &draft)?;

    // @step When I run `fspec update-foundation projectVision "My vision"`
    let result = update_foundation(tmp.path(), "projectVision", "My vision");

    // @step Then the command should exit with code 0
    let data = envelope(&result)?;

    // @step And the output should contain "Updated \"projectVision\" in foundation.json.draft"
    assert_eq!(
        text(&data, "message"),
        "Updated \"projectVision\" in foundation.json.draft"
    );

    // @step And the output should contain a system-reminder for Field 3/8
    let reminder = text(&data, "systemReminder");
    assert!(reminder.starts_with("<system-reminder>\n"), "{reminder}");
    assert!(reminder.contains("Field 3/8: "), "{reminder}");

    // @step And the system-reminder should mention "project.projectType"
    assert!(
        reminder.contains("Field 3/8: project.projectType"),
        "{reminder}"
    );

    // @step And the system-reminder should include the command to run next
    assert!(
        reminder.contains("Run: fspec update-foundation projectType \"<type>\""),
        "{reminder}"
    );
    Ok(())
}

#[test]
fn show_the_finalize_command_when_all_fields_complete() -> TestResult {
    // @step Given I have a foundation.json.draft with all fields except last filled
    let tmp = tempfile::tempdir()?;
    let draft = json!({
        "version": "2.0.0",
        "project": { "name": "MyProject", "vision": "My vision", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": {
                "title": "Specs drift",
                "description": "Specs drift from the code",
                "impact": "high"
            }
        },
        "solutionSpace": {
            "overview": "[QUESTION: What can users DO?]",
            "capabilities": [{ "name": "Spec Validation", "description": "Validate specs" }]
        },
        "personas": [
            { "name": "Developer", "description": "Builds features", "goals": ["Ship"] }
        ]
    });
    write_draft(tmp.path(), &draft)?;

    // @step When I run `fspec update-foundation solutionOverview "My solution"`
    let result = update_foundation(tmp.path(), "solutionOverview", "My solution");

    // @step Then the command should exit with code 0
    let data = envelope(&result)?;

    // @step And the output should contain "Updated \"solutionOverview\" in foundation.json.draft"
    assert_eq!(
        text(&data, "message"),
        "Updated \"solutionOverview\" in foundation.json.draft"
    );

    // @step And the result should carry no system-reminder and its nextSteps should say "8/8 fields complete"
    assert!(data.get("systemReminder").is_none(), "{data}");
    let next_steps = text(&data, "nextSteps");
    assert!(
        next_steps.starts_with("progress: 8/8 fields complete | remaining: none\n"),
        "{next_steps}"
    );

    // @step And the nextSteps should include "next: fspec discover-foundation --finalize"
    assert!(
        next_steps.ends_with("next: fspec discover-foundation --finalize"),
        "{next_steps}"
    );
    Ok(())
}
