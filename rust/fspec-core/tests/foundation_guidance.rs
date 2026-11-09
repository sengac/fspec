#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/foundation-field-reminder-dedup.feature
//
// Integration tests for the shared foundation/guidance.rs module (the single
// source of truth for the 8-field draft table, full-field scan, progress
// renderer, and unified field reminder). Each scenario maps to exactly one
// #[test] with @step comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: the module does not exist yet, so these tests FAIL to compile /
// fail their assertions now. They assert the real expected behaviour the
// implementation must satisfy.

use std::path::Path;

use codelet_fspec_core::foundation::guidance;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

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

fn write_draft(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    std::fs::create_dir_all(&spec).expect("mkdir spec");
    std::fs::write(
        spec.join("foundation.json.draft"),
        serde_json::to_string_pretty(value).expect("ser draft"),
    )
    .expect("write draft");
}

// ---------- scenarios ----------

#[test]
fn scan_fields_reports_status_for_all_eight_fields_not_just_the_first() {
    // @step Given a project root whose spec/foundation.json.draft is the canonical 8-field placeholder draft
    let draft = placeholder_draft();

    // @step When I scan the draft fields with the shared guidance module
    let rows = guidance::scan_fields(&draft);

    // @step Then the scan returns exactly 8 rows, one per draft field
    assert_eq!(rows.len(), 8, "scan must cover all 8 fields: {rows:?}");

    // @step And every row reports an incomplete status (placeholder, empty-array, or placeholder-entries)
    for row in &rows {
        assert!(
            !row.is_complete(),
            "field {} must be incomplete in a fresh draft: {rows:?}",
            row.path
        );
    }

    // @step And the capabilities row is empty-array while the personas row is placeholder-entries
    let caps = rows
        .iter()
        .find(|r| r.path == "solutionSpace.capabilities")
        .expect("caps row");
    let personas = rows
        .iter()
        .find(|r| r.path == "personas")
        .expect("personas row");
    assert!(
        caps.is_empty_array(),
        "capabilities [] is empty-array: {caps:?}"
    );
    assert!(
        personas.has_placeholder_entries(),
        "placeholder personas are placeholder-entries: {personas:?}"
    );
}

#[test]
fn scan_fields_reports_complete_arrays_and_counts_progress() {
    // @step Given a draft where 3 scalar fields are filled, capabilities has 2 real entries, and personas has 1 real entry
    let mut draft = placeholder_draft();
    draft["project"]["name"] = json!("fspec");
    draft["project"]["vision"] = json!("Ship faster");
    draft["project"]["projectType"] = json!("cli-tool");
    draft["solutionSpace"]["capabilities"] = json!([
        { "name": "Spec Validation", "description": "d" },
        { "name": "Work Units", "description": "d" }
    ]);
    draft["personas"] = json!([{ "name": "Dev", "description": "d", "goals": ["g"] }]);

    // @step When I scan the draft fields with the shared guidance module
    let rows = guidance::scan_fields(&draft);

    // @step Then the progress is 5 of 8 complete (3 scalars + capabilities + personas)
    let complete = rows.iter().filter(|r| r.is_complete()).count();
    assert_eq!(complete, 5, "expected 5 complete; rows: {rows:?}");

    // @step And an array field with a remaining placeholder entry is NOT complete
    let mut draft2 = draft.clone();
    draft2["personas"] = json!([
        { "name": "Dev", "description": "d", "goals": ["g"] },
        { "name": "[QUESTION: Who else?]", "description": "d", "goals": [] }
    ]);
    let rows2 = guidance::scan_fields(&draft2);
    let personas2 = rows2
        .iter()
        .find(|r| r.path == "personas")
        .expect("personas row");
    assert!(
        !personas2.is_complete(),
        "mixed real+placeholder personas must be incomplete: {personas2:?}"
    );
}

