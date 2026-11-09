#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/generate-scenarios-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `generate-scenarios`
// (RPC-234). Each scenario maps to exactly one #[test] with @step comments
// mirroring the Gherkin steps verbatim. RED PHASE: the current stub returns
// NotYetPorted, so every test fails until commands::generate_scenarios::run
// is ported and wired into the dispatcher.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "generate-scenarios".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, data: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(data).expect("serialize work-units"),
    )
    .expect("write work-units.json");
}

/// Build a one-unit work-units.json `Value` with the canonical id/title/type/
/// status/createdAt/updatedAt shape. Callers mutate example-mapping fields
/// (`rules`, `examples`, `questions`, `userStory`, `architectureNotes`) before
/// writing it to disk.
fn unit_data(id: &str, title: &str, status: &str) -> Value {
    let wu = json!({
        "id": id,
        "title": title,
        "type": "story",
        "status": status,
        "createdAt": "2026-06-01T00:00:00.000Z",
        "updatedAt": "2026-06-01T00:00:00.000Z",
    });
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), Value::Array(arr));
    }
    json!({
        "version": "0.7.1",
        "workUnits": { id: wu },
        "states": Value::Object(states),
    })
}

fn empty_work_units() -> Value {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert((*st).to_string(), Value::Array(vec![]));
    }
    json!({
        "version": "0.7.1",
        "workUnits": {},
        "states": Value::Object(states),
    })
}

/// A work unit that satisfies every gate: a rule, one active example, and a
/// user story.
fn ready_unit(id: &str, title: &str) -> Value {
    let mut data = unit_data(id, title, "specifying");
    data["workUnits"][id]["rules"] =
        json!([{ "id": 0, "text": "Password must be 8+ characters", "deleted": false }]);
    data["workUnits"][id]["examples"] =
        json!([{ "id": 0, "text": "User views the account settings page", "deleted": false }]);
    data["workUnits"][id]["userStory"] = json!({ "role": "registered user", "action": "log in securely", "benefit": "access my account" });
    data
}

fn write_existing_feature(project_root: &Path, file: &str, scenario_name: &str) {
    let dir = project_root.join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir spec/features");
    let content = format!(
        "@EXIST-001\nFeature: Existing Capability\n\n  Scenario: {scenario_name}\n    Given I am a registered user\n    When I log in with valid credentials\n    Then I should see the dashboard\n"
    );
    fs::write(dir.join(file), content).expect("write existing feature");
}

/// Count `Scenario:` blocks in a generated feature file.
fn scenario_block_count(content: &str) -> usize {
    content
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count()
}

// ---------- scenarios ----------

#[test]
fn dispatch_creates_a_context_only_feature_file_for_a_complete_work_unit() {
    // Scenario: Dispatch creates a context-only feature file for a complete work unit

    // @step Given a project root tempdir whose work unit WU-1 has rules, an active example, and a user story
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "User Authentication"));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a feature file is created under spec/features for WU-1 containing zero Scenario blocks
    let feature = tmp.path().join("spec/features/user-authentication.feature");
    assert!(
        feature.exists(),
        "feature file must be created at {feature:?}"
    );
    let content = fs::read_to_string(&feature).expect("read generated feature");
    assert_eq!(
        scenario_block_count(&content),
        0,
        "generated file must contain ZERO Scenario blocks; got:\n{content}"
    );

    // @step Then the rendered output contains the substring "Created context-only feature file"
    assert!(
        result.data.contains("Created context-only feature file"),
        "expected creation message; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "ZERO scenarios"
    assert!(
        result.data.contains("ZERO scenarios"),
        "expected the ZERO scenarios reminder; got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_fails_for_a_missing_work_unit() {
    // Scenario: Dispatch fails for a missing work unit

    // @step Given a project root tempdir with an empty work-units store
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &empty_work_units());

    // @step When I dispatch generate-scenarios with workUnitId="MISSING-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "MISSING-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "does not exist"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("does not exist"),
        "expected 'does not exist'; got: {msg}"
    );
}

