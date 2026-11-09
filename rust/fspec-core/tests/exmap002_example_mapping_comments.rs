// Feature: spec/features/preserve-example-mapping-context-as-comments-in-generated-feature-files.feature
//
// EXMAP-002: generate-scenarios writes the work unit's example mapping as a
// bordered `# EXAMPLE MAPPING CONTEXT` comment block above the Background
// (no scenarios), emits the scenario-generation reminder, and the commands
// that edit the file afterwards (add-architecture, add-scenario,
// add-background, format) keep that block. Every command runs through the
// dispatcher (the entry the CLI subcommands call) on a temporary project.
// One test file per feature (BUG-357); the formatter scenarios live in
// rust/fspec-core/src/io/gherkin_format/comment_tests.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

const WU: &str = "EXMAP-002";
const FEATURE_REL: &str = "spec/features/user-login.feature";

fn run(root: &Path, command: &str, args: Value) -> DispatchResult {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    assert!(result.success, "{command} failed: {result:?}");
    result
}

/// A project whose board holds one specifying story EXMAP-002 "User Login"
/// with no example mapping yet.
fn project() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let data = json!({
        "version": "0.7.1",
        "workUnits": {
            WU: {
                "id": WU,
                "title": "User Login",
                "type": "story",
                "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [WU], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&data).expect("serialize"),
    )
    .expect("write work-units.json");
    tmp
}

fn add_rule(root: &Path, rule: &str) {
    run(root, "add-rule", json!({ "workUnitId": WU, "rule": rule }));
}

fn add_example(root: &Path, example: &str) {
    run(
        root,
        "add-example",
        json!({ "workUnitId": WU, "example": example }),
    );
}

fn set_user_story(root: &Path) {
    run(
        root,
        "set-user-story",
        json!({
            "workUnitId": WU,
            "role": "registered user",
            "action": "log in securely",
            "benefit": "I can reach my account"
        }),
    );
}

/// The usual ready work unit: one rule, one example, a user story.
fn ready(root: &Path) {
    add_rule(root, "Passwords must be 8+ chars");
    add_example(root, "User logs in with valid creds");
    set_user_story(root);
}

fn generate(root: &Path) -> String {
    run(root, "generate-scenarios", json!({ "workUnitId": WU })).data
}

fn read_feature(root: &Path) -> String {
    fs::read_to_string(root.join(FEATURE_REL)).expect("read generated feature")
}

fn scenario_count(content: &str) -> usize {
    content
        .lines()
        .filter(|l| l.trim_start().starts_with("Scenario:"))
        .count()
}

/// The lines of the `# EXAMPLE MAPPING CONTEXT` block, from its first border
/// to its closing border.
fn comment_block(content: &str) -> Vec<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim() == "# ========================================")
        .unwrap_or_else(|| panic!("no comment block in:\n{content}"));
    let end = lines
        .iter()
        .rposition(|l| l.trim() == "# ========================================")
        .expect("closing border");
    lines[start..=end].to_vec()
}

const TODO_DOCSTRING: &str = "  \"\"\"\n  Architecture notes:\n  - TODO: Add key architectural decisions\n  - TODO: Add dependencies and integrations\n  - TODO: Add critical implementation requirements\n  \"\"\"\n";

const STORY_BACKGROUND: &str = "  Background: User Story\n    As a registered user\n    I want to log in securely\n    So that I can reach my account\n";

/// The generated file of the usual ready work unit.
const READY_FILE: &str = "@EXMAP-002\nFeature: User Login\n  \"\"\"\n  Architecture notes:\n  - TODO: Add key architectural decisions\n  - TODO: Add dependencies and integrations\n  - TODO: Add critical implementation requirements\n  \"\"\"\n\n  # ========================================\n  # EXAMPLE MAPPING CONTEXT\n  # ========================================\n  #\n  # BUSINESS RULES:\n  #   1. Passwords must be 8+ chars\n  #\n  # EXAMPLES:\n  #   1. User logs in with valid creds\n  #\n  # ========================================\n  Background: User Story\n    As a registered user\n    I want to log in securely\n    So that I can reach my account\n";

const READY_BLOCK: [&str; 11] = [
    "  # ========================================",
    "  # EXAMPLE MAPPING CONTEXT",
    "  # ========================================",
    "  #",
    "  # BUSINESS RULES:",
    "  #   1. Passwords must be 8+ chars",
    "  #",
    "  # EXAMPLES:",
    "  #   1. User logs in with valid creds",
    "  #",
    "  # ========================================",
];

// ---------- scenarios ----------