#[test]
fn field_reminder_keeps_existing_substrings_and_appends_examples() {
    // @step Given the shared guidance module's field reminder builder
    // @step When I build the reminder for the project.vision field (position 2 of 8, non-meta agent, no detected value)
    let body = guidance::field_reminder_body("project.vision", 2, 8, false, None);

    // @step Then the reminder still contains the asserted header 'Field 2/8: project.vision (elevator pitch)'
    assert!(
        body.contains("Field 2/8: project.vision (elevator pitch)"),
        "header must survive dedup: {body}"
    );

    // @step And the reminder still contains 'Run: fspec update-foundation projectVision'
    assert!(
        body.contains("Run: fspec update-foundation projectVision"),
        "fix command line must survive dedup: {body}"
    );

    // @step And the reminder now contains an 'Examples:' section with at least one example
    assert!(
        body.contains("Examples:"),
        "reminders must carry examples: {body}"
    );

    // @step And the ULTRATHINK branch is still keyed off meta-cognition support
    let meta = guidance::field_reminder_body("project.vision", 2, 8, true, None);
    assert!(
        meta.contains("ULTRATHINK: Read ALL code, understand the system deeply."),
        "ULTRATHINK branch must survive dedup: {meta}"
    );
    assert!(
        !meta.contains("Think a lot about the entire codebase."),
        "non-ULTRATHINK line must not appear for meta agents: {meta}"
    );
}

#[test]
fn scan_uses_the_same_agent_meta_cognition_resolver_as_before() {
    // @step Given the shared module exposes the agent meta-cognition resolver
    // @step When I call it against a non-existent project root (no FSPEC_AGENT, no config)
    let root = Path::new("/nonexistent-disc003");
    let supports = guidance::agent_supports_meta_cognition(root);

    // @step Then it defaults to false (safe default, matching the pre-refactor behaviour)
    assert!(!supports, "default agent must not support meta-cognition");
}

