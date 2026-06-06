// Feature: spec/features/list-feature-tags-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-feature-tags`
// (RPC-244). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: list-feature-tags is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical
// "not yet ported" error string instead of the expected
// `{success, tags, message?, categorizedTags?, error?}` payload / text rendering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-feature-tags".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    let parent = abs.parent().expect("parent dir");
    fs::create_dir_all(parent).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn write_tags_json(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn scenario_returns_error_when_feature_file_does_not_exist() {
    // Scenario: Returns error when the requested feature file does not exist

    // @step Given an empty project root directory containing no spec/features/missing.feature
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch list-feature-tags with file='spec/features/missing.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/missing.feature", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected dispatcher success=true, got {result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=false
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "expected success=false in payload, got {}",
        result.data
    );

    // @step Then the parsed JSON has tags array of length 0
    assert_eq!(
        data["tags"].as_array().map(Vec::len),
        Some(0),
        "expected empty tags array, got {}",
        result.data
    );

    // @step Then the parsed JSON has error field equal to 'File not found: spec/features/missing.feature'
    assert_eq!(
        data["error"].as_str(),
        Some("File not found: spec/features/missing.feature"),
        "expected canonical 'File not found' error, got {}",
        result.data
    );
}

#[test]
fn scenario_returns_feature_level_tags_in_declaration_order() {
    // Scenario: Returns feature-level tags in declaration order when feature has tags

    // @step Given spec/features/user-auth.feature exists with feature-level tags '@critical @auth @wip' on a single line before 'Feature: User Authentication'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical @auth @wip\nFeature: User Authentication\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/user-auth.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/user-auth.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/user-auth.feature", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags=['@critical','@auth','@wip'] in that exact order
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 3, "expected 3 tags, got {tags:?}");
    assert_eq!(tags[0].as_str(), Some("@critical"));
    assert_eq!(tags[1].as_str(), Some("@auth"));
    assert_eq!(tags[2].as_str(), Some("@wip"));

    // @step Then the parsed JSON does NOT contain a top-level 'message' field
    assert!(
        data.get("message").is_none(),
        "populated tags must NOT include message; got {}",
        result.data
    );

    // @step Then the parsed JSON does NOT contain a top-level 'error' field
    assert!(
        data.get("error").is_none(),
        "happy path must NOT include error; got {}",
        result.data
    );
}

#[test]
fn scenario_returns_empty_tags_with_sentinel_message_when_no_tags() {
    // Scenario: Returns empty tags with sentinel message when feature has no tags

    // @step Given spec/features/no-tags.feature exists containing 'Feature: No Tags' with no tag lines anywhere in the file
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: No Tags\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/no-tags.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/no-tags.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/no-tags.feature", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags array of length 0
    assert_eq!(
        data["tags"].as_array().map(Vec::len),
        Some(0),
        "expected empty tags array, got {}",
        result.data
    );

    // @step Then the parsed JSON has message field equal to 'No tags found on this feature'
    assert_eq!(
        data["message"].as_str(),
        Some("No tags found on this feature"),
        "expected canonical sentinel message, got {}",
        result.data
    );
}

#[test]
fn scenario_excludes_scenario_level_tags_from_returned_tags() {
    // Scenario: Excludes scenario-level tags from the returned tag list

    // @step Given spec/features/mixed-tags.feature exists with feature-level tag '@critical' and a scenario tagged '@smoke' beneath the Feature header
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Mixed Tags\n\n  @smoke\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/mixed-tags.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/mixed-tags.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/mixed-tags.feature", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags=['@critical'] containing exactly one entry
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1, "expected exactly one feature-level tag, got {tags:?}");
    assert_eq!(tags[0].as_str(), Some("@critical"));

    // @step Then the tags array does NOT contain '@smoke'
    assert!(
        !tags.iter().any(|t| t.as_str() == Some("@smoke")),
        "scenario-level tag must not appear in feature-level tags; got {tags:?}"
    );
}

#[test]
fn scenario_returns_error_when_file_has_no_feature_header() {
    // Scenario: Returns error when file does not contain a valid Feature header

    // @step Given spec/features/junk.feature exists containing only the bytes 'This is not gherkin at all\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/junk.feature",
        "This is not gherkin at all\n",
    );

    // @step When I dispatch list-feature-tags with file='spec/features/junk.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/junk.feature", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=false
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "expected success=false on missing Feature header, got {}",
        result.data
    );

    // @step Then the parsed JSON has tags array of length 0
    assert_eq!(
        data["tags"].as_array().map(Vec::len),
        Some(0),
        "expected empty tags array, got {}",
        result.data
    );

    // @step Then the parsed JSON has error field equal to 'File does not contain a valid Feature'
    assert_eq!(
        data["error"].as_str(),
        Some("File does not contain a valid Feature"),
        "expected canonical 'no Feature header' error, got {}",
        result.data
    );
}

