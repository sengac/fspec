#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-tag-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-tag`
// (RPC-222). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: Until the supervisor wires `commands::delete_tag::run`
// into the dispatcher (replacing the NotYetPorted stub), these tests
// will fail with `result.success == false` and `error` containing
// "not yet ported". That is the expected red state at the end of
// Phase B.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "delete-tag".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_tags(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn write_feature(project_root: &Path, rel_path: &str, body: &str) {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir feature parent");
    }
    fs::write(&full, body).expect("write feature file");
}

fn read_tags(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/tags.json"))
        .expect("read tags.json after dispatch");
    serde_json::from_str(&raw).expect("tags.json on disk is valid JSON")
}

fn category_has_tag(data: &Value, tag: &str) -> bool {
    data["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .any(|c| {
            c.get("tags")
                .and_then(|t| t.as_array())
                .map(|arr| arr.iter().any(|t| t["name"].as_str() == Some(tag)))
                .unwrap_or(false)
        })
}

const CANONICAL_DEPRECATED_STATUS: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Status Tags", "description": "s", "required": false, "tags": [
      {"name": "@deprecated", "description": "Deprecated features"}
    ]}
  ]
}"#;

const CANONICAL_CRITICAL_PHASE: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]},
    {"name": "Status Tags", "description": "s", "required": false, "tags": []}
  ]
}"#;

const CANONICAL_CRITICAL_STATUS: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Status Tags", "description": "s", "required": false, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]}
  ]
}"#;

// ---------- scenarios ----------

#[test]
fn deletes_tag_and_regenerates_tags_md_when_no_feature_files_reference_it() {
    // Scenario: Deletes a tag and regenerates TAGS.md when no feature files reference it

    // @step Given spec/tags.json contains a tag '@deprecated' under Status Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_DEPRECATED_STATUS);

    // @step And no feature files in the tempdir reference '@deprecated'
    // (No feature files written — directory absent is equivalent to "no matches".)
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch delete-tag with tag '@deprecated' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@deprecated"})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true; got error={:?}",
        result.error
    );

    // @step And the dispatcher output contains the substring 'Successfully deleted tag @deprecated from registry'
    assert!(
        result
            .data
            .contains("Successfully deleted tag @deprecated from registry"),
        "missing canonical success substring; got:\n{}",
        result.data
    );

    // @step And spec/tags.json on disk no longer contains a tag named '@deprecated' in any category
    let on_disk = read_tags(tmp.path());
    assert!(
        !category_has_tag(&on_disk, "@deprecated"),
        "@deprecated must be removed from all categories"
    );

    // @step And spec/TAGS.md exists in the project root after the call
    assert!(
        tmp.path().join("spec/TAGS.md").exists(),
        "spec/TAGS.md must be regenerated on success"
    );
}

#[test]
fn blocks_deletion_when_tag_referenced_and_force_not_set() {
    // Scenario: Blocks deletion when the tag is referenced by feature files and --force is not set

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step And spec/features/auth.feature contains the substring '@critical'
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        "@critical\nFeature: Auth\n  Scenario: ok\n    Given x\n",
    );

    // @step And spec/features/billing.feature contains the substring '@critical'
    write_feature(
        tmp.path(),
        "spec/features/billing.feature",
        "@critical\nFeature: Billing\n  Scenario: ok\n    Given x\n",
    );

    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch delete-tag with tag '@critical' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical"})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    let msg = result.error.as_ref().expect("error msg");

    // @step And the error message contains the substring 'Tag @critical is used in 2 feature file(s):'
    assert!(
        msg.contains("Tag @critical is used in 2 feature file(s):"),
        "missing canonical usage-blocked substring; got: {msg}"
    );

    // @step And the error message contains the substring 'spec/features/auth.feature'
    assert!(
        msg.contains("spec/features/auth.feature"),
        "missing auth.feature in usage list; got: {msg}"
    );

    // @step And the error message contains the substring 'spec/features/billing.feature'
    assert!(
        msg.contains("spec/features/billing.feature"),
        "missing billing.feature in usage list; got: {msg}"
    );

    // @step And the error message contains the substring 'Use --force to delete anyway'
    assert!(
        msg.contains("Use --force to delete anyway"),
        "missing --force tail; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(
        before, after,
        "tags.json MUST be unchanged on blocked deletion"
    );
}

