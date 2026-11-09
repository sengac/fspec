#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/discover-foundation-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `discover-foundation`
// (RPC-226). Each scenario maps to exactly one #[test] with @step comments
// mirroring the Gherkin steps verbatim.
//
// RED PHASE: the command is still a stub returning NotYetPorted, so these
// tests FAIL now. They assert the real expected behaviour the Phase C
// implementation must satisfy.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "discover-foundation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_draft_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), body).expect("write draft");
}

fn write_foundation_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), body).expect("write foundation.json");
}

/// A fully-filled, schema-valid draft (no [QUESTION:]/[DETECTED:] markers).
fn valid_filled_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "Acme", "vision": "Ship faster", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": { "title": "Pain", "description": "Real pain", "impact": "high" }
        },
        "solutionSpace": {
            "overview": "A CLI",
            "capabilities": [ { "name": "Cap", "description": "Does things" } ]
        },
        "personas": [ { "name": "Dev", "description": "Builds", "goals": ["Ship"] } ]
    })
}

/// The canonical draft template `discover-foundation` writes on creation.
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

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).expect("read file")
}

// ---------- scenarios ----------

#[test]
fn fresh_discovery_creates_draft_with_placeholders_and_first_field_reminder() {
    // @step Given an empty project root tempdir with no spec/foundation.json.draft and no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And spec/foundation.json.draft exists on disk
    let draft_path = tmp.path().join("spec/foundation.json.draft");
    assert!(draft_path.exists(), "draft must be created");

    // @step And the draft on disk contains the version "2.0.0"
    let draft = read_to_string(&draft_path);
    assert!(
        draft.contains("\"2.0.0\""),
        "draft must contain version: {draft}"
    );

    // @step And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    assert!(
        draft.contains("[QUESTION: What is the project name?]"),
        "missing name placeholder"
    );

    // @step And the draft on disk contains the placeholder "[DETECTED: cli-tool]"
    assert!(
        draft.contains("[DETECTED: cli-tool]"),
        "missing detected placeholder"
    );

    // @step And the returned systemReminder contains "Field 1/8: project.name"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("Field 1/8: project.name"),
        "reminder: {reminder}"
    );
}

#[test]
fn rerun_without_force_when_draft_exists_is_blocked() {
    // @step Given a project root tempdir that already has a spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&placeholder_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let pre = read_to_string(&tmp.path().join("spec/foundation.json.draft"));

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned systemReminder contains "ERROR: foundation.json.draft already exists!"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("ERROR: foundation.json.draft already exists!"),
        "reminder: {reminder}"
    );

    // @step And spec/foundation.json.draft on disk is byte-equal to its pre-call contents
    let post = read_to_string(&tmp.path().join("spec/foundation.json.draft"));
    assert_eq!(pre, post, "draft must be unchanged");

    // FOUND-044 (merged here): the three next steps, and no draft content.
    for expected in [
        // @step And the returned systemReminder contains "Choose ONE of these three next steps:"
        "Choose ONE of these three next steps:",
        // @step And the returned systemReminder contains "→ fspec discover-foundation --finalize"
        "→ fspec discover-foundation --finalize",
        // @step And the returned systemReminder contains "fspec show-foundation, which now auto-shows the draft"
        "fspec show-foundation, which now auto-shows the draft",
        // @step And the returned systemReminder contains "→ fspec discover-foundation --force"
        "→ fspec discover-foundation --force",
    ] {
        assert!(
            reminder.contains(expected),
            "missing {expected:?}: {reminder}"
        );
    }
    // @step And the returned systemReminder does not contain the draft content "[QUESTION: What is the project name?]"
    assert!(
        !result
            .data
            .contains("[QUESTION: What is the project name?]"),
        "the draft content must not be returned inline: {}",
        result.data
    );
}

#[test]
fn running_without_force_when_foundation_exists_is_blocked() {
    // @step Given a project root tempdir that has a spec/foundation.json but no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_foundation_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned systemReminder contains "ERROR: foundation.json already exists!"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("ERROR: foundation.json already exists!"),
        "reminder: {reminder}"
    );

    // @step And no spec/foundation.json.draft is created
    assert!(
        !tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must NOT be created on the foundation-exists block"
    );
}

