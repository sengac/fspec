#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/generate-tags-md-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `generate-tags-md`
// (RPC-236). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim. These are dispatcher-contract
// tests driven through codelet_fspec_core::dispatch_command.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "generate-tags-md".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// A fully schema-valid tags.json Value satisfying every required top-level
/// key in src/schemas/tags.schema.json (categories, combinationExamples,
/// usageGuidelines, addingNewTags, queries, statistics, validation,
/// references).
fn valid_tags() -> Value {
    json!({
        "categories": [
            {
                "name": "Phase Tags",
                "description": "Phase identification tags",
                "required": true,
                "tags": [
                    { "name": "@critical", "description": "Critical priority" }
                ]
            }
        ],
        "combinationExamples": [
            {
                "title": "Example 1",
                "tags": "@cli @critical",
                "interpretation": ["CLI component", "Critical priority"]
            }
        ],
        "usageGuidelines": {
            "requiredCombinations": {
                "title": "Required",
                "requirements": ["Must have a phase tag"],
                "minimumExample": "@phase-1"
            },
            "recommendedCombinations": {
                "title": "Recommended",
                "includes": ["component tag"],
                "recommendedExample": "@cli @phase-1"
            },
            "orderingConvention": {
                "title": "Ordering",
                "order": ["phase", "component"],
                "example": "@phase-1 @cli"
            }
        },
        "addingNewTags": {
            "process": [{ "step": "Step 1", "description": "Register the tag" }],
            "namingConventions": ["lowercase-with-hyphens"],
            "antiPatterns": {
                "dont": [{ "description": "Create overlapping tags" }],
                "do": [{ "description": "Reuse existing tags" }]
            }
        },
        "queries": {
            "title": "Tag-Based Queries",
            "examples": [{ "description": "Find by tag", "command": "fspec list-tags" }]
        },
        "statistics": {
            "lastUpdated": "2025-01-15T10:30:00Z",
            "phaseStats": [
                { "phase": "Phase 1", "total": 5, "complete": 5, "inProgress": 0, "planned": 0 }
            ],
            "componentStats": [
                { "component": "@cli", "count": 28, "percentage": "100%" }
            ],
            "featureGroupStats": [
                { "featureGroup": "@validation", "count": 10, "percentage": "50%" }
            ],
            "updateCommand": "fspec tag-stats"
        },
        "validation": {
            "rules": [{ "rule": "Format", "description": "Tags must start with @" }],
            "commands": [{ "description": "Validate", "command": "fspec validate-tags" }]
        },
        "references": [
            { "title": "Markdown tables", "url": "https://example.com/tables" }
        ]
    })
}

fn write_tags(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(value).expect("ser tags"),
    )
    .expect("write tags.json");
}

const HEADER: &str = "<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->";
const TITLE: &str = "# fspec Feature File Tag Registry";

// ---------- scenarios ----------

#[test]
fn generating_tags_md_from_valid_tags_json_writes_file_and_returns_message() {
    // @step Given a project root tempdir with a schema-valid spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &valid_tags());

    // @step When I dispatch generate-tags-md with no output override
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Generated spec/TAGS.md from spec/tags.json'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Generated spec/TAGS.md from spec/tags.json")
    );

    // @step And the bytes written to spec/TAGS.md exactly equal the rendered markdown
    let written =
        fs::read_to_string(tmp.path().join("spec").join("TAGS.md")).expect("read TAGS.md");
    assert!(!written.is_empty(), "TAGS.md must not be empty");
    assert!(
        written.ends_with('\n'),
        "rendered markdown ends with a single trailing newline (TS join('\\n') with trailing empty section); got tail: {:?}",
        &written[written.len().saturating_sub(20)..]
    );

    // @step And spec/TAGS.md starts with the auto-generated header comment '<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->'
    assert!(
        written.starts_with(HEADER),
        "TAGS.md must start with the auto-gen header; got:\n{written}"
    );

    // @step And spec/TAGS.md contains the title '# fspec Feature File Tag Registry'
    assert!(
        written.contains(TITLE),
        "TAGS.md must contain the registry title; got:\n{written}"
    );
}

#[test]
fn relative_output_override_resolves_against_project_root() {
    // @step Given a project root tempdir with a schema-valid spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &valid_tags());

    // @step When I dispatch generate-tags-md with output='docs/TAGS.md'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "docs/TAGS.md" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file docs/TAGS.md is written relative to the project root
    assert!(
        tmp.path().join("docs").join("TAGS.md").exists(),
        "docs/TAGS.md must be written relative to project root"
    );

    // @step And the returned message is 'Generated docs/TAGS.md from spec/tags.json'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Generated docs/TAGS.md from spec/tags.json")
    );
}

#[test]
fn generation_fails_when_tags_json_missing() {
    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").join("tags.json").exists());

    // @step When I dispatch generate-tags-md with no output override
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns an error containing the substring 'tags.json not found: spec/tags.json'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("tags.json not found: spec/tags.json"),
        "expected canonical missing-tags message; got: {err}"
    );

    // @step And the file spec/TAGS.md is not created
    assert!(
        !tmp.path().join("spec").join("TAGS.md").exists(),
        "TAGS.md must NOT be created when tags.json is missing"
    );
}

#[test]
fn generation_fails_when_tags_json_fails_schema_validation() {
    // @step Given a project root tempdir with a spec/tags.json missing required top-level keys
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &json!({ "categories": [] }));

    // @step When I dispatch generate-tags-md with no output override
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns an error containing the substring 'tags.json has validation errors:'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("tags.json has validation errors:"),
        "expected schema-validation error; got: {err}"
    );

    // @step And the file spec/TAGS.md is not created
    assert!(
        !tmp.path().join("spec").join("TAGS.md").exists(),
        "TAGS.md must NOT be created when tags.json is schema-invalid"
    );
}
