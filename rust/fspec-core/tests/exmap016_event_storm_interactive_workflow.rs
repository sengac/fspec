//! EXMAP-016 — the Event Storm workflow is offered when a work unit enters
//! specifying, `discover-event-storm` emits its guidance, and the bootstrap
//! documentation carries the Feature Event Storm section between the
//! foundation steps and Example Mapping.
//!
//! Feature: spec/features/complete-event-storm-interactive-workflow-with-example-mapping-integration.feature
//!
//! Code: update_work_unit_status/reminders/status.rs (EVENT_STORM, wired in
//! update_work_unit_status/output.rs), discover_event_storm/guidance.rs and
//! commands/bootstrap_doc.txt.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A board whose only work unit, the story `id`, is in `status`.
fn board(dir: &Path, id: &str, status: &str) -> TestResult {
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
        let ids: Vec<&str> = if state == status { vec![id] } else { vec![] };
        states.insert(state.to_string(), json!(ids));
    }
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {id: {
                "id": id, "title": "Checkout", "type": "story", "status": status,
                "createdAt": "2026-11-01T00:00:00.000Z",
                "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": states,
        }))?,
    )?;
    Ok(())
}

/// Dispatch `command` with `args`; its data on success.
fn dispatch(dir: &Path, command: &str, args: Value) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("{command} failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The `<system-reminder>` block discover-event-storm printed after its
/// confirmation line.
fn guidance_block(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let start = output
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    Ok(&output[start..])
}

fn assert_contains(text: &str, needle: &str) {
    assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
}

#[test]
fn system_reminder_prompts_ai_to_assess_domain_complexity_when_moving_to_specifying() -> TestResult
{
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit in backlog status
    board(dir, "WORK-001", "backlog")?;

    // @step When I run "fspec update-work-unit-status WORK-001 specifying"
    let output = dispatch(
        dir,
        "update-work-unit-status",
        json!({"workUnitId": "WORK-001", "status": "specifying"}),
    )?;

    // @step Then a system-reminder should appear with Event Storm assessment questions
    let start = output
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let reminder = &output[start..];
    assert_contains(reminder, "EVENT STORM ASSESSMENT - Domain Complexity Check");

    // @step And the reminder should ask "Do you understand the core domain events?"
    assert_contains(reminder, "1. Do you understand the core domain events?");

    // @step And the reminder should ask "Are commands and policies clear?"
    assert_contains(reminder, "2. Are commands and policies clear?");

    // @step And the reminder should ask "Is there significant domain complexity?"
    assert_contains(reminder, "3. Is there significant domain complexity?");

    // @step And the reminder should present choice: Run Event Storm FIRST or skip to Example Mapping
    assert_contains(
        reminder,
        "Option 1: RUN EVENT STORM FIRST (if domain is complex)",
    );
    assert_contains(reminder, "→ Run: fspec discover-event-storm WORK-001");
    assert_contains(
        reminder,
        "Option 2: SKIP TO EXAMPLE MAPPING (if domain is simple)",
    );

    // @step And the reminder should include concrete examples of when Event Storm helped
    assert_contains(reminder, "EXAMPLES when Event Storm helped:");
    assert_contains(reminder, "• Payment Processing:");
    assert_contains(reminder, "EXAMPLES when Event Storm was overkill:");

    // @step And the reminder should tell the AI to STOP and assess domain complexity before jumping to Example Mapping
    assert_contains(
        reminder,
        "BEFORE jumping to Example Mapping, STOP and assess domain complexity.",
    );
    assert_contains(reminder, "CONSCIOUS CHOICE:");
    Ok(())
}

#[test]
fn ai_runs_discover_event_storm_and_receives_comprehensive_guidance() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit "AUTH-001" in specifying status
    board(dir, "AUTH-001", "specifying")?;

    // @step When I run "fspec discover-event-storm AUTH-001"
    let output = dispatch(
        dir,
        "discover-event-storm",
        json!({"workUnitId": "AUTH-001"}),
    )?;

    // @step Then comprehensive guidance should be emitted as system-reminder
    assert!(
        output.starts_with(
            "✓ Event Storm discovery session started for AUTH-001\n<system-reminder>\n"
        ),
        "the guidance is not a system reminder:\n{output}"
    );
    let guidance = guidance_block(&output)?;
    assert!(
        guidance.trim_end().ends_with("</system-reminder>"),
        "{guidance}"
    );
    assert_contains(
        guidance,
        "## Step 4: Feature Event Storm (BEFORE Example Mapping)",
    );

    // @step And guidance should show conversation pattern with examples
    assert_contains(guidance, "**Ask the human**:");
    assert_contains(
        guidance,
        "fspec add-domain-event AUTH-001 \"UserRegistered\"",
    );
    assert_contains(guidance, "### Examples: When Event Storm Helped");

    // @step And guidance should explain flow: Events → Commands → Policies → Hotspots
    assert_contains(guidance, "Capture: Events → Commands → Policies → Hotspots");

    // @step And guidance should list available commands (add-domain-event, add-command, add-policy, add-hotspot)
    for command in [
        "fspec add-domain-event <workUnitId> <text>",
        "fspec add-command <workUnitId> <text>",
        "fspec add-policy <workUnitId> <text> --when <event> --then <command>",
        "fspec add-hotspot <workUnitId> <text> --concern <description>",
    ] {
        assert_contains(guidance, command);
    }

    // @step And guidance should show when to stop criteria
    assert_contains(guidance, "### When to Stop Event Storm");
    assert_contains(guidance, "1. ✅ You've captured all major domain events");

    // @step And guidance should explain generate-example-mapping-from-event-storm for transformation
    assert_contains(guidance, "### Transform Event Storm → Example Mapping");
    assert_contains(
        guidance,
        "fspec generate-example-mapping-from-event-storm <workUnitId>",
    );
    Ok(())
}

