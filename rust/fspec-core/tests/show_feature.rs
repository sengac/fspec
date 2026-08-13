#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/show-feature-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `show-feature`
// (RPC-304). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-feature".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_work_units(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a `spec/work-units.json` payload with the given (id, title, status) entries.
fn work_units_with(entries: &[(&str, &str, &str)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, title, status) in entries {
        wus.insert(
            id.to_string(),
            json!({
                "id": id,
                "title": title,
                "status": status,
                "createdAt": "x",
                "updatedAt": "x"
            }),
        );
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

const LOGIN_FEATURE_NO_TAGS: &str = "Feature: Login\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    When I submit credentials\n    Then I see the dashboard\n";

// ───────── scenarios ─────────

#[test]
fn bare_name_lookup_resolves_to_spec_features_file_with_no_wu_tags() {
    // Scenario: Bare-name lookup resolves to spec/features/<name>.feature and renders Work Units None when no WU tags present

    // @step Given a temp project root contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/login.feature",
        LOGIN_FEATURE_NO_TAGS,
    );

    // @step When I dispatch show-feature with feature='login' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "login", "format": "text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the data field reproduces the file content verbatim followed by '\n\nWork Units: None\n\n'
    // TS-parity: the text render emits a final trailing newline to mirror the
    // TS CLI's terminating `output.log('')` empty-line.
    let expected = format!("{LOGIN_FEATURE_NO_TAGS}\n\nWork Units: None\n\n");
    assert_eq!(
        result.data, expected,
        "expected file body + sentinel; got:\n{}\n",
        result.data
    );
}

#[test]
fn direct_path_lookup_resolves_a_feature_path_ending_in_dot_feature() {
    // Scenario: Direct path lookup resolves a feature path ending in .feature

    // @step Given a temp project root contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/login.feature",
        LOGIN_FEATURE_NO_TAGS,
    );

    // @step When I dispatch show-feature with feature='spec/features/login.feature' and format='text'
    let direct = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "format": "text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(direct.success, "{direct:?}");

    // @step And the data field equals the data returned for the bare-name lookup with feature='login'
    let bare = dispatch_command(req(
        tmp.path(),
        json!({"feature": "login", "format": "text"}),
    ));
    assert!(bare.success, "{bare:?}");
    assert_eq!(
        direct.data, bare.data,
        "direct-path and bare-name resolution must agree byte-for-byte"
    );
}

#[test]
fn missing_feature_file_returns_canonical_not_found_error() {
    // Scenario: Missing feature file returns Feature file not found with the unresolved input

    // @step Given a temp project root with no spec/features/ directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch show-feature with feature='missing-name' and format='text'
    // (Per the list_feature_tags pattern, format='json' gives us a clean
    // JSON envelope to assert against. The dispatcher's outer success flag
    // remains true; the structured error lives inside the JSON envelope.)
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "missing-name", "format": "json"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope must be success=true; structured error lives in data: {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}; got:\n{}", result.data));
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error field equals 'Feature file not found: missing-name'
    assert_eq!(
        data["error"].as_str(),
        Some("Feature file not found: missing-name")
    );
}

#[test]
fn invalid_gherkin_returns_invalid_gherkin_syntax_prefix() {
    // Scenario: Invalid Gherkin syntax returns success false with Invalid Gherkin syntax prefix

    // @step Given a temp project root contains spec/features/broken.feature with the bytes 'this is not gherkin'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/broken.feature",
        "this is not gherkin",
    );

    // @step When I dispatch show-feature with feature='broken' and format='text'
    // (We dispatch with format='json' to assert against the structured
    // envelope shape; the text-format path simply renders error.clone().)
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "broken", "format": "json"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "outer envelope success flag must remain true; structured error in data"
    );
    let data: Value = serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}; got:\n{}", result.data));
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error field starts with the prefix 'Invalid Gherkin syntax: '
    let err = data["error"].as_str().expect("error string");
    assert!(
        err.starts_with("Invalid Gherkin syntax: "),
        "expected canonical prefix; got: {err}"
    );
}