#[test]
fn example_mapping_comments_coexist_with_architecture_doc_strings() {
    // @step Given I generate scenarios from a work unit
    let tmp = project();
    let root = tmp.path();
    ready(root);
    generate(root);

    // @step And the feature file has "# EXAMPLE MAPPING CONTEXT" comments
    assert_eq!(read_feature(root), READY_FILE);

    // @step When I run "fspec add-architecture" to add technical notes
    run(
        root,
        "add-architecture",
        json!({ "feature": FEATURE_REL, "text": "Uses bcrypt for password hashing" }),
    );

    // @step Then the file should contain both # comments AND """ doc string
    let after = read_feature(root);
    assert!(
        after.contains("  \"\"\"\n  Uses bcrypt for password hashing\n  \"\"\"\n"),
        "{after}"
    );
    assert_eq!(comment_block(&after), READY_BLOCK, "{after}");

    // @step And neither should overwrite the other
    assert_eq!(
        after,
        READY_FILE.replace(
            TODO_DOCSTRING,
            "  \"\"\"\n  Uses bcrypt for password hashing\n  \"\"\"\n"
        ),
        "only the doc string changes"
    );

    // @step And format command should preserve both
    run(root, "format", json!({ "file": FEATURE_REL }));
    assert_eq!(read_feature(root), after, "format changes nothing");
}

#[test]
fn generate_context_only_feature_file_with_no_scenarios() {
    // @step Given I have a work unit EXMAP-002 with example mapping data
    let tmp = project();
    let root = tmp.path();
    set_user_story(root);

    // @step And the work unit has 3 rules and 5 examples
    for rule in ["Rule one", "Rule two", "Rule three"] {
        add_rule(root, rule);
    }
    for example in [
        "Example one",
        "Example two",
        "Example three",
        "Example four",
        "Example five",
    ] {
        add_example(root, example);
    }

    // @step When I run "fspec generate-scenarios EXMAP-002"
    let output = generate(root);

    // @step Then a feature file should be created
    assert!(
        output.starts_with(&format!(
            "✓ Created context-only feature file: {}\n  Contains example mapping context as comments (NO scenarios yet)\n",
            root.join(FEATURE_REL).display()
        )),
        "{output}"
    );
    let content = read_feature(root);

    // @step And the file should contain # comment block with all rules and examples
    assert_eq!(
        comment_block(&content),
        [
            "  # ========================================",
            "  # EXAMPLE MAPPING CONTEXT",
            "  # ========================================",
            "  #",
            "  # BUSINESS RULES:",
            "  #   1. Rule one",
            "  #   2. Rule two",
            "  #   3. Rule three",
            "  #",
            "  # EXAMPLES:",
            "  #   1. Example one",
            "  #   2. Example two",
            "  #   3. Example three",
            "  #   4. Example four",
            "  #   5. Example five",
            "  #",
            "  # ========================================",
        ]
    );

    // @step And the file should contain Background with user story
    assert!(
        content.ends_with(&format!(
            "  # ========================================\n{STORY_BACKGROUND}"
        )),
        "{content}"
    );

    // @step And the file should contain ZERO scenarios
    assert_eq!(scenario_count(&content), 0, "{content}");

    // @step And a system-reminder should tell AI to write scenarios
    assert!(
        output.contains("<system-reminder>\nCONTEXT-ONLY FEATURE FILE CREATED\n"),
        "{output}"
    );
    assert!(
        output.contains("  ✗ ZERO scenarios (AI must write them)\n"),
        "{output}"
    );
}

#[test]
fn user_story_only_in_background_section_not_in_comments() {
    // @step Given I set user story for work unit using "fspec set-user-story"
    let tmp = project();
    let root = tmp.path();
    ready(root);

    // @step When I run "fspec generate-scenarios" on the work unit
    generate(root);
    let content = read_feature(root);

    // @step Then the feature file # comments should NOT contain the user story
    let comments: Vec<&str> = content
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .collect();
    assert_eq!(comments, READY_BLOCK, "{content}");
    for part in [
        "registered user",
        "log in securely",
        "I can reach my account",
    ] {
        assert!(
            comments.iter().all(|l| !l.contains(part)),
            "{part}: {content}"
        );
    }

    // @step And the Background section should contain the user story
    assert!(content.ends_with(STORY_BACKGROUND), "{content}");

    // @step And the user story should only appear in Background (FEAT-012 separation)
    for line in STORY_BACKGROUND.lines().skip(1) {
        assert_eq!(content.matches(line.trim()).count(), 1, "{line}: {content}");
    }
}

