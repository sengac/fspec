//! REMIND-017 — add-tag-to-feature and generate-scenarios hand back one
//! consolidated `<system-reminder>` block, however many reminders apply.
//!
//! Feature: spec/features/multiple-system-reminder-blocks-emitted-instead-of-single-consolidated-block.feature
//!
//! add-tag-to-feature returns JSON whose `systemReminder` field is what the
//! CLI prints (rust/fspec/src/add_tag_to_feature.rs); generate-scenarios
//! returns its text output with the block appended.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn dispatch(dir: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    })
}

/// The content of the only `<system-reminder>` block in `text`, after
/// checking there is exactly one opening tag.
fn single_block(text: &str) -> Result<&str, Box<dyn std::error::Error>> {
    assert_eq!(text.matches("<system-reminder>").count(), 1, "{text}");
    let start = text
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{text}"))?;
    let end = text
        .find("</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{text}"))?;
    Ok(&text[start..end])
}

#[test]
fn add_tag_to_feature_consolidates_unregistered_tag_reminders_into_single_block() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given a feature file and three unregistered tags
    std::fs::create_dir_all(dir.join("spec/features"))?;
    std::fs::write(
        dir.join("spec/features/login.feature"),
        "Feature: Login\n\n  Scenario: Sign in\n    Given a user\n    When they sign in\n    Then they see the dashboard\n",
    )?;
    std::fs::write(
        dir.join("spec/tags.json"),
        serde_json::to_string_pretty(&json!({
            "categories": [{"name": "Phase Tags", "tags": [{"name": "@phase1", "description": "Phase 1"}]}]
        }))?,
    )?;
    let tags = ["@alpha", "@beta", "@gamma"];

    // @step When I run add-tag-to-feature without validate-registry
    let result = dispatch(
        dir,
        "add-tag-to-feature",
        json!({"file": "spec/features/login.feature", "tags": tags}),
    );
    if !result.success {
        return Err(format!("add-tag-to-feature failed: {result:?}").into());
    }
    let response: Value = serde_json::from_str(&result.data)?;
    let printed = response["systemReminder"]
        .as_str()
        .ok_or_else(|| format!("no systemReminder in {response}"))?;

    // @step Then the output contains exactly one system-reminder opening tag
    let block = single_block(printed)?;

    // @step Then the block contains all three unregistered tag warnings
    for tag in tags {
        assert!(
            block.contains(&format!(
                "Tag \"{tag}\" is not registered in spec/tags.json."
            )),
            "missing the warning for {tag} in {block}"
        );
    }
    Ok(())
}

#[test]
fn generate_scenarios_consolidates_reminders_into_single_block() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given a work unit with example mapping data that will trigger both generation and prefill reminders
    // No user story: the generated Background keeps its [role] placeholders,
    // which the prefill check reports.
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {"EXP-001": {
                "id": "EXP-001", "title": "Export Report", "type": "story", "status": "specifying",
                "rules": [{"id": 0, "text": "Exports are CSV", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"}],
                "examples": [{"id": 0, "text": "User exports the monthly report", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"}],
                "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": {"backlog": [], "specifying": ["EXP-001"], "testing": [], "implementing": [],
                       "validating": [], "done": [], "blocked": []},
        }))?,
    )?;

    // @step When I run generate-scenarios for that work unit
    let result = dispatch(dir, "generate-scenarios", json!({"workUnitId": "EXP-001"}));
    if !result.success {
        return Err(format!("generate-scenarios failed: {result:?}").into());
    }

    // @step Then the output contains exactly one system-reminder opening tag
    let block = single_block(&result.data)?;

    // @step Then the block contains both the generation guidance and prefill detection content
    assert!(
        block.contains("CONTEXT-ONLY FEATURE FILE CREATED"),
        "{block}"
    );
    assert!(
        block.contains("PREFILL DETECTED in feature file."),
        "{block}"
    );
    Ok(())
}