#[test]
fn force_overwrite_regenerates_draft_from_scratch_with_warning() {
    // @step Given a project root tempdir that already has a spec/foundation.json.draft with custom content
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        "{\"version\":\"2.0.0\",\"project\":{\"name\":\"custom\"}}",
    );

    // @step When I dispatch discover-foundation with force=true
    let result = dispatch_command(req(tmp.path(), json!({"force": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    let draft = read_to_string(&tmp.path().join("spec/foundation.json.draft"));
    assert!(
        draft.contains("[QUESTION: What is the project name?]"),
        "draft regenerated: {draft}"
    );

    // @step And the returned systemReminder contains a force-overwrite warning
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("overwritten with --force"),
        "reminder: {reminder}"
    );

    // @step And forceOverwriteWarning is true because a draft was actually overwritten
    assert_eq!(
        data["forceOverwriteWarning"].as_bool(),
        Some(true),
        "force over an existing draft must set the stderr warning flag: {data}"
    );
}

#[test]
fn force_without_existing_draft_still_shows_stdout_banner_but_no_stderr_warning() {
    // PARITY (discover-foundation.ts:669-679 + 735-740): the STDOUT banner is
    // gated on `options.force` ALONE (so it shows even with no prior draft),
    // while the STDERR `output.warn` only fires when a draft was actually
    // overwritten. Here no draft exists, so the banner shows but the stderr
    // warning flag is false.

    // @step Given an empty project root tempdir with no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch discover-foundation with force=true
    let result = dispatch_command(req(tmp.path(), json!({"force": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the returned systemReminder still contains the force-overwrite banner
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("WARNING: Existing draft was overwritten with --force flag"),
        "banner must show on --force even without a prior draft: {reminder}"
    );

    // @step And forceOverwriteWarning is false because no draft was actually overwritten
    assert_eq!(
        data["forceOverwriteWarning"].as_bool(),
        Some(false),
        "no prior draft → no stderr warning: {data}"
    );
}

#[test]
fn finalize_blocked_when_draft_still_has_placeholder_fields() {
    // @step Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&placeholder_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned validationErrors contains "Cannot finalize: draft still has unfilled placeholder fields"
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.contains("Cannot finalize: draft still has unfilled placeholder fields"),
        "validationErrors: {errors}"
    );

    // @step And spec/foundation.json is not created
    assert!(
        !tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must not be written"
    );

    // @step And spec/foundation.json.draft still exists on disk
    assert!(
        tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must remain"
    );
}

#[test]
fn finalize_blocked_when_filled_draft_violates_schema() {
    // @step Given a project root tempdir whose spec/foundation.json.draft has no placeholders but empty solutionSpace.capabilities
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = valid_filled_draft();
    draft["solutionSpace"]["capabilities"] = json!([]);
    let body = serde_json::to_string_pretty(&draft).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned validationErrors starts with "Schema validation failed."
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.starts_with("Schema validation failed."),
        "validationErrors: {errors}"
    );

    // @step And spec/foundation.json is not created
    assert!(
        !tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must not be written"
    );

    // @step And the returned validationErrors contains "Missing required: solutionSpace.capabilities (at least one item required)"
    assert!(
        errors
            .contains("Missing required: solutionSpace.capabilities (at least one item required)"),
        "validationErrors: {errors}"
    );

    // @step And the returned validationErrors contains the fix command 'fspec add-capability "<name>" "<description>"'
    assert!(
        errors.contains("- For capabilities: fspec add-capability \"<name>\" \"<description>\""),
        "validationErrors: {errors}"
    );
}

#[test]
fn finalize_success_writes_foundation_deletes_draft_and_creates_found_unit() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true and autoGenerateMd=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"finalize": true, "autoGenerateMd": true}),
    ));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the returned completionMessage contains "Discovery complete!"
    let msg = data["completionMessage"].as_str().unwrap_or_default();
    assert!(
        msg.contains("Discovery complete!"),
        "completionMessage: {msg}"
    );

    // @step And spec/foundation.json exists on disk
    assert!(
        tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must be written"
    );

    // @step And spec/foundation.json.draft no longer exists on disk
    assert!(
        !tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must be deleted after finalize"
    );

    // @step And spec/FOUNDATION.md exists on disk
    assert!(
        tmp.path().join("spec/FOUNDATION.md").exists(),
        "FOUNDATION.md must be generated"
    );

    // @step And spec/work-units.json contains a FOUND-prefixed work unit
    let wu = read_to_string(&tmp.path().join("spec/work-units.json"));
    assert!(
        wu.contains("FOUND-"),
        "work-units.json must contain a FOUND- unit: {wu}"
    );

    // @step And the envelope reports workUnitCreated=true with workUnitId "FOUND-001"
    assert_eq!(
        data["workUnitCreated"].as_bool(),
        Some(true),
        "workUnitCreated: {data}"
    );
    assert_eq!(
        data["workUnitId"].as_str(),
        Some("FOUND-001"),
        "workUnitId: {data}"
    );

    // @step And the FOUND task carries a stateHistory array, no children array, and the file has no prefixCounters key
    let wu_json: Value = serde_json::from_str(&wu).expect("parse work-units.json");
    let task = &wu_json["workUnits"]["FOUND-001"];
    assert_eq!(
        task["type"].as_str(),
        Some("task"),
        "FOUND task type: {task}"
    );
    assert!(
        task.get("stateHistory").and_then(Value::as_array).is_some(),
        "FOUND task must carry stateHistory: {task}"
    );
    assert_eq!(
        task["stateHistory"][0]["state"].as_str(),
        Some("backlog"),
        "stateHistory[0].state: {task}"
    );
    assert!(
        task.get("children").is_none(),
        "FOUND task must NOT have a children array: {task}"
    );
    assert!(
        wu_json.get("prefixCounters").is_none(),
        "work-units.json must NOT carry prefixCounters (TS createWorkUnit never writes it): {wu_json}"
    );
}