#[test]
fn feature_level_wu_tag_attaches_to_scenarios_without_their_own_wu_tag() {
    // Scenario: Feature-level work-unit tag attaches to scenarios that lack their own WU tag

    // @step Given a temp project root contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with two scenarios 'A' on line 4 and 'B' on line 7 with no scenario-level work-unit tags
    let tmp = TempDir::new().expect("tempdir");
    // Layout (1-indexed lines):
    //   1: @AUTH-001
    //   2: Feature: Auth
    //   3: (blank)
    //   4: Scenario: A
    //   5:   Given step a
    //   6: (blank)
    //   7: Scenario: B
    //   8:   Given step b
    let body =
        "@AUTH-001\nFeature: Auth\n\nScenario: A\n  Given step a\n\nScenario: B\n  Given step b\n";
    write_file(tmp.path(), "spec/features/auth.feature", body);

    // @step And spec/work-units.json contains AUTH-001 with title 'Login' and status 'implementing'
    write_work_units(
        tmp.path(),
        &work_units_with(&[("AUTH-001", "Login", "implementing")]),
    );

    // @step When I dispatch show-feature with feature='auth' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "auth", "format": "text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the data field contains the substring '\nWork Units:\n'
    assert!(
        result.data.contains("\nWork Units:\n"),
        "missing 'Work Units:' header; got:\n{}",
        result.data
    );

    // @step And the data field contains the exact line '  AUTH-001 (feature-level) - Login'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  AUTH-001 (feature-level) - Login"),
        "missing AUTH-001 header line; got:\n{}",
        result.data
    );

    // @step And the data field contains the exact line '    auth.feature:4 - A'
    assert!(
        result.data.lines().any(|l| l == "    auth.feature:4 - A"),
        "missing scenario A line; got:\n{}",
        result.data
    );

    // @step And the data field contains the exact line '    auth.feature:7 - B'
    assert!(
        result.data.lines().any(|l| l == "    auth.feature:7 - B"),
        "missing scenario B line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_level_wu_tag_overrides_feature_level_for_tagged_scenario() {
    // Scenario: Scenario-level WU tag overrides feature-level for that scenario and produces a scenario-level entry

    // @step Given a temp project root contains spec/features/mixed.feature tagged '@AUTH-001' at the feature level with scenario 'X' additionally tagged '@AUTH-002' and scenario 'Y' having no scenario-level tag
    let tmp = TempDir::new().expect("tempdir");
    let body = "@AUTH-001\nFeature: Mixed\n\n@AUTH-002\nScenario: X\n  Given x\n\nScenario: Y\n  Given y\n";
    write_file(tmp.path(), "spec/features/mixed.feature", body);

    // @step And spec/work-units.json contains AUTH-001 with title 'Login' and status 'done' and AUTH-002 with title 'Logout' and status 'implementing'
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Logout", "implementing"),
        ]),
    );

    // @step When I dispatch show-feature with feature='mixed' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "mixed", "format": "text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the AUTH-001 block in the data field has level 'feature-level' and lists only scenario 'Y'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  AUTH-001 (feature-level) - Login"),
        "missing AUTH-001 feature-level header; got:\n{}",
        result.data
    );
    // AUTH-001 block must list Y
    let auth001_block_start = result
        .data
        .find("  AUTH-001 (feature-level) - Login")
        .expect("AUTH-001 header found");
    let after_auth001 = &result.data[auth001_block_start..];
    // The AUTH-001 block continues until the next '  AUTH-' header or end of string.
    let block_end = after_auth001[1..]
        .find("\n  AUTH-")
        .map(|i| i + 1)
        .unwrap_or(after_auth001.len());
    let auth001_block = &after_auth001[..block_end];
    assert!(
        auth001_block.lines().any(|l| l.contains(" - Y")),
        "AUTH-001 block must list scenario Y; got:\n{auth001_block}"
    );
    assert!(
        !auth001_block.lines().any(|l| l.contains(" - X")),
        "AUTH-001 block must NOT list scenario X (X has its own WU tag); got:\n{auth001_block}"
    );

    // @step And the AUTH-002 block in the data field has level 'scenario-level' and lists only scenario 'X'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  AUTH-002 (scenario-level) - Logout"),
        "missing AUTH-002 scenario-level header; got:\n{}",
        result.data
    );
    let auth002_block_start = result
        .data
        .find("  AUTH-002 (scenario-level) - Logout")
        .expect("AUTH-002 header found");
    let auth002_block = &result.data[auth002_block_start..];
    assert!(
        auth002_block.lines().any(|l| l.contains(" - X")),
        "AUTH-002 block must list scenario X; got:\n{auth002_block}"
    );
    assert!(
        !auth002_block.lines().any(|l| l.contains(" - Y")),
        "AUTH-002 block must NOT list scenario Y; got:\n{auth002_block}"
    );
}

