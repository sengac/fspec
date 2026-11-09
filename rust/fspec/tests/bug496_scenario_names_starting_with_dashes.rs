//! BUG-496 — link-coverage accepts a scenario name that starts with dashes.
//!
//! Feature: spec/features/link-coverage-accepts-a-scenario-name-that-starts-with-dashes.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

mod common;

use common::fspec_bin;

const SCENARIO: &str = "--all removes everything";

const FEATURE: &str = "Feature: Reset\n\n  Scenario: --all removes everything\n    Given a mapped scenario\n    When it is reset\n    Then it has no mappings\n";

/// The @step comments are on lines 2 to 4.
const TEST_FILE: &str = "fn reset() {\n    // @step Given a mapped scenario\n    // @step When it is reset\n    // @step Then it has no mappings\n}\n";

fn project(mappings: Value) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    let write = |rel: &str, body: &str| {
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    };
    write("spec/features/reset.feature", FEATURE);
    write("tests/reset_test.rs", TEST_FILE);
    write(
        "spec/features/reset.feature.coverage",
        &json!({
            "scenarios": [{"name": SCENARIO, "testMappings": mappings}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
                "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
        })
        .to_string(),
    );
    root
}

fn fspec(root: &Path, args: &[&str]) -> (bool, String) {
    let output = Command::new(fspec_bin())
        .args(args)
        .current_dir(root)
        .output()
        .expect("spawn fspec");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

fn rows(root: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(root.join("spec/features/reset.feature.coverage")).expect("read");
    let coverage: Value = serde_json::from_str(&raw).expect("json");
    coverage["scenarios"][0]["testMappings"]
        .as_array()
        .cloned()
        .expect("testMappings")
}

#[test]
fn linking_a_scenario_whose_name_starts_with_two_dashes() {
    // @step Given a feature with scenario "--all removes everything" and a test file holding its @step comments
    let root = project(json!([]));
    // @step When I run fspec link-coverage with --scenario followed by "--all removes everything" as a separate argument
    let (ok, text) = fspec(
        root.path(),
        &[
            "link-coverage",
            "reset",
            "--scenario",
            SCENARIO,
            "--test-file",
            "tests/reset_test.rs",
            "--test-lines",
            "2-4",
        ],
    );
    // @step Then the command succeeds
    assert!(ok, "{text}");
    // @step And the scenario's coverage maps the test lines
    let rows = rows(root.path());
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["lines"], "2-4", "{rows:?}");
}

#[test]
fn unlinking_a_scenario_whose_name_starts_with_two_dashes() {
    // @step Given scenario "--all removes everything" has a test mapping
    let root = project(json!([
        {"file": "tests/reset_test.rs", "lines": "2-4", "implMappings": []}
    ]));
    // @step When I run fspec unlink-coverage with --scenario followed by "--all removes everything" as a separate argument and --all
    let (ok, text) = fspec(
        root.path(),
        &["unlink-coverage", "reset", "--scenario", SCENARIO, "--all"],
    );
    // @step Then the command succeeds
    assert!(ok, "{text}");
    // @step And the scenario has no test mappings
    assert!(rows(root.path()).is_empty());
}
