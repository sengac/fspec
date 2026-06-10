#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-tag-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `update-tag`
// (RPC-316). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: Until the supervisor wires `commands::update_tag::run` into
// the dispatcher (replacing the NotYetPorted stub), these tests will fail
// with `result.success == false` and `error` containing "not yet ported".
// That is the expected red state at the end of Phase B.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-tag".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_tags(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn read_tags(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/tags.json"))
        .expect("read tags.json after dispatch");
    serde_json::from_str(&raw).expect("tags.json on disk is valid JSON")
}

fn find_category<'a>(data: &'a Value, name: &str) -> Option<&'a Value> {
    data["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|c| c["name"].as_str() == Some(name))
}

fn category_tag_names(data: &Value, cat: &str) -> Vec<String> {
    find_category(data, cat)
        .expect("category present")
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn tag_description(data: &Value, cat: &str, tag: &str) -> Option<String> {
    find_category(data, cat)?
        .get("tags")?
        .as_array()?
        .iter()
        .find(|t| t["name"].as_str() == Some(tag))?
        .get("description")?
        .as_str()
        .map(String::from)
}

const CANONICAL_CRITICAL_PHASE: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]},
    {"name": "Component Tags", "description": "c", "required": true, "tags": []},
    {"name": "Priority Tags", "description": "pri", "required": false, "tags": []}
  ]
}"#;

// ---------- scenarios ----------

#[test]
fn updates_only_description_when_only_description_provided() {
    // Scenario: Updates only the description when only --description is provided

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step When I dispatch update-tag with tag '@critical' and description 'Critical paths only'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "description": "Critical paths only"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    // @step And the Phase Tags category on disk contains a tag with name '@critical' and description 'Critical paths only'
    let on_disk = read_tags(tmp.path());
    assert_eq!(
        tag_description(&on_disk, "Phase Tags", "@critical").as_deref(),
        Some("Critical paths only")
    );

    // @step And the dispatcher output contains the substring 'Successfully updated @critical'
    assert!(
        result.data.contains("Successfully updated @critical"),
        "missing canonical success substring; got:\n{}", result.data
    );
}

#[test]
fn moves_tag_to_different_category_preserving_description() {
    // Scenario: Moves tag to a different category preserving original description when --description is omitted

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step And Priority Tags exists as an empty category
    // (Already in fixture.)
    assert!(find_category(&read_tags(tmp.path()), "Priority Tags").is_some());

    // @step When I dispatch update-tag with tag '@critical' and category 'Priority Tags'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "category": "Priority Tags"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    let on_disk = read_tags(tmp.path());

    // @step And the Phase Tags category on disk does not contain a tag named '@critical'
    let phase_names = category_tag_names(&on_disk, "Phase Tags");
    assert!(
        !phase_names.iter().any(|n| n == "@critical"),
        "@critical must be removed from Phase Tags; got: {phase_names:?}"
    );

    // @step And the Priority Tags category on disk contains a tag with name '@critical' and description 'Critical features'
    assert_eq!(
        tag_description(&on_disk, "Priority Tags", "@critical").as_deref(),
        Some("Critical features")
    );
}

#[test]
fn moves_tag_to_different_category_and_overrides_description() {
    // Scenario: Moves tag to a different category and overrides description when both --category and --description are provided

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step And Priority Tags exists as an empty category
    assert!(find_category(&read_tags(tmp.path()), "Priority Tags").is_some());

    // @step When I dispatch update-tag with tag '@critical', category 'Priority Tags', and description 'High priority work'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@critical",
            "category": "Priority Tags",
            "description": "High priority work"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    // @step And the Priority Tags category on disk contains a tag with name '@critical' and description 'High priority work'
    let on_disk = read_tags(tmp.path());
    assert_eq!(
        tag_description(&on_disk, "Priority Tags", "@critical").as_deref(),
        Some("High priority work")
    );
}