#[test]
fn dispatch_fails_when_a_question_is_unanswered() {
    // Scenario: Dispatch fails when a question is unanswered

    // @step Given a project root tempdir whose work unit WU-1 has an unanswered question
    let tmp = TempDir::new().expect("tempdir");
    let mut data = ready_unit("WU-1", "User Authentication");
    data["workUnits"]["WU-1"]["questions"] = json!([{ "id": 0, "text": "@human: Should we support OAuth?", "deleted": false, "selected": false }]);
    write_work_units(tmp.path(), &data);

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "unanswered question"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("unanswered question"),
        "expected 'unanswered question'; got: {msg}"
    );
}

#[test]
fn dispatch_fails_when_there_is_no_example_mapping_data() {
    // Scenario: Dispatch fails when there is no Example Mapping data

    // @step Given a project root tempdir whose work unit WU-1 has no rules and no examples
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &unit_data("WU-1", "User Authentication", "specifying"),
    );

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "No Example Mapping data found"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("No Example Mapping data found"),
        "expected 'No Example Mapping data found'; got: {msg}"
    );
}

#[test]
fn dispatch_fails_when_there_are_no_active_examples() {
    // Scenario: Dispatch fails when there are no active examples

    // @step Given a project root tempdir whose work unit WU-1 has a rule but only deleted examples
    let tmp = TempDir::new().expect("tempdir");
    let mut data = unit_data("WU-1", "User Authentication", "specifying");
    data["workUnits"]["WU-1"]["rules"] =
        json!([{ "id": 0, "text": "Password must be 8+ characters", "deleted": false }]);
    data["workUnits"]["WU-1"]["examples"] =
        json!([{ "id": 0, "text": "Deleted example", "deleted": true }]);
    write_work_units(tmp.path(), &data);

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "has no examples to generate scenarios from"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("has no examples to generate scenarios from"),
        "expected the no-examples error; got: {msg}"
    );
}

#[test]
fn dispatch_fails_when_the_target_feature_file_already_exists() {
    // Scenario: Dispatch fails when the target feature file already exists

    // @step Given a project root tempdir whose work unit WU-1 is ready and spec/features/wu-1.feature already exists
    let tmp = TempDir::new().expect("tempdir");
    // Title "Wu 1" kebab-cases to "wu-1", matching the pre-existing file.
    write_work_units(tmp.path(), &ready_unit("WU-1", "Wu 1"));
    let dir = tmp.path().join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir features");
    fs::write(dir.join("wu-1.feature"), "@WU-1\nFeature: Wu 1\n").expect("pre-existing feature");

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "already exists"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("already exists"),
        "expected 'already exists'; got: {msg}"
    );
}

#[test]
fn dispatch_blocks_on_a_duplicate_scenario_without_the_override_flag() {
    // Scenario: Dispatch blocks on a duplicate scenario without the override flag

    // @step Given a project root tempdir whose work unit WU-1 has an example that matches an existing scenario above threshold
    let tmp = TempDir::new().expect("tempdir");
    let mut data = ready_unit("WU-1", "User Authentication");
    data["workUnits"]["WU-1"]["examples"] = json!([
        { "id": 0, "text": "Given I am a registered user When I log in with valid credentials Then I should see the dashboard", "deleted": false }
    ]);
    write_work_units(tmp.path(), &data);
    write_existing_feature(
        tmp.path(),
        "existing.feature",
        "Given I am a registered user When I log in with valid credentials Then I should see the dashboard",
    );

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "DUPLICATE SCENARIOS DETECTED"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("DUPLICATE SCENARIOS DETECTED"),
        "expected the duplicate-detection reminder; got: {msg}"
    );
}

