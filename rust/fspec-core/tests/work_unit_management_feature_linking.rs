// Feature: spec/features/work-unit-management.feature
//
// Work units link to feature files through work-unit ID tags; nothing is
// stored in work-units.json. These dispatcher tests run the real commands
// (create-story, add-tag-to-feature, remove-tag-from-feature,
// show-work-unit, show-feature, validate-tags) on temporary projects and
// check the linking from both sides. Each scenario maps to one #[test] with
// @step comments matching the Gherkin steps verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────────────────────── helpers ─────────────────────────

fn run(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn ok(root: &Path, command: &str, args: Value) -> String {
    let result = run(root, command, args);
    assert!(result.success, "{command} failed: {result:?}");
    result.data
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

/// A project with spec/, a foundation (create-story needs one), the AUTH
/// prefix and a tag registry with the required component and feature-group
/// tags the feature carries.
fn project_with_auth_prefix() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "spec/foundation.json", r#"{"version":"2.0.0"}"#);
    write(
        root,
        "spec/prefixes.json",
        &serde_json::to_string_pretty(&json!({
            "prefixes": {
                "AUTH": {"prefix": "AUTH", "description": "AUTH features", "createdAt": "2026-06-01T00:00:00.000Z"}
            }
        }))
        .unwrap(),
    );
    write(
        root,
        "spec/tags.json",
        &serde_json::to_string_pretty(&json!({
            "categories": [
                {"name": "Component Tags", "description": "", "required": true,
                 "tags": [{"name": "@cli", "description": "CLI"}]},
                {"name": "Feature Group Tags", "description": "", "required": true,
                 "tags": [{"name": "@auth", "description": "Authentication"}]}
            ]
        }))
        .unwrap(),
    );
    tmp
}

/// Work units AUTH-001.. with the given titles, all in backlog.
fn write_work_units(root: &Path, units: &[(&str, &str)]) {
    let mut wus = serde_json::Map::new();
    let mut backlog = Vec::new();
    for (id, title) in units {
        wus.insert(
            (*id).to_string(),
            json!({
                "id": id, "title": title, "type": "story", "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
            }),
        );
        backlog.push(json!(id));
    }
    write(
        root,
        "spec/work-units.json",
        &serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "workUnits": Value::Object(wus),
            "states": {
                "backlog": backlog, "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }))
        .unwrap(),
    );
}

const OAUTH_LOGIN_UNTAGGED: &str = "@cli
@auth
Feature: OAuth Login

  Scenario: Login with Google
    Given a registered user

  Scenario: Login with GitHub
    Given a registered user
";

/// add-tag-to-feature puts a work-unit ID tag first.
const OAUTH_LOGIN_TAGGED: &str = "@AUTH-001
@cli
@auth
Feature: OAuth Login

  Scenario: Login with Google
    Given a registered user

  Scenario: Login with GitHub
    Given a registered user
";

fn show_work_unit(root: &Path, id: &str) -> String {
    ok(root, "show-work-unit", json!({"workUnitId": id}))
}

fn show_feature(root: &Path, file: &str) -> String {
    ok(root, "show-feature", json!({"feature": file}))
}

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn complete_end_to_end_work_unit_to_feature_linking_workflow() {
    // @step Given I have a project with spec directory
    // @step And the prefix "AUTH" is registered in spec/prefixes.json
    let tmp = project_with_auth_prefix();
    let root = tmp.path();
    // @step And a feature file "oauth-login.feature" exists without work unit tags
    write(
        root,
        "spec/features/oauth-login.feature",
        OAUTH_LOGIN_UNTAGGED,
    );

    // @step When I run "fspec create-story AUTH 'OAuth Login Implementation'"
    let created = run(
        root,
        "create-story",
        json!({"prefix": "AUTH", "title": "OAuth Login Implementation"}),
    );
    // @step Then the command should succeed
    assert!(created.success, "create-story failed: {created:?}");
    // @step And a work unit "AUTH-001" should be created with title "OAuth Login Implementation"
    let units: Value = serde_json::from_str(&read(root, "spec/work-units.json")).unwrap();
    assert_eq!(
        units["workUnits"]["AUTH-001"]["title"],
        json!("OAuth Login Implementation")
    );

    // @step When I run "fspec add-tag-to-feature oauth-login.feature @AUTH-001"
    let tagged = run(
        root,
        "add-tag-to-feature",
        json!({"file": "spec/features/oauth-login.feature", "tags": ["@AUTH-001"]}),
    );
    // @step Then the command should succeed
    assert!(tagged.success, "add-tag-to-feature failed: {tagged:?}");
    // @step And the feature file should contain tag "@AUTH-001"
    assert_eq!(
        read(root, "spec/features/oauth-login.feature"),
        OAUTH_LOGIN_TAGGED
    );

    // @step When I run "fspec show-work-unit AUTH-001"
    // @step Then the command should succeed
    let shown = show_work_unit(root, "AUTH-001");
    // @step And the output should show linked feature "oauth-login.feature"
    // @step And the output should list all scenarios from the feature
    assert!(
        shown.contains(
            "\nLinked Features:\n\n  spec/features/oauth-login.feature\n    spec/features/oauth-login.feature:6 - Login with Google\n    spec/features/oauth-login.feature:9 - Login with GitHub\n"
        ),
        "show-work-unit output:\n{shown}"
    );

    // @step When I run "fspec show-feature oauth-login.feature"
    // @step Then the command should succeed
    let feature = show_feature(root, "spec/features/oauth-login.feature");
    // @step And the output should show work unit "AUTH-001" linked to this feature
    assert!(
        feature.ends_with(
            "\n\nWork Units:\n\n  AUTH-001 (feature-level) - OAuth Login Implementation\n    oauth-login.feature:6 - Login with Google\n    oauth-login.feature:9 - Login with GitHub\n\n"
        ),
        "show-feature output:\n{feature}"
    );

    // @step When I run "fspec validate-tags oauth-login.feature"
    let validated = run(
        root,
        "validate-tags",
        json!({"file": "spec/features/oauth-login.feature"}),
    );
    // @step Then the command should succeed
    assert!(validated.success, "validate-tags failed: {validated:?}");
    // @step And work unit tag "@AUTH-001" should be validated against spec/work-units.json
    let report: Value = serde_json::from_str(&validated.data).unwrap();
    assert_eq!(report["validCount"], json!(1), "report: {report}");
    assert_eq!(report["invalidCount"], json!(0), "report: {report}");
}

#[test]
fn remove_work_unit_tag_from_feature_and_verify_unlinking() {
    // @step Given I have a project with spec directory
    // @step And the prefix "AUTH" is registered in spec/prefixes.json
    let tmp = project_with_auth_prefix();
    let root = tmp.path();
    // @step And a work unit "AUTH-001" exists with title "OAuth Login Implementation"
    write_work_units(root, &[("AUTH-001", "OAuth Login Implementation")]);
    // @step And a feature file "oauth-login.feature" is tagged with "@AUTH-001"
    write(
        root,
        "spec/features/oauth-login.feature",
        OAUTH_LOGIN_TAGGED,
    );

    // @step When I run "fspec show-work-unit AUTH-001"
    // @step Then the command should succeed
    let before = show_work_unit(root, "AUTH-001");
    // @step And the output should show linked feature "oauth-login.feature"
    assert!(
        before.contains("\nLinked Features:\n\n  spec/features/oauth-login.feature\n"),
        "show-work-unit output:\n{before}"
    );

    // @step When I run "fspec remove-tag-from-feature oauth-login.feature @AUTH-001"
    let removed = run(
        root,
        "remove-tag-from-feature",
        json!({"file": "spec/features/oauth-login.feature", "tags": ["@AUTH-001"]}),
    );
    // @step Then the command should succeed
    assert!(
        removed.success,
        "remove-tag-from-feature failed: {removed:?}"
    );
    // @step And the feature file should not contain tag "@AUTH-001"
    assert_eq!(
        read(root, "spec/features/oauth-login.feature"),
        OAUTH_LOGIN_UNTAGGED
    );

    // @step When I run "fspec show-work-unit AUTH-001"
    // @step Then the command should succeed
    let after = show_work_unit(root, "AUTH-001");
    // @step And the output should have no Linked Features section
    assert!(
        !after.contains("Linked Features"),
        "show-work-unit output:\n{after}"
    );

    // @step When I run "fspec show-feature oauth-login.feature"
    // @step Then the command should succeed
    let feature = show_feature(root, "spec/features/oauth-login.feature");
    // @step And the output should not show work unit "AUTH-001"
    assert!(
        feature.ends_with("\n\nWork Units: None\n\n"),
        "show-feature output:\n{feature}"
    );
    assert!(
        !feature.contains("AUTH-001"),
        "show-feature output:\n{feature}"
    );
}

#[test]
fn show_work_unit_with_scenario_level_tags() {
    // @step Given I have a project with spec directory
    let tmp = project_with_auth_prefix();
    let root = tmp.path();
    // @step And a work unit "AUTH-001" exists
    // @step And a work unit "AUTH-002" exists
    write_work_units(root, &[("AUTH-001", "OAuth"), ("AUTH-002", "Tokens")]);
    // @step And a feature file "oauth.feature" has feature-level tag "@AUTH-001"
    // @step And the feature has a scenario "Login" inheriting @AUTH-001
    // @step And the feature has a scenario "Refresh tokens" with tag "@AUTH-002"
    write(
        root,
        "spec/features/oauth.feature",
        "@auth\n@AUTH-001\nFeature: OAuth\n\n  Scenario: Login\n    Given a registered user\n\n  @AUTH-002\n  Scenario: Refresh tokens\n    Given an expired token\n",
    );

    // @step When I run "fspec show-work-unit AUTH-001"
    // @step Then the command should succeed
    let first = show_work_unit(root, "AUTH-001");
    // @step And the output should show scenario "Login" under AUTH-001
    // @step And the output should not show scenario "Refresh tokens"
    assert!(
        first.contains(
            "\nLinked Features:\n\n  spec/features/oauth.feature\n    spec/features/oauth.feature:5 - Login\n\n"
        ),
        "show-work-unit AUTH-001 output:\n{first}"
    );
    assert!(!first.contains("Refresh tokens"), "output:\n{first}");

    // @step When I run "fspec show-work-unit AUTH-002"
    let second = show_work_unit(root, "AUTH-002");
    // @step Then the output should show only scenario "Refresh tokens"
    assert!(
        second.contains(
            "\nLinked Features:\n\n  spec/features/oauth.feature\n    spec/features/oauth.feature:9 - Refresh tokens\n\n"
        ),
        "show-work-unit AUTH-002 output:\n{second}"
    );
    assert!(!second.contains(" - Login"), "output:\n{second}");
}

#[test]
fn show_work_unit_with_multiple_feature_files() {
    // @step Given I have a project with spec directory
    let tmp = project_with_auth_prefix();
    let root = tmp.path();
    // @step And a work unit "AUTH-001" exists
    write_work_units(root, &[("AUTH-001", "OAuth")]);
    // @step And feature file "oauth-login.feature" is tagged with "@AUTH-001"
    write(
        root,
        "spec/features/oauth-login.feature",
        OAUTH_LOGIN_TAGGED,
    );
    // @step And feature file "oauth-refresh.feature" is tagged with "@AUTH-001"
    write(
        root,
        "spec/features/oauth-refresh.feature",
        "@AUTH-001\nFeature: OAuth Refresh\n\n  Scenario: Refresh an expired token\n    Given an expired token\n",
    );

    // @step When I run "fspec show-work-unit AUTH-001"
    // @step Then the command should succeed
    let shown = show_work_unit(root, "AUTH-001");
    // @step And the output should show both feature files
    // @step And scenarios should be grouped by feature file
    assert!(
        shown.contains(
            "\nLinked Features:\n\n  spec/features/oauth-login.feature\n    spec/features/oauth-login.feature:6 - Login with Google\n    spec/features/oauth-login.feature:9 - Login with GitHub\n\n  spec/features/oauth-refresh.feature\n    spec/features/oauth-refresh.feature:4 - Refresh an expired token\n\n"
        ),
        "show-work-unit output:\n{shown}"
    );
}