#[test]
fn the_field_reminder_text_comes_from_the_shared_guidance_module_with_examples() {
    // @step Given a project root whose spec/foundation.json.draft has only project.name filled
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = placeholder_draft();
    draft["project"]["name"] = json!("fspec");
    write_draft(tmp.path(), &draft);

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let result = dispatch_command(DispatchRequest {
        command: "update-foundation".to_string(),
        args_json: json!({ "section": "projectName", "content": "fspec" }).to_string(),
        project_root: tmp.path().to_path_buf(),
    });

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let reminder = data["systemReminder"].as_str().unwrap_or_default();

    // @step And the result systemReminder contains 'Field 2/8: project.vision (elevator pitch)'
    assert!(
        reminder.contains("Field 2/8: project.vision (elevator pitch)"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Run: fspec update-foundation projectVision'
    assert!(
        reminder.contains("Run: fspec update-foundation projectVision"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains an 'Examples:' section
    assert!(
        reminder.contains("Examples:"),
        "reminder must carry examples: {reminder}"
    );

    // @step And update_foundation.rs and discover_foundation.rs no longer define their own scan or field-reminder functions
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "src/commands/update_foundation.rs",
        "src/commands/discover_foundation.rs",
    ] {
        let src = std::fs::read_to_string(workspace.join(rel))
            .unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for forbidden in [
            "fn scan_draft_for_next_field",
            "fn field_reminder_body",
            "fn extract_detected_value",
            "fn agent_supports_meta_cognition",
            "fn is_known_agent",
        ] {
            assert!(
                !src.contains(forbidden),
                "{rel} must not define `{forbidden}` after dedup — use foundation::guidance instead"
            );
        }
    }
    // The shared module must own the logic.
    let guidance_src_path = workspace.join("src/foundation/guidance.rs");
    assert!(
        guidance_src_path.exists(),
        "foundation/guidance.rs must exist: {}",
        guidance_src_path.display()
    );
}

// ---------- scenarios moved from DISC-001 and FOUND-010 (COV-072) ----------

/// A draft whose first `filled` fields (in the 8-field order) hold real
/// values and whose remaining fields still hold their placeholders.
fn draft_filled_up_to(filled: usize) -> Value {
    let mut draft = placeholder_draft();
    let real: [(&str, Value); 7] = [
        ("/project/name", json!("fspec")),
        (
            "/project/vision",
            json!("Specs that stay in sync with code"),
        ),
        ("/project/projectType", json!("cli-tool")),
        ("/problemSpace/primaryProblem/title", json!("Spec drift")),
        (
            "/problemSpace/primaryProblem/description",
            json!("Specs drift out of sync with code"),
        ),
        ("/solutionSpace/overview", json!("A CLI that manages specs")),
        (
            "/solutionSpace/capabilities",
            json!([{"name": "Spec Validation", "description": "Validates Gherkin"}]),
        ),
    ];
    for (pointer, value) in real.into_iter().take(filled) {
        *draft.pointer_mut(pointer).expect("draft field") = value;
    }
    draft
}

/// Dispatch update-foundation projectName 'fspec' against `draft`; the
/// systemReminder naming the next unfilled field.
fn next_field_reminder_after_update(draft: &Value) -> String {
    let tmp = TempDir::new().expect("tempdir");
    write_draft(tmp.path(), draft);
    let result = dispatch_command(DispatchRequest {
        command: "update-foundation".to_string(),
        args_json: json!({ "section": "projectName", "content": "fspec" }).to_string(),
        project_root: tmp.path().to_path_buf(),
    });
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    data["systemReminder"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn the_project_type_reminder_asks_the_ai_to_determine_the_type_and_verify_it_with_the_human() {
    // @step Given a project root whose spec/foundation.json.draft has project.name and project.vision filled and project.projectType "[QUESTION: What type of project is this?]"
    let mut draft = draft_filled_up_to(2);
    draft["project"]["projectType"] = json!("[QUESTION: What type of project is this?]");

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let reminder = next_field_reminder_after_update(&draft);

    // @step Then the result systemReminder contains 'Field 3/8: project.projectType'
    assert!(
        reminder.contains("Field 3/8: project.projectType\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Analyze codebase to determine project type. Verify with human.'
    assert!(
        reminder.contains("\nAnalyze codebase to determine project type. Verify with human.\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder lists the example project types cli-tool, web-app and library
    assert!(
        reminder.contains(
            "Examples (non-exhaustive, any short descriptor is valid): cli-tool, web-app, library,"
        ),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Run: fspec update-foundation projectType "<type>"'
    assert!(
        reminder.contains("Run: fspec update-foundation projectType \"<type>\""),
        "reminder: {reminder}"
    );
}

#[test]
fn a_detected_project_type_is_shown_in_its_reminder_for_the_human_to_verify() {
    // @step Given a project root whose spec/foundation.json.draft has project.name and project.vision filled and project.projectType "[DETECTED: web-app]"
    let mut draft = draft_filled_up_to(2);
    draft["project"]["projectType"] = json!("[DETECTED: web-app]");

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let reminder = next_field_reminder_after_update(&draft);

    // @step Then the result systemReminder contains 'Field 3/8: project.projectType'
    assert!(
        reminder.contains("Field 3/8: project.projectType\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains '[DETECTED: web-app] Analyze codebase to determine project type. Verify with human.'
    assert!(
        reminder.contains(
            "[DETECTED: web-app] Analyze codebase to determine project type. Verify with human."
        ),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Run: fspec update-foundation projectType "<type>"'
    assert!(
        reminder.contains("Run: fspec update-foundation projectType \"<type>\""),
        "reminder: {reminder}"
    );
}

#[test]
fn the_primary_problem_reminder_asks_for_the_user_perspective() {
    // @step Given a project root whose spec/foundation.json.draft has the project fields filled and problemSpace.primaryProblem.title "[QUESTION: What problem does this solve?]"
    let draft = draft_filled_up_to(3);
    assert_eq!(
        draft["problemSpace"]["primaryProblem"]["title"],
        json!("[QUESTION: What problem does this solve?]")
    );

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let reminder = next_field_reminder_after_update(&draft);

    // @step Then the result systemReminder contains 'Field 4/8: problemSpace.primaryProblem.title'
    assert!(
        reminder.contains("Field 4/8: problemSpace.primaryProblem.title\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'CRITICAL: Think from USER perspective. WHO uses this (persona)?'
    assert!(
        reminder.contains("CRITICAL: Think from USER perspective. WHO uses this (persona)?"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'WHAT problem do THEY face? WHY do they need this solution?'
    assert!(
        reminder.contains("WHAT problem do THEY face? WHY do they need this solution?"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Requires title, description, impact.'
    assert!(
        reminder.contains("Requires title, description, impact."),
        "reminder: {reminder}"
    );
}

#[test]
fn the_capabilities_reminder_asks_for_what_users_can_do_not_how() {
    // @step Given a project root whose spec/foundation.json.draft has every field before solutionSpace.capabilities filled and a capability "[QUESTION: What can users DO?]"
    // (an empty capabilities array is skipped by the chain, like the
    // TypeScript scan: only placeholder entries make it the next field.)
    let mut draft = draft_filled_up_to(6);
    draft["solutionSpace"]["capabilities"] = json!([{
        "name": "[QUESTION: What can users DO?]",
        "description": "[QUESTION: What can users DO?]"
    }]);

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let reminder = next_field_reminder_after_update(&draft);

    // @step Then the result systemReminder contains 'Field 7/8: solutionSpace.capabilities'
    assert!(
        reminder.contains("Field 7/8: solutionSpace.capabilities\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'List 3-7 high-level abilities users have. Focus on WHAT not HOW.'
    assert!(
        reminder.contains("List 3-7 high-level abilities users have. Focus on WHAT not HOW."),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Example: "Spec Validation" (WHAT), NOT "Uses Cucumber parser" (HOW)'
    assert!(
        reminder
            .contains("Example: \"Spec Validation\" (WHAT), NOT \"Uses Cucumber parser\" (HOW)"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Run: fspec add-capability "Capability Name" "Capability Description"'
    assert!(
        reminder
            .contains("Run: fspec add-capability \"Capability Name\" \"Capability Description\""),
        "reminder: {reminder}"
    );
}

#[test]
fn the_personas_reminder_asks_for_every_user_type_with_goals_and_pain_points() {
    // @step Given a project root whose spec/foundation.json.draft has every field but personas filled and a persona "[QUESTION: Who uses this?]"
    let draft = draft_filled_up_to(7);
    assert_eq!(
        draft["personas"][0]["name"],
        json!("[QUESTION: Who uses this?]")
    );

    // @step When I dispatch update-foundation with section='projectName' and content='fspec'
    let reminder = next_field_reminder_after_update(&draft);

    // @step Then the result systemReminder contains 'Field 8/8: personas'
    assert!(
        reminder.contains("Field 8/8: personas\n"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Identify ALL user types from interactions.'
    assert!(
        reminder.contains("Identify ALL user types from interactions."),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'CLI tools: who runs commands? Web apps: who uses UI + who calls API?'
    assert!(
        reminder.contains("CLI tools: who runs commands? Web apps: who uses UI + who calls API?"),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Ask human about goals and pain points.'
    assert!(
        reminder.contains("Ask human about goals and pain points."),
        "reminder: {reminder}"
    );

    // @step And the result systemReminder contains 'Run: fspec add-persona "Persona Name" "Persona Description" --goal "Primary goal"'
    assert!(
        reminder.contains(
            "Run: fspec add-persona \"Persona Name\" \"Persona Description\" --goal \"Primary goal\""
        ),
        "reminder: {reminder}"
    );
}