#[test]
fn dispatch_proceeds_past_duplicates_with_ignore_possible_duplicates() {
    // Scenario: Dispatch proceeds past duplicates with ignore-possible-duplicates

    // @step Given a project root tempdir whose work unit WU-1 has an example that matches an existing scenario above threshold
    let tmp = TempDir::new().expect("tempdir");
    let mut data = ready_unit("WU-1", "User Authentication");
    data["workUnits"]["WU-1"]["examples"] = json!([
        { "id": 0, "text": "Given I am a registered user When I log in with valid credentials Then I should see the dashboard", "deleted": false }
    ]);
    write_work_units(tmp.path(), &data);
    write_existing_feature(
        tmp.path(),
        "existing.feature",
        "Given I am a registered user When I log in with valid credentials Then I should see the dashboard",
    );

    // @step When I dispatch generate-scenarios with workUnitId="WU-1" and ignorePossibleDuplicates=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "WU-1", "ignorePossibleDuplicates": true }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a feature file is created under spec/features for WU-1 containing zero Scenario blocks
    let feature = tmp.path().join("spec/features/user-authentication.feature");
    assert!(
        feature.exists(),
        "feature file must be created at {feature:?}"
    );
    let content = fs::read_to_string(&feature).expect("read generated feature");
    assert_eq!(
        scenario_block_count(&content),
        0,
        "generated file must contain ZERO Scenario blocks; got:\n{content}"
    );
}

#[test]
fn dispatch_honours_an_explicit_feature_name() {
    // Scenario: Dispatch honours an explicit feature name

    // @step Given a project root tempdir whose work unit WU-1 is ready with title "Some Other Title"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "Some Other Title"));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1" and feature="login"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "WU-1", "feature": "login" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the file spec/features/login.feature exists on disk
    assert!(
        tmp.path().join("spec/features/login.feature").exists(),
        "spec/features/login.feature must exist"
    );
}

#[test]
fn background_falls_back_to_placeholder_tokens_without_a_user_story() {
    // Scenario: Background falls back to placeholder tokens without a user story

    // @step Given a project root tempdir whose work unit WU-1 is ready but has no user story
    let tmp = TempDir::new().expect("tempdir");
    let mut data = unit_data("WU-1", "User Authentication", "specifying");
    data["workUnits"]["WU-1"]["rules"] =
        json!([{ "id": 0, "text": "Password must be 8+ characters", "deleted": false }]);
    data["workUnits"]["WU-1"]["examples"] =
        json!([{ "id": 0, "text": "User views the account settings page", "deleted": false }]);
    write_work_units(tmp.path(), &data);

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the created feature file Background contains role, action and benefit placeholder tokens
    let feature = tmp.path().join("spec/features/user-authentication.feature");
    let content = fs::read_to_string(&feature).expect("read generated feature");
    assert!(
        content.contains("[role]") && content.contains("[action]") && content.contains("[benefit]"),
        "Background must contain placeholder tokens; got:\n{content}"
    );

    // @step Then the rendered output contains a prefill reminder
    assert!(
        result.data.contains("PREFILL DETECTED"),
        "expected a prefill reminder; got:\n{}",
        result.data
    );
}

#[test]
fn cli_and_dispatcher_converge_on_the_same_fspec_core_run_function() {
    // Scenario: CLI and dispatcher converge on the same fspec_core run function

    // @step Given a project root tempdir whose work unit WU-1 is ready
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "User Authentication"));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1" and also run the CLI subcommand fspec generate-scenarios WU-1 against an equivalent project root
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then both paths produce output containing "Created context-only feature file"
    assert!(
        result.data.contains("Created context-only feature file"),
        "dispatcher path must render the creation message; got:\n{}",
        result.data
    );

    // @step Then the CLI bridge module rust/fspec/src/generate_scenarios.rs contains no analysis, gap-detection, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fspec/src/generate_scenarios.rs");
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "CONTEXT-ONLY FEATURE FILE CREATED",
        "DUPLICATE SCENARIOS DETECTED",
        "PREFILL DETECTED",
        "No Example Mapping data found",
        "Background: User Story",
        "extractStepsFromExample",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ---------- scenarios merged from TypeScript-era features (COV-072) ----------

/// The `"""` docstring lines (fences included) of a feature file.
fn docstring_lines(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if line.trim() == "\"\"\"" {
            out.push(line.to_string());
            if inside {
                break;
            }
            inside = true;
        } else if inside {
            out.push(line.to_string());
        }
    }
    out
}