#[test]
fn sorts_tags_alphabetically_within_target_category_after_move() {
    // Scenario: Sorts tags alphabetically within the target category after cross-category move

    // @step Given spec/tags.json contains Phase Tags with tag '@critical' and Priority Tags with tags '@zed', '@aaa', '@mid' in that insertion order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]},
    {"name": "Priority Tags", "description": "pri", "required": false, "tags": [
      {"name": "@zed", "description": "z"},
      {"name": "@aaa", "description": "a"},
      {"name": "@mid", "description": "m"}
    ]}
  ]
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch update-tag with tag '@critical' and category 'Priority Tags'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "category": "Priority Tags"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    // @step And the Priority Tags category on disk contains tags in the order '@aaa', '@critical', '@mid', '@zed'
    let on_disk = read_tags(tmp.path());
    let names = category_tag_names(&on_disk, "Priority Tags");
    assert_eq!(
        names,
        vec!["@aaa", "@critical", "@mid", "@zed"],
        "Priority Tags must be alphabetised after move; got: {names:?}"
    );
}

#[test]
fn preserves_insertion_order_on_description_only_update() {
    // Scenario: Preserves insertion order when description-only update inside the same category

    // @step Given spec/tags.json contains Phase Tags with tags '@zed', '@aaa', '@mid' in that insertion order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@zed", "description": "z"},
      {"name": "@aaa", "description": "a"},
      {"name": "@mid", "description": "m"}
    ]}
  ]
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch update-tag with tag '@aaa' and description 'New A description'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@aaa", "description": "New A description"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    // @step And the Phase Tags category on disk contains tags in the order '@zed', '@aaa', '@mid'
    let on_disk = read_tags(tmp.path());
    let names = category_tag_names(&on_disk, "Phase Tags");
    assert_eq!(
        names,
        vec!["@zed", "@aaa", "@mid"],
        "Phase Tags MUST preserve insertion order on description-only update; got: {names:?}"
    );
}

#[test]
fn rejects_when_no_category_or_description_provided() {
    // Scenario: Rejects request when neither --category nor --description is provided

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch update-tag with tag '@critical' and no category or description
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'No updates specified. Use --category and/or --description'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("No updates specified. Use --category and/or --description"),
        "missing canonical 'No updates' substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(before, after, "tags.json MUST be unchanged on 'no updates' rejection");
}

#[test]
fn rejects_when_tags_json_does_not_exist() {
    // Scenario: Rejects request when spec/tags.json does not exist

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch update-tag with tag '@critical' and description 'New description'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "description": "New description"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'spec/tags.json not found'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("spec/tags.json not found"),
        "missing canonical 'not found' substring; got: {msg}"
    );

    // @step And spec/tags.json was not created by the command
    assert!(
        !tmp.path().join("spec/tags.json").exists(),
        "update-tag MUST NOT auto-create spec/tags.json"
    );
}

#[test]
fn rejects_when_tag_not_found_in_any_category() {
    // Scenario: Rejects request when the tag is not found in any category

    // @step Given spec/tags.json exists with the canonical empty category set
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Component Tags", "description": "c", "required": true, "tags": []}
  ]
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch update-tag with tag '@nonexistent' and description 'Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@nonexistent", "description": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'Tag @nonexistent not found in registry'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Tag @nonexistent not found in registry"),
        "missing canonical tag-not-found substring; got: {msg}"
    );
}

#[test]
fn rejects_unknown_target_category_with_available_list() {
    // Scenario: Rejects unknown target category with canonical Available categories list

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch update-tag with tag '@critical' and category 'Nonexistent Tags'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "category": "Nonexistent Tags"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'Invalid category: Nonexistent Tags. Available categories: Phase Tags'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Invalid category: Nonexistent Tags. Available categories: Phase Tags"),
        "missing canonical invalid-category substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(before, after, "tags.json MUST be unchanged on invalid-category failure");
}