#[test]
fn finalize_found_autocreation_is_idempotent_when_found_unit_already_exists() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and a FOUND-001 work unit already exists
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let seed = json!({
        "workUnits": {
            "FOUND-001": {
                "id": "FOUND-001",
                "title": "Existing Foundation Event Storm",
                "type": "task",
                "status": "backlog",
                "createdAt": "x",
                "updatedAt": "x"
            }
        },
        "states": { "backlog": ["FOUND-001"], "specifying": [], "testing": [],
                    "implementing": [], "validating": [], "done": [], "blocked": [] },
        "prefixCounters": { "FOUND": 1 }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&seed).unwrap(),
    )
    .expect("write work-units.json");

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And spec/work-units.json still has exactly one FOUND-prefixed work unit
    let wu: Value = serde_json::from_str(&read_to_string(&spec.join("work-units.json"))).unwrap();
    let found_count = wu["workUnits"]
        .as_object()
        .map(|m| m.keys().filter(|k| k.starts_with("FOUND-")).count())
        .unwrap_or(0);
    assert_eq!(
        found_count, 1,
        "expected exactly one FOUND- unit; got {found_count}"
    );

    // @step And the envelope reports workUnitCreated=false reusing workUnitId "FOUND-001"
    assert_eq!(
        data["workUnitCreated"].as_bool(),
        Some(false),
        "workUnitCreated: {data}"
    );
    assert_eq!(
        data["workUnitId"].as_str(),
        Some("FOUND-001"),
        "workUnitId: {data}"
    );
}

// ---------- scenarios moved from the BUG-016, BUG-078, BUG-084, DISC-002 and FOUND-013 features (COV-072) ----------

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_to_string(path)).expect("parse json")
}

/// The FOUND- ids on the board at `root` (none when there is no board).
fn found_ids(root: &Path) -> Vec<String> {
    let path = root.join("spec/work-units.json");
    if !path.exists() {
        return Vec::new();
    }
    read_json(&path)["workUnits"]
        .as_object()
        .map(|units| {
            units
                .keys()
                .filter(|id| id.starts_with("FOUND-"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// A valid draft whose personas hold a real persona and the template
/// persona with its placeholders.
fn draft_with_template_persona() -> Value {
    let mut draft = valid_filled_draft();
    draft["personas"] = json!([
        { "name": "Dev", "description": "Builds", "goals": ["Ship"] },
        {
            "name": "[QUESTION: Who uses this?]",
            "description": "[QUESTION: Who uses this?]",
            "goals": ["[QUESTION: What are their goals?]"]
        }
    ]);
    draft
}

#[test]
fn the_found_work_unit_is_a_backlog_task_with_the_foundation_event_storm_description() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid and no FOUND prefix is registered
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    assert!(!tmp.path().join("spec/prefixes.json").exists());

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=true
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And spec/work-units.json has FOUND-001 titled "Conduct Foundation Event Storm for Foundation" with type "task" and status "backlog"
    let board = read_json(&tmp.path().join("spec/work-units.json"));
    let unit = &board["workUnits"]["FOUND-001"];
    assert_eq!(
        unit["title"].as_str(),
        Some("Conduct Foundation Event Storm for Foundation"),
        "{unit}"
    );
    assert_eq!(unit["type"].as_str(), Some("task"), "{unit}");
    assert_eq!(unit["status"].as_str(), Some("backlog"), "{unit}");

    // @step And spec/work-units.json states.backlog lists FOUND-001
    assert_eq!(board["states"]["backlog"], json!(["FOUND-001"]), "{board}");

    // @step And spec/prefixes.json registers the FOUND prefix with the description "Foundation Event Storm tasks"
    let prefixes = read_json(&tmp.path().join("spec/prefixes.json"));
    assert_eq!(
        prefixes["prefixes"]["FOUND"]["description"].as_str(),
        Some("Foundation Event Storm tasks"),
        "{prefixes}"
    );

    // @step And FOUND-001's description lists add-foundation-bounded-context, add-aggregate-to-foundation, add-domain-event-to-foundation and show-foundation-event-storm
    let description = unit["description"].as_str().unwrap_or_default();
    for command in [
        "- fspec add-foundation-bounded-context <name>",
        "- fspec add-aggregate-to-foundation <context> <aggregate>",
        "- fspec add-domain-event-to-foundation <context> <event>",
        "- fspec show-foundation-event-storm",
    ] {
        assert!(
            description.contains(command),
            "missing {command:?}: {description}"
        );
    }

    // @step And FOUND-001's description explains why it matters and points to the "Foundation Event Storm" section of spec/AGENTS.md
    assert!(
        description.contains("\nWhy this matters:\n"),
        "{description}"
    );
    assert!(
        description.contains(
            "See spec/AGENTS.md \"Foundation Event Storm\" section for detailed guidance."
        ),
        "{description}"
    );
}

#[test]
fn discovery_without_finalize_creates_no_work_unit() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid and whose spec/work-units.json has no work units
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let board = r#"{"version":"0.7.1","workUnits":{},"states":{"backlog":[],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}}"#;
    fs::write(tmp.path().join("spec/work-units.json"), board).expect("write board");

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["valid"].as_bool(), Some(false), "{data}");

    // @step Then no FOUND work unit is created
    assert!(found_ids(tmp.path()).is_empty());

    // @step And spec/work-units.json is byte-equal to its pre-call contents
    assert_eq!(
        read_to_string(&tmp.path().join("spec/work-units.json")),
        board
    );
}

#[test]
fn a_refused_finalize_creates_no_work_unit() {
    // @step Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders and whose spec/work-units.json has no work units
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&placeholder_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let board = r#"{"version":"0.7.1","workUnits":{},"states":{"backlog":[],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}}"#;
    fs::write(tmp.path().join("spec/work-units.json"), board).expect("write board");

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false with the validationErrors explaining why
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["valid"].as_bool(), Some(false), "{data}");
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.starts_with("Cannot finalize: draft still has unfilled placeholder fields."),
        "validationErrors: {errors}"
    );

    // @step And no FOUND work unit is created
    assert!(found_ids(tmp.path()).is_empty());

    // @step And spec/work-units.json is byte-equal to its pre-call contents
    assert_eq!(
        read_to_string(&tmp.path().join("spec/work-units.json")),
        board
    );
}

