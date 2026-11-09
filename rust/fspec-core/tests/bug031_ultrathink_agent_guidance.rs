//! BUG-031 / BUG-048 — ULTRATHINK guidance follows the agent's meta-cognition
//! support in discovery.
//!
//! Feature: spec/features/no-support-for-ultrathink-in-discovery-driven-feedback-loop.feature
//!
//! `discover-foundation` tells an agent with meta-cognition (claude,
//! antigravity) to "ULTRATHINK the entire codebase" and every other agent to
//! "think a lot about the entire codebase"; the project.vision field
//! reminder follows the same split. The agent comes from `FSPEC_AGENT`, so
//! every test here sets or clears it while holding one process-wide lock,
//! and restores the previous value afterwards.

use std::path::Path;
use std::sync::Mutex;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Serializes every `FSPEC_AGENT` change and read in this test binary.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Run `body` with `FSPEC_AGENT` set to `agent` (cleared for `None`).
fn with_agent<T>(agent: Option<&str>, body: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let previous = std::env::var_os("FSPEC_AGENT");
    match agent {
        Some(agent) => std::env::set_var("FSPEC_AGENT", agent),
        None => std::env::remove_var("FSPEC_AGENT"),
    }
    let result = body();
    match previous {
        Some(value) => std::env::set_var("FSPEC_AGENT", value),
        None => std::env::remove_var("FSPEC_AGENT"),
    }
    result
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// The `systemReminder` of a successful dispatch.
fn reminder(result: &DispatchResult) -> Result<String, Box<dyn std::error::Error>> {
    if !result.success {
        return Err(format!("dispatch failed: {result:?}").into());
    }
    let data: Value = serde_json::from_str(&result.data)?;
    Ok(data["systemReminder"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

/// `discover-foundation` in a fresh project, run as `agent`.
fn discover_as(agent: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let result = with_agent(agent, || {
        dispatch(tmp.path(), "discover-foundation", json!({}))
    });
    reminder(&result)
}

/// The draft `discover-foundation` writes, then the project.name field
/// filled by `update-foundation projectName` run as `agent`.
fn vision_reminder_as(agent: &str) -> Result<String, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let result = with_agent(Some(agent), || {
        let created = dispatch(tmp.path(), "discover-foundation", json!({}));
        if !created.success {
            return created;
        }
        dispatch(
            tmp.path(),
            "update-foundation",
            json!({ "section": "projectName", "content": "MyProject" }),
        )
    });
    reminder(&result)
}

#[test]
fn claude_code_agent_receives_ultrathink_guidance_in_system_reminder() -> TestResult {
    // @step Given I am using Claude Code (FSPEC_AGENT=claude)
    let agent = Some("claude");

    // @step And Claude Code has supportsMetaCognition flag set to true
    let tmp = tempfile::tempdir()?;
    assert!(with_agent(agent, || {
        codelet_fspec_core::foundation::guidance::agent_supports_meta_cognition(tmp.path())
    }));

    // @step When I run fspec discover-foundation
    let output = discover_as(agent)?;

    // @step Then I should see "you must ULTRATHINK the entire codebase" in the output
    assert!(
        output.contains(
            "Draft created. To complete foundation, you must ULTRATHINK the entire codebase."
        ),
        "{output}"
    );

    // @step And the output should contain system-reminder tags
    assert!(
        output.contains("<system-reminder>") && output.contains("</system-reminder>"),
        "{output}"
    );

    // @step And the guidance should emphasize deep codebase analysis
    assert!(
        output.contains(
            "Analyze EVERYTHING: code structure, entry points, user interactions, documentation."
        ),
        "{output}"
    );
    assert!(!output.contains("think a lot"), "{output}");
    Ok(())
}

/// The assertions shared by every agent without meta-cognition.
fn assert_think_a_lot(agent: Option<&str>) -> TestResult {
    let output = discover_as(agent)?;
    assert!(
        output.contains(
            "Draft created. To complete foundation, you must think a lot about the entire codebase."
        ),
        "{output}"
    );
    assert!(!output.contains("ULTRATHINK"), "{output}");
    Ok(())
}

#[test]
fn cursor_ide_agent_receives_think_a_lot_guidance() -> TestResult {
    // @step Given I am using Cursor (FSPEC_AGENT=cursor)
    let agent = Some("cursor");

    // @step And Cursor has supportsMetaCognition flag set to false
    let tmp = tempfile::tempdir()?;
    assert!(!with_agent(agent, || {
        codelet_fspec_core::foundation::guidance::agent_supports_meta_cognition(tmp.path())
    }));

    // @step When I run fspec discover-foundation
    // @step Then I should see "you must think a lot about the entire codebase" in the output
    // @step And I should NOT see "ULTRATHINK" in the output
    assert_think_a_lot(agent)
}

#[test]
fn aider_cli_agent_receives_think_a_lot_guidance() -> TestResult {
    // @step Given I am using Aider (FSPEC_AGENT=aider)
    let agent = Some("aider");

    // @step And Aider has supportsMetaCognition flag set to false
    let tmp = tempfile::tempdir()?;
    assert!(!with_agent(agent, || {
        codelet_fspec_core::foundation::guidance::agent_supports_meta_cognition(tmp.path())
    }));

    // @step When I run fspec discover-foundation
    // @step Then I should see "you must think a lot about the entire codebase" in the output
    // @step And I should NOT see "ULTRATHINK" in the output
    assert_think_a_lot(agent)
}

#[test]
fn unknown_agent_receives_think_a_lot_guidance() -> TestResult {
    // @step Given no agent is configured (no FSPEC_AGENT env var or config file)
    // (discover_as runs in a fresh project with no spec/fspec-config.json)

    // @step When I run fspec discover-foundation
    // @step Then I should see "you must think a lot about the entire codebase" in the output
    // @step And I should NOT see "ULTRATHINK" in the output
    assert_think_a_lot(None)
}

#[test]
fn project_vision_field_guidance_follows_the_agents_meta_cognition_support() -> TestResult {
    // @step Given a foundation draft whose next unfilled field after project.name is project.vision
    // (vision_reminder_as creates the draft with discover-foundation)

    // @step When I run fspec update-foundation projectName once with FSPEC_AGENT=claude and once with FSPEC_AGENT=cursor
    let claude = vision_reminder_as("claude")?;
    let cursor = vision_reminder_as("cursor")?;

    // @step Then the claude reminder for Field 2/8 should include "ULTRATHINK: Read ALL code, understand the system deeply."
    assert!(
        claude.contains("Field 2/8: project.vision (elevator pitch)"),
        "{claude}"
    );
    assert!(
        claude.contains("ULTRATHINK: Read ALL code, understand the system deeply."),
        "{claude}"
    );

    // @step And the cursor reminder for Field 2/8 should include "Think a lot about the entire codebase."
    assert!(
        cursor.contains("Field 2/8: project.vision (elevator pitch)"),
        "{cursor}"
    );
    assert!(
        cursor.contains("Think a lot about the entire codebase."),
        "{cursor}"
    );
    assert!(!cursor.contains("ULTRATHINK"), "{cursor}");
    Ok(())
}