/// Zero-based index of the first line that satisfies `pred`.
fn line_index(content: &str, pred: impl Fn(&str) -> bool) -> usize {
    content
        .lines()
        .position(pred)
        .unwrap_or_else(|| panic!("no matching line in:\n{content}"))
}

#[test]
fn background_is_built_from_the_user_story_of_the_work_unit() {
    // @step Given a project root tempdir whose work unit WU-1 has rules, an active example, and a user story with role "registered user", action "log in securely" and benefit "access my account"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "User Authentication"));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the created feature file Background reads "As a registered user", "I want to log in securely" and "So that access my account"
    let content = fs::read_to_string(tmp.path().join("spec/features/user-authentication.feature"))
        .expect("read generated feature");
    let background = &content[content
        .find("  Background: User Story")
        .unwrap_or_else(|| panic!("no Background in:\n{content}"))..];
    let background_lines: Vec<&str> = background.lines().map(str::trim).collect();
    assert_eq!(
        background_lines,
        vec![
            "Background: User Story",
            "As a registered user",
            "I want to log in securely",
            "So that access my account",
        ],
        "got:\n{content}"
    );

    // @step And the created feature file contains none of the placeholder tokens "[role]", "[action]" and "[benefit]"
    for token in ["[role]", "[action]", "[benefit]"] {
        assert!(
            !content.contains(token),
            "placeholder {token} must not appear; got:\n{content}"
        );
    }
}

#[test]
fn generated_file_places_the_architecture_docstring_between_the_feature_line_and_the_example_mapping_comments(
) {
    // @step Given a project root tempdir whose work unit WU-1 has rules, an active example, a user story and no architecture notes
    let tmp = TempDir::new().expect("tempdir");
    let data = ready_unit("WU-1", "User Authentication");
    assert!(data["workUnits"]["WU-1"].get("architectureNotes").is_none());
    write_work_units(tmp.path(), &data);

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the created feature file lists the "@WU-1" tag line, the "Feature:" line, the docstring, the "# EXAMPLE MAPPING CONTEXT" comments and the "Background: User Story" line in that order
    let content = fs::read_to_string(tmp.path().join("spec/features/user-authentication.feature"))
        .expect("read generated feature");
    let order = [
        line_index(&content, |l| l == "@WU-1"),
        line_index(&content, |l| l.starts_with("Feature:")),
        line_index(&content, |l| l.trim() == "\"\"\""),
        line_index(&content, |l| l.trim() == "# EXAMPLE MAPPING CONTEXT"),
        line_index(&content, |l| l.trim() == "Background: User Story"),
    ];
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "expected tag -> Feature -> docstring -> comments -> Background, got line indices {order:?} in:\n{content}"
    );
    let docstring = docstring_lines(&content);
    let closing_fence = order[2] + docstring.len() - 1;
    assert!(
        closing_fence < order[3],
        "the docstring must close before the comments; got:\n{content}"
    );

    // @step And the docstring contains "Architecture notes:" and the three TODO placeholder lines
    assert_eq!(
        docstring,
        vec![
            "  \"\"\"",
            "  Architecture notes:",
            "  - TODO: Add key architectural decisions",
            "  - TODO: Add dependencies and integrations",
            "  - TODO: Add critical implementation requirements",
            "  \"\"\"",
        ],
        "got:\n{content}"
    );

    // @step And the docstring lines are identical to the docstring create-feature writes
    let created = dispatch_command(DispatchRequest {
        command: "create-feature".to_string(),
        args_json: json!({ "name": "Template Probe" }).to_string(),
        project_root: tmp.path().to_path_buf(),
    });
    assert!(created.success, "create-feature failed: {created:?}");
    let template = fs::read_to_string(tmp.path().join("spec/features/template-probe.feature"))
        .expect("read create-feature output");
    assert_eq!(docstring, docstring_lines(&template), "got:\n{template}");
}