#[test]
fn treats_category_lookup_as_case_sensitive() {
    // Scenario: Treats category lookup as case-sensitive (lowercase variant does not match)

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step When I dispatch update-tag with tag '@critical' and category 'phase tags'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "category": "phase tags"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'Invalid category: phase tags'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Invalid category: phase tags"),
        "lookup MUST be case-sensitive; got: {msg}"
    );
}

#[test]
fn preserves_auxiliary_fields_and_does_not_bump_last_updated() {
    // Scenario: Preserves auxiliary top-level fields and does NOT bump statistics.lastUpdated

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags plus auxiliary fields combinationExamples, usageGuidelines, references, and statistics.lastUpdated set to '1999-01-01T00:00:00.000Z'
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]}
  ],
  "combinationExamples": [
    {"title": "demo", "tags": "@a @b", "interpretation": ["a", "b"]}
  ],
  "usageGuidelines": {
    "requiredCombinations": {"title": "req", "requirements": ["one"], "minimumExample": "@x"},
    "recommendedCombinations": {"title": "rec", "includes": ["two"], "recommendedExample": "@y"},
    "orderingConvention": {"title": "ord", "order": ["three"], "example": "@z"}
  },
  "references": [
    {"title": "spec", "url": "https://example.com"}
  ],
  "statistics": {
    "lastUpdated": "1999-01-01T00:00:00.000Z",
    "phaseStats": [],
    "componentStats": [],
    "featureGroupStats": [],
    "updateCommand": "fspec tag-stats"
  }
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch update-tag with tag '@critical' and description 'New description'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "description": "New description"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got error={:?}", result.error);

    let on_disk = read_tags(tmp.path());

    // @step And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    assert_eq!(on_disk["combinationExamples"][0]["title"].as_str(), Some("demo"));
    assert_eq!(
        on_disk["usageGuidelines"]["requiredCombinations"]["title"].as_str(),
        Some("req")
    );
    assert_eq!(on_disk["references"][0]["title"].as_str(), Some("spec"));

    // @step And spec/tags.json statistics.lastUpdated on disk still equals '1999-01-01T00:00:00.000Z'
    let last_updated = on_disk["statistics"]["lastUpdated"]
        .as_str()
        .expect("statistics.lastUpdated string");
    assert_eq!(
        last_updated, "1999-01-01T00:00:00.000Z",
        "statistics.lastUpdated MUST NOT be bumped by update-tag; got: {last_updated}"
    );
}

#[test]
fn escalates_malformed_tags_json_as_structured_parse_error() {
    // Scenario: Escalates malformed tags.json as a structured parse error

    // @step Given spec/tags.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), "{ not valid json");
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch update-tag with tag '@critical' and description 'New description'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "description": "New description"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got data={}", result.data);

    // @step And the error message contains the substring 'Failed to parse tags.json'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Failed to parse tags.json"),
        "missing canonical parse-error substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(before, after, "tags.json MUST be untouched on parse failure");
}

#[test]
fn renders_multi_line_success_block() {
    // Scenario: Renders multi-line success block on success

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step When I dispatch update-tag with tag '@critical' and description 'Critical paths only'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@critical", "description": "Critical paths only"}),
    ));

    // @step Then the dispatcher output contains the substring '✓ Successfully updated @critical'
    assert!(
        result.data.contains("✓ Successfully updated @critical"),
        "missing canonical success line; got:\n{}", result.data
    );

    // @step And the dispatcher output contains the substring 'Updated: spec/tags.json'
    assert!(
        result.data.contains("Updated: spec/tags.json"),
        "missing Updated line; got:\n{}", result.data
    );

    // @step And the dispatcher output contains the substring 'Regenerated: spec/TAGS.md'
    assert!(
        result.data.contains("Regenerated: spec/TAGS.md"),
        "missing Regenerated line; got:\n{}", result.data
    );
}