#[test]
fn system_reminder_guides_ai_to_write_scenarios_from_examples() {
    // @step Given I generate scenarios for a work unit
    let tmp = project();
    let root = tmp.path();
    add_rule(root, "Passwords must be 8+ chars");
    add_example(root, "User logs in with valid creds");
    add_example(root, "User logs in with a wrong password");
    set_user_story(root);
    let output = generate(root);

    // @step And the feature file lists "1. User logs in with valid creds" under "# EXAMPLES:"
    let content = read_feature(root);
    assert!(
        content.contains(
            "  # EXAMPLES:\n  #   1. User logs in with valid creds\n  #   2. User logs in with a wrong password\n"
        ),
        "{content}"
    );

    // @step When the command completes
    let feature_path = root.join(FEATURE_REL).display().to_string();

    // @step Then a system-reminder should be emitted
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    assert!(
        output.trim_end().ends_with("</system-reminder>"),
        "{output}"
    );

    // @step And the reminder should say "Write scenarios based on # EXAMPLES section"
    assert!(
        output.contains("NEXT STEP: Write scenarios based on # EXAMPLES section\n"),
        "{output}"
    );

    // @step And the reminder should list all examples found
    assert!(
        output.contains(
            "Classify each example - does it describe registered user's experience?\n  1. \"User logs in with valid creds\"\n     Describes registered user's experience? [YES/NO]\n  2. \"User logs in with a wrong password\"\n     Describes registered user's experience? [YES/NO]\n"
        ),
        "{output}"
    );

    // @step And the reminder should instruct using Edit tool
    assert!(
        output.contains(&format!(
            "  3. Use the Edit tool to add scenarios to {feature_path}\n"
        )),
        "{output}"
    );
}

#[test]
fn answered_questions_preserved_in_comments() {
    // @step Given I answer question during example mapping
    let tmp = project();
    let root = tmp.path();
    ready(root);
    run(
        root,
        "add-question",
        json!({ "workUnitId": WU, "question": "@human: OAuth support?" }),
    );

    // @step And the answer says "OAuth support deferred to Phase 2"
    run(
        root,
        "answer-question",
        json!({ "workUnitId": WU, "index": 0, "answer": "OAuth support deferred to Phase 2" }),
    );

    // @step When I generate scenarios from the work unit
    generate(root);
    let content = read_feature(root);

    // @step Then the feature file should contain "# QUESTIONS (ANSWERED):" section
    // @step And the section should show "Q: OAuth support?" followed by "A: OAuth support deferred to Phase 2"
    assert!(
        content.contains(
            "  #   1. User logs in with valid creds\n  #\n  # QUESTIONS (ANSWERED):\n  #   Q: OAuth support?\n  #   A: OAuth support deferred to Phase 2\n  #\n  # ========================================\n"
        ),
        "{content}"
    );
}

#[test]
fn assumptions_documented_in_comments() {
    // @step Given I add assumption "email verification handled by external service"
    let tmp = project();
    let root = tmp.path();
    ready(root);
    run(
        root,
        "add-assumption",
        json!({ "workUnitId": WU, "assumption": "email verification handled by external service" }),
    );

    // @step When I generate scenarios from the work unit
    let output = generate(root);
    let content = read_feature(root);

    // @step Then the feature file should contain "# ASSUMPTIONS:" section
    // @step And the section should list the email verification assumption
    assert!(
        content.contains(
            "  #   1. User logs in with valid creds\n  #\n  # ASSUMPTIONS:\n  #   1. email verification handled by external service\n  #\n  # ========================================\n"
        ),
        "{content}"
    );

    // @step And the scenario-generation reminder says "Check # ASSUMPTIONS to know what NOT to test"
    assert!(
        output.contains("  6. Check # ASSUMPTIONS to know what NOT to test\n"),
        "{output}"
    );
}

#[test]
fn comment_block_has_visual_borders_for_easy_identification() {
    // @step Given I generate scenarios from a work unit
    let tmp = project();
    let root = tmp.path();
    ready(root);
    generate(root);

    // @step When I view the generated feature file
    let content = read_feature(root);
    let block = comment_block(&content);

    // @step Then the comment block should have "# ===" separator lines at top and bottom
    assert_eq!(
        block.first(),
        Some(&"  # ========================================")
    );
    assert_eq!(
        block.last(),
        Some(&"  # ========================================")
    );

    // @step And the block starts with "# EXAMPLE MAPPING CONTEXT" between the first two separator lines
    assert_eq!(
        block[..3],
        [
            "  # ========================================",
            "  # EXAMPLE MAPPING CONTEXT",
            "  # ========================================",
        ]
    );
}

