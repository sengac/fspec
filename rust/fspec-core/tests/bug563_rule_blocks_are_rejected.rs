//! BUG-563 — Rule blocks are rejected.
//!
//! A `Rule:` line makes every following scenario a child of a Rule block,
//! which the coverage commands did not track. fspec does not support Rule
//! blocks (coordinator decision, 2026-11-07): validate reports each one with
//! its line, and the coverage commands refuse the feature.
//!
//! Feature: spec/features/rule-blocks-are-rejected.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

/// Line 5 opens a Rule block; the Background and scenario after it become its
/// children (the shape that hid websearch-tool-pause-integration's scenarios).
const FEATURE: &str = concat!(
    "@cli\n",
    "Feature: Pause integration\n",
    "  Adds a pause field.\n",
    "\n",
    "  Rule: pause implies a visible browser\n",
    "\n",
    "    Background: User Story\n",
    "      As a developer\n",
    "\n",
    "    Scenario: Pause shows the browser\n",
    "      Given a paused page\n",
    "      Then the browser is visible\n",
);

const RULE_LINE: &str = "line 5: Rule blocks are not supported";

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec/features")).expect("mkdir features");
    fs::write(root.path().join("spec/features/pause.feature"), FEATURE).expect("write feature");
    fs::write(
        root.path().join("tests.rs"),
        "// @step Given a paused page\n// @step Then the browser is visible\n",
    )
    .expect("write test");
    root
}

fn dispatch(root: &Path, command: &str, args: serde_json::Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn text(result: &DispatchResult) -> String {
    format!(
        "{} {}",
        result.data,
        result.error.clone().unwrap_or_default()
    )
}

#[test]
fn validate_reports_a_rule_block_naming_its_line() {
    // @step Given a feature whose line 5 is "  Rule: pause implies a visible browser" followed by scenarios
    let root = project();
    assert_eq!(
        FEATURE.lines().nth(4),
        Some("  Rule: pause implies a visible browser")
    );

    // @step When fspec validate runs on it
    let result = dispatch(
        root.path(),
        "validate",
        json!({"file": "spec/features/pause.feature"}),
    );
    let out = text(&result);

    // @step Then it reports "line 5: Rule blocks are not supported" with the Rule line text
    assert!(out.contains(RULE_LINE), "{out}");
    assert!(
        out.contains("Rule: pause implies a visible browser"),
        "{out}"
    );

    // @step And the feature is invalid
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("validate data");
    assert_eq!(data["results"][0]["valid"], json!(false), "{data}");
}

#[test]
fn generate_coverage_refuses_a_feature_with_a_rule_block() {
    // @step Given a feature with a Rule block and no coverage file
    let root = project();
    let coverage = root.path().join("spec/features/pause.feature.coverage");

    // @step When fspec generate-coverage runs for all features and for that feature by name
    let all = dispatch(root.path(), "generate-coverage", json!({}));
    let named = dispatch(
        root.path(),
        "generate-coverage",
        json!({"featureName": "pause"}),
    );

    // @step Then both runs fail naming the Rule block line
    for result in [&all, &named] {
        assert!(!result.success, "{result:?}");
        assert!(text(result).contains(RULE_LINE), "{result:?}");
    }

    // @step And no coverage file is written
    assert!(!coverage.exists());
}

#[test]
fn link_audit_and_show_coverage_refuse_a_feature_with_a_rule_block() {
    // @step Given a feature with a Rule block and a coverage file
    let root = project();
    fs::write(
        root.path().join("spec/features/pause.feature.coverage"),
        serde_json::to_string_pretty(&json!({
            "scenarios": [{"name": "Pause shows the browser", "testMappings": []}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
                "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
        }))
        .expect("serialize coverage"),
    )
    .expect("write coverage");

    // @step When fspec link-coverage, fspec audit and fspec show-coverage run on it
    let link = dispatch(
        root.path(),
        "link-coverage",
        json!({"featureName": "pause", "scenario": "Pause shows the browser",
               "testFile": "tests.rs", "testLines": "1-2"}),
    );
    let audit = dispatch(
        root.path(),
        "audit",
        json!({"feature": ["pause"], "area": ["coverage"]}),
    );
    let show = dispatch(
        root.path(),
        "show-coverage",
        json!({"featureName": "pause"}),
    );

    // @step Then each fails naming the Rule block line
    for (name, result) in [("link", &link), ("show", &show)] {
        assert!(!result.success, "{name}: {result:?}");
        assert!(text(result).contains(RULE_LINE), "{name}: {result:?}");
    }
    let report: Value = serde_json::from_str(&audit.data).expect("audit envelope");
    assert_eq!(report["exitCode"], 1, "audit: {audit:?}");
    assert!(
        report["output"]
            .as_str()
            .is_some_and(|output| output.contains(RULE_LINE)),
        "audit: {audit:?}"
    );
}