#[test]
fn forces_deletion_with_warning_prefix_when_force_set_and_tag_in_use() {
    // Scenario: Forces deletion with a warning prefix when --force is set and the tag is still in use

    // @step Given spec/tags.json contains a tag '@critical' under Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_PHASE);

    // @step And spec/features/auth.feature contains the substring '@critical'
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        "@critical\nFeature: Auth\n  Scenario: ok\n    Given x\n",
    );

    // @step And spec/features/billing.feature contains the substring '@critical'
    write_feature(
        tmp.path(),
        "spec/features/billing.feature",
        "@critical\nFeature: Billing\n  Scenario: ok\n    Given x\n",
    );

    // @step When I dispatch delete-tag with tag '@critical' and --force
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical", "force": true})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true; got error={:?}",
        result.error
    );

    // @step And the dispatcher output contains the substring 'Warning: Tag @critical is still used in 2 file(s):'
    assert!(
        result
            .data
            .contains("Warning: Tag @critical is still used in 2 file(s):"),
        "missing canonical warning prefix; got:\n{}",
        result.data
    );

    // @step And the dispatcher output contains the substring 'spec/features/auth.feature'
    assert!(
        result.data.contains("spec/features/auth.feature"),
        "missing auth.feature in warning; got:\n{}",
        result.data
    );

    // @step And the dispatcher output contains the substring 'spec/features/billing.feature'
    assert!(
        result.data.contains("spec/features/billing.feature"),
        "missing billing.feature in warning; got:\n{}",
        result.data
    );

    // @step And the dispatcher output contains the substring 'Successfully deleted tag @critical from registry'
    assert!(
        result
            .data
            .contains("Successfully deleted tag @critical from registry"),
        "missing canonical success line; got:\n{}",
        result.data
    );

    // @step And spec/tags.json on disk no longer contains a tag named '@critical' in any category
    let on_disk = read_tags(tmp.path());
    assert!(
        !category_has_tag(&on_disk, "@critical"),
        "@critical must be removed despite --force usage warning"
    );
}

#[test]
fn dry_run_reports_intended_deletion_without_mutating_disk() {
    // Scenario: Dry-run reports the intended deletion without mutating disk

    // @step Given spec/tags.json contains a tag '@critical' under Status Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_STATUS);
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch delete-tag with tag '@critical' and --dry-run
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical", "dryRun": true})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true; got error={:?}",
        result.error
    );

    // @step And the dispatcher output contains the substring 'Would delete tag @critical from category "Status Tags"'
    assert!(
        result
            .data
            .contains("Would delete tag @critical from category \"Status Tags\""),
        "missing canonical dry-run message; got:\n{}",
        result.data
    );

    // @step And the dispatcher output does not contain the substring 'Updated: spec/tags.json'
    assert!(
        !result.data.contains("Updated: spec/tags.json"),
        "dry-run MUST suppress 'Updated:' line; got:\n{}",
        result.data
    );

    // @step And the dispatcher output does not contain the substring 'Regenerated: spec/TAGS.md'
    assert!(
        !result.data.contains("Regenerated: spec/TAGS.md"),
        "dry-run MUST suppress 'Regenerated:' line; got:\n{}",
        result.data
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(before, after, "tags.json MUST be unchanged on dry-run");
}