#[test]
fn add_scenario_command_preserves_example_mapping_comments() {
    // @step Given a feature file has # example mapping comments at the top
    let tmp = project();
    let root = tmp.path();
    ready(root);
    generate(root);
    assert_eq!(read_feature(root), READY_FILE);

    // @step When I run "fspec add-scenario" to manually add a scenario
    run(
        root,
        "add-scenario",
        json!({ "feature": FEATURE_REL, "scenario": "User logs in with valid credentials" }),
    );
    let after = read_feature(root);

    // @step Then the new scenario should be appended at the end
    let tail = after
        .strip_prefix(READY_FILE)
        .unwrap_or_else(|| panic!("the generated text must stay as it was:\n{after}"));
    assert!(
        tail.starts_with("\n  Scenario: User logs in with valid credentials\n"),
        "{after}"
    );
    assert_eq!(scenario_count(&after), 1, "{after}");

    // @step And the # comments at the top should remain unchanged
    assert_eq!(comment_block(&after), READY_BLOCK, "{after}");

    // @step And no comments should be removed
    assert_eq!(
        after
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count(),
        READY_BLOCK.len(),
        "{after}"
    );
}

#[test]
fn add_background_command_preserves_example_mapping_comments() {
    // @step Given a feature file has # comments before Background section
    let tmp = project();
    let root = tmp.path();
    ready(root);
    generate(root);
    assert_eq!(read_feature(root), READY_FILE);

    // @step When I run "fspec add-background" to update the user story
    run(
        root,
        "add-background",
        json!({
            "feature": FEATURE_REL,
            "text": "As a returning user\nI want to log in with a passkey\nSo that I skip typing passwords"
        }),
    );
    let after = read_feature(root);

    // @step Then the Background section should be replaced
    assert!(
        after.contains("    As a returning user\n    I want to log in with a passkey\n    So that I skip typing passwords\n"),
        "{after}"
    );
    assert!(!after.contains("As a registered user"), "{after}");

    // @step And the # comments above Background should remain intact
    assert_eq!(comment_block(&after), READY_BLOCK, "{after}");
    let block_end = after
        .find("  # ========================================\n  Background:")
        .map(|i| i + "  # ========================================\n".len());
    assert!(
        block_end.is_some(),
        "comment block directly above Background:\n{after}"
    );

    // @step And no comments should be lost
    assert_eq!(
        after
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count(),
        READY_BLOCK.len(),
        "{after}"
    );
}

#[test]
fn prefill_detection_still_works_after_adding_comments() {
    // @step Given I generate scenarios without setting user story
    let tmp = project();
    let root = tmp.path();
    add_rule(root, "Passwords must be 8+ chars");
    add_example(root, "User logs in with valid creds");
    let output = generate(root);

    // @step And the Background contains placeholder text
    let content = read_feature(root);
    assert!(
        content.ends_with(
            "  # ========================================\n  Background: User Story\n    As a [role]\n    I want to [action]\n    So that [benefit]\n"
        ),
        "{content}"
    );

    // @step When the command completes
    let reminder_start = output.find("<system-reminder>").expect("a reminder");
    let reminder = &output[reminder_start..];

    // @step Then a system-reminder should detect the prefill
    assert!(
        reminder.contains("PREFILL DETECTED in feature file.\n\nFound 6 placeholder(s)"),
        "{output}"
    );

    // @step And the reminder should suggest "fspec set-user-story" command
    assert!(
        reminder.contains(
            "  Line 22: [role] → Use 'fspec set-user-story' to replace this placeholder\n"
        ) && reminder.contains("  - Use 'fspec set-user-story' to replace this placeholder\n"),
        "{output}"
    );

    // @step And the prefill reminder should coexist with scenario generation reminder
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    let generation = reminder
        .find("CONTEXT-ONLY FEATURE FILE CREATED")
        .expect("generation reminder");
    let prefill = reminder.find("PREFILL DETECTED").expect("prefill reminder");
    assert!(generation < prefill, "{output}");
}

#[test]
fn zero_scenarios_means_file_incomplete_system_reminder_triggered() {
    // @step Given I generate scenarios from a work unit
    let tmp = project();
    let root = tmp.path();
    ready(root);
    let output = generate(root);
    let content = read_feature(root);

    // @step And the generated file has # comments + Background
    assert_eq!(content, READY_FILE);

    // @step But the file has zero Scenario blocks
    assert_eq!(scenario_count(&content), 0, "{content}");

    // @step When the command completes
    let feature_path = root.join(FEATURE_REL).display().to_string();

    // @step Then a system-reminder should be emitted
    assert!(output.contains("<system-reminder>\n"), "{output}");

    // @step And the reminder should say "ZERO scenarios (AI must write them)"
    assert!(
        output.contains(&format!(
            "The feature file {feature_path} contains:\n  ✓ Example mapping context as comments (# EXAMPLE MAPPING CONTEXT)\n  ✓ Background section with user story\n  ✗ ZERO scenarios (AI must write them)\n"
        )),
        "{output}"
    );

    // @step And the reminder should reference the # EXAMPLES section
    assert!(
        output.contains(
            "  2. For each example in # EXAMPLES, write a corresponding Scenario block\n"
        ),
        "{output}"
    );
}
