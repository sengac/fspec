#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/foundation-discovery-agent-guidance.feature
//
// Tests the universal `nextSteps` status trailer appended to the success
// envelope of foundation-domain mutation commands. Each scenario maps to
// exactly one #[test] with @step comments mirroring the Gherkin steps
// verbatim.
//
// RED PHASE: no command emits `nextSteps` yet, so these tests FAIL now.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(command: &str, project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), body).expect("write foundation.json");
}

fn write_draft_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), body).expect("write draft");
}

fn result_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).unwrap_or(Value::Null)
}

/// Draft with 5/8 fields filled: name, vision, projectType, problemTitle,
/// and a real capabilities entry. problemDefinition, solutionOverview, and
/// personas left placeholder. Used by the add-capability trailer scenario.
fn draft_five_of_eight() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "fspec", "vision": "Ship faster", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": {
                "title": "Specs drift out of sync with code",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": {
            "overview": "[QUESTION: What can users DO?]",
            "capabilities": [ { "name": "Spec Validation", "description": "Validates" } ]
        },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

/// Draft with 6/8 fields filled: name, vision, projectType, problemTitle,
/// problemDefinition, and a real capabilities entry. solutionOverview and
/// personas left placeholder. Used by the add-persona trailer scenario.
fn draft_six_of_eight() -> Value {
    let mut d = draft_five_of_eight();
    d["problemSpace"]["primaryProblem"]["description"] = json!("Specs rot");
    d
}

/// Finalized foundation with an event storm item already present.
fn final_with_auth_context() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "fspec", "vision": "v", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
        "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] },
        "personas": [{ "name": "Dev", "description": "d", "goals": ["g"] }],
        "eventStorm": {
            "level": "big_picture",
            "items": [
                { "id": 1, "type": "bounded_context", "text": "Auth", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z" }
            ],
            "nextItemId": 2
        }
    })
}

// ---------- scenarios: draft-phase trailers ----------