#[test]
fn event_storm_section_is_integrated_into_bootstrap() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given the fspec bootstrap documentation
    // (an empty project: bootstrap needs no spec files.)

    // @step When AI runs "fspec bootstrap"
    let output = dispatch(dir, "bootstrap", json!({}))?;

    // @step Then bootstrap output should include Event Storm guidance
    let event_storm = output
        .find("## Step 4: Feature Event Storm (BEFORE Example Mapping)")
        .ok_or("the Feature Event Storm section is missing")?;

    // @step And the Feature Event Storm section should appear after "## Step 2: Bootstrap Foundation" and before "## Step 5: Example Mapping"
    let foundation = output
        .find("## Step 2: Bootstrap Foundation")
        .ok_or("the Bootstrap Foundation section is missing")?;
    let example_mapping = output
        .find("## Step 5: Example Mapping")
        .ok_or("the Example Mapping section is missing")?;
    assert!(
        foundation < event_storm && event_storm < example_mapping,
        "sections out of order: foundation {foundation}, event storm {event_storm}, example mapping {example_mapping}"
    );
    Ok(())
}

#[test]
fn bootstrap_guidance_displays_event_storm_workflow() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given Event Storm section is integrated
    // @step When AI runs "fspec bootstrap"
    let output = dispatch(dir, "bootstrap", json!({}))?;
    let start = output
        .find("## Step 4: Feature Event Storm (BEFORE Example Mapping)")
        .ok_or("the Feature Event Storm section is missing")?;
    let end = output
        .find("## Step 5: Example Mapping")
        .ok_or("the Example Mapping section is missing")?;
    let section = &output[start..end];

    // @step Then AI should see Event Storm section in output
    assert_contains(section, "**WHEN TO USE FEATURE EVENT STORM**");

    // @step And section should explain when to use Event Storm vs skip to Example Mapping
    assert_contains(
        section,
        "### Decision: Feature Event Storm vs Skip to Example Mapping",
    );
    assert_contains(section, "**RUN FEATURE EVENT STORM FIRST** if:");
    assert_contains(section, "**SKIP TO EXAMPLE MAPPING** if:");

    // @step And section should include concrete examples (complex domain vs simple bug)
    assert_contains(
        section,
        "**Example 1: Payment Processing (Complex Domain)**",
    );
    assert_contains(section, "**Example 2: User Login Bug (Simple)**");

    // @step And section should show command: fspec discover-event-storm <workUnitId>
    assert_contains(section, "fspec discover-event-storm <workUnitId>");

    // @step And section should explain transformation to Example Mapping
    assert_contains(section, "### Transform Event Storm → Example Mapping");
    assert_contains(
        section,
        "fspec generate-example-mapping-from-event-storm <workUnitId>",
    );
    Ok(())
}