#[test]
fn finalize_is_refused_while_a_template_persona_remains_next_to_a_real_persona() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is filled, with capabilities, a real persona and the template persona "[QUESTION: Who uses this?]"
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&draft_with_template_persona()).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["valid"].as_bool(), Some(false), "{data}");

    // @step And the returned validationErrors contains "Cannot finalize: draft still has unfilled placeholder fields"
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.contains("Cannot finalize: draft still has unfilled placeholder fields"),
        "validationErrors: {errors}"
    );

    // @step And the returned validationErrors lists only personas, as "1. personas (placeholder-entries)" with its fix command fspec add-persona
    assert!(
        errors.contains(
            "  1. personas (placeholder-entries) → fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"\n"
        ),
        "validationErrors: {errors}"
    );
    assert!(!errors.contains("  2. "), "validationErrors: {errors}");

    // @step And spec/foundation.json is not created
    assert!(!tmp.path().join("spec/foundation.json").exists());

    // @step And spec/foundation.json.draft on disk is byte-equal to its pre-call contents
    assert_eq!(
        read_to_string(&tmp.path().join("spec/foundation.json.draft")),
        body
    );
}

#[test]
fn finalize_lists_every_unfilled_field_with_its_fix_command_in_one_message() {
    // @step Given a project root tempdir whose spec/foundation.json.draft has placeholders in project.vision, solutionSpace.overview and personas
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = draft_with_template_persona();
    draft["project"]["vision"] = json!("[QUESTION: What is the one-sentence vision?]");
    draft["solutionSpace"]["overview"] = json!("[QUESTION: What can users DO?]");
    let body = serde_json::to_string_pretty(&draft).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["valid"].as_bool(), Some(false), "{data}");

    // @step And the returned validationErrors lists the three fields, numbered, each with its fix command
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    for line in [
        "  1. projectVision (placeholder) → fspec update-foundation projectVision \"<value>\"\n",
        "  2. solutionOverview (placeholder) → fspec update-foundation solutionOverview \"<value>\"\n",
        "  3. personas (placeholder-entries) → fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"\n",
    ] {
        assert!(errors.contains(line), "missing {line:?} in validationErrors: {errors}");
    }
    assert!(!errors.contains("  4. "), "validationErrors: {errors}");

    // @step And the returned validationErrors ends with "Then re-run: fspec discover-foundation --finalize"
    assert!(
        errors.ends_with("Then re-run: fspec discover-foundation --finalize"),
        "validationErrors: {errors}"
    );
}