#[test]
fn rejects_when_tags_json_does_not_exist() {
    // Scenario: Rejects request when spec/tags.json does not exist

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch delete-tag with tag '@deprecated' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@deprecated"})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'spec/tags.json not found'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("spec/tags.json not found"),
        "missing canonical 'not found' substring; got: {msg}"
    );

    // @step And spec/tags.json was not created by the command
    assert!(
        !tmp.path().join("spec/tags.json").exists(),
        "delete-tag MUST NOT auto-create spec/tags.json"
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
    {"name": "Status Tags", "description": "s", "required": false, "tags": []}
  ]
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch delete-tag with tag '@nonexistent' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@nonexistent"})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Tag @nonexistent not found in registry'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Tag @nonexistent not found in registry"),
        "missing canonical tag-not-found substring; got: {msg}"
    );
}

#[test]
fn preserves_aux_fields_and_does_not_bump_last_updated() {
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

    // @step When I dispatch delete-tag with tag '@critical' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical"})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true; got error={:?}",
        result.error
    );

    let on_disk = read_tags(tmp.path());

    // @step And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    assert_eq!(
        on_disk["combinationExamples"][0]["title"].as_str(),
        Some("demo")
    );
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
        "statistics.lastUpdated MUST NOT be bumped by delete-tag; got: {last_updated}"
    );
}

#[test]
fn escalates_malformed_tags_json_as_structured_parse_error() {
    // Scenario: Escalates malformed tags.json as a structured parse error

    // @step Given spec/tags.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), "{ not valid json");
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch delete-tag with tag '@critical' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical"})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Failed to parse tags.json'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Failed to parse tags.json"),
        "missing canonical parse-error substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(
        before, after,
        "tags.json MUST be untouched on parse failure"
    );
}

#[test]
fn dry_run_suppresses_updated_and_regenerated_lines() {
    // Scenario: Suppresses 'Updated:' and 'Regenerated:' lines when dry-run succeeds

    // @step Given spec/tags.json contains a tag '@critical' under Status Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_CRITICAL_STATUS);

    // @step When I dispatch delete-tag with tag '@critical' and --dry-run
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@critical", "dryRun": true})));

    // @step Then the dispatcher output contains the substring 'Would delete tag @critical from category "Status Tags"'
    assert!(
        result
            .data
            .contains("Would delete tag @critical from category \"Status Tags\""),
        "missing canonical dry-run message; got:\n{}",
        result.data
    );

    // @step And the dispatcher output does not contain the substring 'Updated: spec/tags.json'
    assert!(
        !result.data.contains("Updated: spec/tags.json"),
        "dry-run MUST suppress 'Updated:' line; got:\n{}",
        result.data
    );

    // @step And the dispatcher output does not contain the substring 'Regenerated: spec/TAGS.md'
    assert!(
        !result.data.contains("Regenerated: spec/TAGS.md"),
        "dry-run MUST suppress 'Regenerated:' line; got:\n{}",
        result.data
    );
}

#[test]
fn renders_multi_line_success_block_on_non_dry_run_delete() {
    // Scenario: Renders multi-line success block on a non-dry-run delete

    // @step Given spec/tags.json contains a tag '@deprecated' under Status Tags
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), CANONICAL_DEPRECATED_STATUS);

    // @step When I dispatch delete-tag with tag '@deprecated' and no flags
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@deprecated"})));

    // @step Then the dispatcher output contains the substring '✓ Successfully deleted tag @deprecated from registry'
    assert!(
        result
            .data
            .contains("✓ Successfully deleted tag @deprecated from registry"),
        "missing canonical success line; got:\n{}",
        result.data
    );

    // @step And the dispatcher output contains the substring 'Updated: spec/tags.json'
    assert!(
        result.data.contains("Updated: spec/tags.json"),
        "missing Updated line; got:\n{}",
        result.data
    );

    // @step And the dispatcher output contains the substring 'Regenerated: spec/TAGS.md'
    assert!(
        result.data.contains("Regenerated: spec/TAGS.md"),
        "missing Regenerated line; got:\n{}",
        result.data
    );
}