#[test]
fn missing_work_units_json_yields_unknown_title_and_status() {
    // Scenario: Missing work-units json yields Unknown title and unknown status enrichment

    // @step Given a temp project root contains spec/features/orphan.feature tagged '@AUTH-001' at the feature level with one scenario 'A'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@AUTH-001\nFeature: Orphan\n\nScenario: A\n  Given x\n";
    write_file(tmp.path(), "spec/features/orphan.feature", body);

    // @step And spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-feature with feature='orphan' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "orphan", "format": "text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the data field contains the exact line '  AUTH-001 (feature-level) - Unknown'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  AUTH-001 (feature-level) - Unknown"),
        "missing unknown-enrichment line; got:\n{}",
        result.data
    );
}

#[test]
fn json_format_emits_two_space_indent_with_feature_and_work_units() {
    // Scenario: JSON format emits a 2-space-indented object with feature AST and workUnits array

    // @step Given a temp project root contains spec/features/login.feature with one scenario 'A' and no work-unit tags
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: Login\n\nScenario: A\n  Given x\n";
    write_file(tmp.path(), "spec/features/login.feature", body);

    // @step When I dispatch show-feature with feature='login' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "login", "format": "json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the data field parses as JSON whose root object has a 'feature' field containing 'name' and 'children'
    let data: Value = serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}\n{}", result.data));
    assert!(
        data["feature"].is_object(),
        "missing 'feature' object; got:\n{}",
        result.data
    );
    assert!(
        data["feature"]["name"].is_string(),
        "feature.name missing or not string; got:\n{}",
        result.data
    );
    assert!(
        data["feature"]["children"].is_array(),
        "feature.children missing or not array; got:\n{}",
        result.data
    );

    // @step And the data field parses as JSON whose root object has a 'workUnits' array (empty when no tags present)
    let wus = data["workUnits"]
        .as_array()
        .expect("workUnits must be array");
    assert!(wus.is_empty(), "expected empty workUnits; got: {wus:?}");

    // @step And the data field uses 2-space indentation
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"feature\"")),
        "expected 2-space-indented 'feature' field; got:\n{}",
        result.data
    );
}

#[test]
fn output_path_writes_rendered_content_to_disk_and_data_still_echoes_it() {
    // Scenario: Output path writes rendered content to disk and content field still echoes it

    // @step Given a temp project root contains spec/features/login.feature with valid gherkin and no work-unit tags
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/login.feature",
        LOGIN_FEATURE_NO_TAGS,
    );

    // @step When I dispatch show-feature with feature='login', format='text', and output='out/snapshot.txt'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "login",
            "format": "text",
            "output": "out/snapshot.txt",
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the file <project_root>/out/snapshot.txt exists with the same bytes as the data field
    let written =
        fs::read_to_string(tmp.path().join("out/snapshot.txt")).expect("output file written");
    // The dispatcher emits the same rendered content into `data` so callers
    // can echo it as well.
    assert_eq!(
        written, result.data,
        "on-disk output must equal the data field;\nwritten:\n{written}\n\ndata:\n{}",
        result.data
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under fspec-core for reuse by other commands

    // @step Given the rust/fspec-core crate is built
    // (precondition: this test only runs when the crate builds)

    // @step When I inspect rust/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the helper io::feature_glob::glob_feature_files is publicly accessible from the crate root
    let glob_src = fs::read_to_string(crate_src.join("io/feature_glob.rs"))
        .expect("io/feature_glob.rs readable");
    assert!(
        glob_src.contains("pub fn glob_feature_files"),
        "io/feature_glob.rs must declare `pub fn glob_feature_files`; got:\n{glob_src}"
    );

    // @step And commands/show_feature.rs no longer returns FspecCoreError::NotYetPorted
    let show_src = fs::read_to_string(crate_src.join("commands/show_feature.rs"))
        .expect("commands/show_feature.rs readable");
    assert!(
        !show_src.contains("FspecCoreError::NotYetPorted"),
        "commands/show_feature.rs must no longer be a NotYetPorted stub; got:\n{show_src}"
    );
}