#[test]
fn add_capability_on_the_draft_appends_a_progress_trailer() {
    // @step Given a project root whose spec/foundation.json.draft has 5 of 8 fields filled including a non-empty capabilities array
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&draft_five_of_eight()).unwrap(),
    );

    // @step When I dispatch add-capability with name='Spec Validation' and description='Validate Gherkin features'
    let result = dispatch_command(req(
        "add-capability",
        tmp.path(),
        json!({ "name": "Work Units", "description": "Manage work units" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'progress:' reporting fields complete and 'remaining:'
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("progress: 5/8 fields complete"),
        "trailer must report 5/8 progress: {next_steps:?}"
    );
    assert!(
        next_steps.contains("remaining:"),
        "trailer must list remaining aliases: {next_steps:?}"
    );

    // @step And the result nextSteps contains a line starting 'next:' with the next field fix command
    assert!(
        next_steps.contains("next: fspec update-foundation problemDefinition"),
        "trailer must point at the next unfilled field: {next_steps:?}"
    );
}

#[test]
fn add_persona_on_the_draft_appends_a_progress_trailer() {
    // @step Given a project root whose spec/foundation.json.draft has 6 of 8 fields filled and real capabilities
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&draft_six_of_eight()).unwrap(),
    );

    // @step When I dispatch add-persona with name='Developer' and description='Builds features' and goals=['Ship quality code']
    let result = dispatch_command(req(
        "add-persona",
        tmp.path(),
        json!({
            "name": "Developer",
            "description": "Builds features",
            "goals": ["Ship quality code"]
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'progress:' with 'remaining:'
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("progress: 7/8 fields complete"),
        "trailer must report 7/8 after the only remaining field (personas) is filled: {next_steps:?}"
    );
    assert!(
        next_steps.contains("remaining: solutionOverview"),
        "solutionOverview is still unfilled after add-persona: {next_steps:?}"
    );

    // @step And the result nextSteps contains a line starting 'next:'
    assert!(
        next_steps.contains("next: fspec update-foundation solutionOverview"),
        "trailer must point at the next unfilled field: {next_steps:?}"
    );
}

#[test]
fn remove_capability_on_the_draft_appends_a_progress_trailer() {
    // @step Given a project root whose spec/foundation.json.draft has capabilities containing 'Spec Validation' and 5 of 8 fields filled
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&draft_five_of_eight()).unwrap(),
    );

    // @step When I dispatch remove-capability with name='Spec Validation'
    let result = dispatch_command(req(
        "remove-capability",
        tmp.path(),
        json!({ "name": "Spec Validation" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'progress:'
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.starts_with("progress:") || next_steps.contains("progress: "),
        "trailer must start with the progress line: {next_steps:?}"
    );
    assert!(
        next_steps.contains("progress: 4/8 fields complete"),
        "removing the only capability drops capabilities to empty-array: {next_steps:?}"
    );
}

#[test]
fn update_foundation_on_the_draft_keeps_the_field_reminder_and_adds_a_progress_trailer() {
    // @step Given a project root whose spec/foundation.json.draft has only project.name filled
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": { "primaryProblem": { "title": "[QUESTION: What problem does this solve?]", "description": "[QUESTION: What problem does this solve?]", "impact": "high" } },
        "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
        "personas": [{ "name": "[QUESTION: Who uses this?]", "description": "[QUESTION: Who uses this?]", "goals": ["[QUESTION: What are their goals?]"] }]
    });
    draft["project"]["name"] = json!("fspec");
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let result = dispatch_command(req(
        "update-foundation",
        tmp.path(),
        json!({ "section": "projectName", "content": "fspec" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = result_data(&result);

    // @step And the result systemReminder still contains 'Field 2/8: project.vision'
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("Field 2/8: project.vision"),
        "chained reminder must survive the refactor: {reminder}"
    );

    // @step And the result nextSteps contains a line starting 'progress:'
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("progress: 1/8 fields complete"),
        "trailer must report progress after the update: {next_steps:?}"
    );
}

#[test]
fn no_trailer_is_emitted_when_a_mutation_fails() {
    // @step Given an empty project root with no foundation files
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch add-capability with name='X' and description='Y'
    let result = dispatch_command(req(
        "add-capability",
        tmp.path(),
        json!({ "name": "X", "description": "Y" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'foundation.json not found'
    let err = result.error.clone().unwrap_or_default();
    assert!(
        err.contains("foundation.json not found"),
        "must surface canonical error; got: {err}"
    );
}

// ---------- scenarios: event-storm phase trailers ----------

#[test]
fn add_foundation_bounded_context_appends_an_event_storm_trailer() {
    // @step Given a project root with a finalized spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&json!({
            "version": "2.0.0",
            "project": { "name": "fspec", "vision": "v", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
        }))
        .unwrap(),
    );

    // @step When I dispatch add-foundation-bounded-context with text='Auth'
    let result = dispatch_command(req(
        "add-foundation-bounded-context",
        tmp.path(),
        json!({ "text": "Auth" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'eventStorm:' with context, aggregate, event, and command counts
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("eventStorm: 1 contexts, 0 aggregates, 0 events, 0 commands"),
        "event-storm summary must report counts: {next_steps:?}"
    );

    // @step And the result nextSteps contains a line starting 'next:' suggesting an aggregate for 'Auth'
    assert!(
        next_steps.contains("next: fspec add-aggregate-to-foundation \"Auth\"")
            || next_steps.contains("next: add-aggregate-to-foundation \"Auth\""),
        "next action must suggest the next item type for the context: {next_steps:?}"
    );
}

#[test]
fn add_aggregate_to_foundation_appends_an_event_storm_trailer() {
    // @step Given a project root with a finalized spec/foundation.json whose event storm contains bounded context 'Auth'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&final_with_auth_context()).unwrap(),
    );

    // @step When I dispatch add-aggregate-to-foundation with contextName='Auth' and aggregateName='Session'
    let result = dispatch_command(req(
        "add-aggregate-to-foundation",
        tmp.path(),
        json!({ "contextName": "Auth", "aggregateName": "Session" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'eventStorm:' reporting 1 context and 1 aggregate
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("eventStorm: 1 contexts, 1 aggregates, 0 events, 0 commands"),
        "must count the new aggregate: {next_steps:?}"
    );

    // @step And the result nextSteps contains a line starting 'next:' suggesting a domain event for 'Auth'
    assert!(
        next_steps.contains("add-domain-event-to-foundation \"Auth\""),
        "next action must suggest a domain event for the context: {next_steps:?}"
    );
}

#[test]
fn remove_foundation_bounded_context_appends_an_event_storm_trailer() {
    // @step Given a project root with a finalized spec/foundation.json whose event storm contains bounded contexts 'Auth' and 'Billing'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = final_with_auth_context();
    f["eventStorm"]["items"] = json!([
        { "id": 1, "type": "bounded_context", "text": "Auth", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z" },
        { "id": 2, "type": "bounded_context", "text": "Billing", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z" }
    ]);
    write_foundation_raw(tmp.path(), &serde_json::to_string_pretty(&f).unwrap());

    // @step When I dispatch remove-foundation-bounded-context with contextName='Billing'
    let result = dispatch_command(req(
        "remove-foundation-bounded-context",
        tmp.path(),
        json!({ "contextName": "Billing" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the result nextSteps contains a line starting 'eventStorm:'
    let data = result_data(&result);
    let next_steps = data["nextSteps"].as_str().unwrap_or_default();
    assert!(
        next_steps.contains("eventStorm: 1 contexts, 0 aggregates, 0 events, 0 commands"),
        "removal must be reflected in the counts: {next_steps:?}"
    );
}