#[test]
fn scenario_pairs_each_tag_with_its_category_when_show_categories_true() {
    // Scenario: Pairs each tag with its category when showCategories=true and tags are registered

    // @step Given spec/features/critical.feature exists with feature-level tag '@critical'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Critical\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/critical.feature", body);

    // @step Given spec/tags.json registers '@critical' under category 'Priority Tags'
    let registry = r#"{
  "categories": [
    {
      "name": "Priority Tags",
      "description": "Priority",
      "required": false,
      "tags": [ { "name": "@critical", "description": "Critical" } ]
    }
  ]
}"#;
    write_tags_json(tmp.path(), registry);

    // @step When I dispatch list-feature-tags with file='spec/features/critical.feature', showCategories=true, and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/critical.feature",
            "showCategories": true,
            "format": "json",
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags=['@critical']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("@critical"));

    // @step Then the parsed JSON has categorizedTags=[{tag:'@critical',category:'Priority Tags'}]
    let cats = data["categorizedTags"]
        .as_array()
        .expect("categorizedTags array");
    assert_eq!(cats.len(), 1, "expected 1 categorized tag, got {cats:?}");
    assert_eq!(cats[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(cats[0]["category"].as_str(), Some("Priority Tags"));
}

#[test]
fn scenario_maps_unregistered_tags_to_unknown_category() {
    // Scenario: Maps unregistered tags to category 'Unknown' when showCategories=true

    // @step Given spec/features/exotic.feature exists with feature-level tag '@nonexistent'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@nonexistent\nFeature: Exotic\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/exotic.feature", body);

    // @step Given spec/tags.json exists but does NOT register '@nonexistent' in any category
    let registry = r#"{
  "categories": [
    {
      "name": "Priority Tags",
      "description": "Priority",
      "required": false,
      "tags": [ { "name": "@critical", "description": "Critical" } ]
    }
  ]
}"#;
    write_tags_json(tmp.path(), registry);

    // @step When I dispatch list-feature-tags with file='spec/features/exotic.feature', showCategories=true, and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/exotic.feature",
            "showCategories": true,
            "format": "json",
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags=['@nonexistent']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("@nonexistent"));

    // @step Then the categorizedTags array contains exactly one entry with tag='@nonexistent' and category='Unknown'
    let cats = data["categorizedTags"]
        .as_array()
        .expect("categorizedTags array");
    assert_eq!(cats.len(), 1, "expected 1 categorized tag, got {cats:?}");
    assert_eq!(cats[0]["tag"].as_str(), Some("@nonexistent"));
    assert_eq!(cats[0]["category"].as_str(), Some("Unknown"));
}

#[test]
fn scenario_silently_degrades_when_show_categories_true_but_registry_missing() {
    // Scenario: Silently degrades to plain tags when showCategories=true but tags.json is missing

    // @step Given spec/features/simple.feature exists with feature-level tag '@critical'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Simple\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/simple.feature", body);

    // @step Given the project root contains no spec/tags.json
    assert!(!tmp.path().join("spec").join("tags.json").exists());

    // @step When I dispatch list-feature-tags with file='spec/features/simple.feature', showCategories=true, and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/simple.feature",
            "showCategories": true,
            "format": "json",
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);

    // @step Then the parsed JSON has success=true
    assert_eq!(data["success"].as_bool(), Some(true), "{}", result.data);

    // @step Then the parsed JSON has tags=['@critical']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("@critical"));

    // @step Then the parsed JSON does NOT contain a top-level 'categorizedTags' field
    assert!(
        data.get("categorizedTags").is_none(),
        "missing registry must silently omit categorizedTags; got {}",
        result.data
    );

    // @step Then the parsed JSON does NOT contain a top-level 'error' field
    assert!(
        data.get("error").is_none(),
        "missing registry must NOT surface as an error; got {}",
        result.data
    );
}

#[test]
fn scenario_json_format_two_space_indent() {
    // Scenario: JSON format emits two-space indented payload

    // @step Given spec/features/tagged.feature exists with feature-level tag '@critical'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Tagged\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/tagged.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/tagged.feature' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/tagged.feature", "format": "json" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data starts with the exact string "{\n  \"success\": true,\n"
    assert!(
        result.data.starts_with("{\n  \"success\": true,\n"),
        "expected 2-space indented JSON opener; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact substring "\"tags\": [\n    \"@critical\"\n  ]"
    assert!(
        result
            .data
            .contains("\"tags\": [\n    \"@critical\"\n  ]"),
        "expected 2-space indented tags array; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_populated_bullet_list() {
    // Scenario: Text format renders the populated case as a bullet list under a header

    // @step Given spec/features/tagged.feature exists with feature-level tags '@critical' and '@auth'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical @auth\nFeature: Tagged\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/tagged.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/tagged.feature' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/tagged.feature", "format": "text" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Tags on this feature:'
    assert!(
        result.data.lines().any(|l| l == "Tags on this feature:"),
        "missing 'Tags on this feature:' header; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  @critical'
    assert!(
        result.data.lines().any(|l| l == "  @critical"),
        "missing '  @critical' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  @auth'
    assert!(
        result.data.lines().any(|l| l == "  @auth"),
        "missing '  @auth' line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_empty_prints_sentinel_message() {
    // Scenario: Text format prints sentinel message when feature has no tags

    // @step Given spec/features/no-tags.feature exists containing 'Feature: No Tags' with no tag lines
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: No Tags\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/no-tags.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/no-tags.feature' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/no-tags.feature", "format": "text" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No tags found on this feature'
    assert_eq!(
        result.data, "No tags found on this feature",
        "expected exact sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_default_format_is_text() {
    // Scenario: Default format (no format key supplied) is text

    // @step Given spec/features/tagged.feature exists with feature-level tag '@critical'
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Tagged\n\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/tagged.feature", body);

    // @step When I dispatch list-feature-tags with file='spec/features/tagged.feature' and no format key in the args object
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/tagged.feature" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Tags on this feature:'
    assert!(
        result.data.lines().any(|l| l == "Tags on this feature:"),
        "default format must be text with the canonical header; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  @critical'
    assert!(
        result.data.lines().any(|l| l == "  @critical"),
        "default format must render the bullet for '@critical'; got:\n{}",
        result.data
    );
}