#[test]
fn duplicate_reminder_names_the_files_to_investigate_the_next_steps_and_the_override_flag() {
    // @step Given a project root tempdir whose work unit WU-1 has an example that matches a scenario in spec/features/existing.feature above threshold
    let tmp = TempDir::new().expect("tempdir");
    let mut data = ready_unit("WU-1", "User Authentication");
    data["workUnits"]["WU-1"]["examples"] = json!([
        { "id": 0, "text": "Given I am a registered user When I log in with valid credentials Then I should see the dashboard", "deleted": false }
    ]);
    write_work_units(tmp.path(), &data);
    write_existing_feature(
        tmp.path(),
        "existing.feature",
        "Given I am a registered user When I log in with valid credentials Then I should see the dashboard",
    );

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message lists spec/features/existing.feature under "Feature files to investigate:"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Feature files to investigate:\n  - spec/features/existing.feature\n"),
        "expected the file list; got: {msg}"
    );

    // @step And the error message contains the substring "Next steps:"
    assert!(msg.contains("Next steps:"), "got: {msg}");

    // @step And the error message contains the substring "Run: fspec generate-scenarios WU-1 --ignore-possible-duplicates"
    assert!(
        msg.contains("Run: fspec generate-scenarios WU-1 --ignore-possible-duplicates"),
        "got: {msg}"
    );

    // @step And no feature file is created for WU-1
    assert!(
        !tmp.path()
            .join("spec/features/user-authentication.feature")
            .exists(),
        "a blocked run must not write the feature file"
    );
}

#[test]
fn dispatch_writes_the_file_without_duplicate_warnings_when_no_existing_scenario_matches() {
    // @step Given a project root tempdir whose work unit WU-1 is ready and spec/features/existing.feature holds one unrelated scenario
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "User Authentication"));
    let dir = tmp.path().join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir spec/features");
    fs::write(
        dir.join("existing.feature"),
        "@EXIST-001\nFeature: Invoice Export\n\n  Scenario: Export monthly invoices as PDF\n    Given the accountant has closed the month\n    When the invoices are exported\n    Then a PDF archive is produced\n",
    )
    .expect("write existing feature");

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And a feature file is created under spec/features for WU-1 containing zero Scenario blocks
    let content = fs::read_to_string(dir.join("user-authentication.feature"))
        .expect("read generated feature");
    assert_eq!(scenario_block_count(&content), 0, "got:\n{content}");

    // @step And the rendered output contains neither "DUPLICATE SCENARIOS DETECTED" nor "Detected potential refactor"
    assert!(
        !result.data.contains("DUPLICATE SCENARIOS DETECTED")
            && !result.data.contains("Detected potential refactor"),
        "no duplicate warning expected; got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_fails_when_the_work_unit_has_no_title_and_no_feature_name() {
    // @step Given a project root tempdir whose work unit WU-1 is ready but has an empty title
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", ""));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the substring "Cannot determine feature file name"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Cannot determine feature file name"),
        "got: {msg}"
    );

    // @step And the error message contains the substring "Use --feature flag with a capability-based name"
    assert!(
        msg.contains("Use --feature flag with a capability-based name"),
        "got: {msg}"
    );

    // @step And no feature file is written under spec/features
    let features = tmp.path().join("spec/features");
    let written = fs::read_dir(&features)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "feature"))
                .count()
        })
        .unwrap_or(0);
    assert_eq!(written, 0, "no feature file may be written");
}

#[test]
fn the_work_unit_id_is_written_as_a_feature_level_tag_only() {
    // @step Given a project root tempdir whose work unit WU-1 has rules, an active example, and a user story
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &ready_unit("WU-1", "User Authentication"));

    // @step When I dispatch generate-scenarios with workUnitId="WU-1"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "WU-1" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the first line of the created feature file is "@WU-1"
    let content = fs::read_to_string(tmp.path().join("spec/features/user-authentication.feature"))
        .expect("read generated feature");
    assert_eq!(content.lines().next(), Some("@WU-1"), "got:\n{content}");

    // @step And no other line of the created feature file carries the tag "@WU-1"
    let tagged = content
        .lines()
        .skip(1)
        .filter(|l| l.split_whitespace().any(|t| t == "@WU-1"))
        .count();
    assert_eq!(
        tagged, 0,
        "@WU-1 must appear only at feature level; got:\n{content}"
    );
}
