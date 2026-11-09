//! BUG-521 — fspec commands accept values that start with dashes.
//!
//! Feature: spec/features/fspec-commands-accept-values-that-start-with-dashes.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::json;

mod common;

use common::fspec_bin;

fn fspec(cwd: &Path, args: &[&str]) -> (bool, String) {
    let output = Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn fspec");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

/// A project whose board holds bug BUG-1 in specifying and prefix BUG.
fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    let spec = root.path().join("spec");
    fs::create_dir_all(spec.join("features")).expect("mkdir");
    let mut states = serde_json::Map::new();
    for state in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let ids = if state == "specifying" {
            json!(["BUG-1"])
        } else {
            json!([])
        };
        states.insert(state.to_string(), ids);
    }
    let board = json!({
        "version": "0.7.1",
        "prefixes": {"BUG": {"prefix": "BUG", "description": "Bugs"}},
        "workUnits": {"BUG-1": {"id": "BUG-1", "title": "t", "type": "bug",
            "status": "specifying", "createdAt": "2026-06-01T00:00:00.000Z",
            "updatedAt": "2026-06-01T00:00:00.000Z"}},
        "states": states,
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board).expect("json"),
    )
    .expect("write board");
    root
}

#[test]
fn work_unit_texts_may_start_with_dashes() {
    // @step Given a project with a bug work unit
    let root = project();
    // @step When I add a rule, an example, a question, an architecture note and an answer that each start with "--"
    let runs = [
        vec!["add-rule", "BUG-1", "--rule starts with dashes"],
        vec!["add-example", "BUG-1", "--example starts with dashes"],
        vec!["add-question", "BUG-1", "--question starts with dashes"],
        vec![
            "add-architecture-note",
            "BUG-1",
            "--note starts with dashes",
        ],
        vec![
            "answer-question",
            "BUG-1",
            "0",
            "--answer",
            "--answer starts with dashes",
        ],
    ];
    // @step Then every command succeeds
    for args in &runs {
        let (ok, text) = fspec(root.path(), args);
        assert!(ok, "{args:?}: {text}");
    }
    // @step And the work unit shows each text verbatim
    let board = fs::read_to_string(root.path().join("spec/work-units.json")).expect("read");
    for text in [
        "--rule starts with dashes",
        "--example starts with dashes",
        "--question starts with dashes",
        "--note starts with dashes",
        "--answer starts with dashes",
    ] {
        assert!(board.contains(text), "{text} missing from {board}");
    }
}

#[test]
fn step_text_and_titles_may_start_with_dashes() {
    // @step Given a feature file with scenario "S" and step "Given a"
    let root = project();
    let feature = root.path().join("spec/features/x.feature");
    fs::write(&feature, "Feature: X\n\n  Scenario: S\n    Given a\n").expect("write");
    // @step When I add a step, update a step's text and rename the work unit with texts that start with "--"
    let runs = [
        vec![
            "add-step",
            "spec/features/x.feature",
            "S",
            "and",
            "--added step",
        ],
        vec![
            "update-step",
            "x",
            "S",
            "Given a",
            "--text",
            "--updated step",
        ],
        vec!["update-work-unit", "BUG-1", "--title", "--work unit title"],
    ];
    // @step Then every command succeeds
    for args in &runs {
        let (ok, text) = fspec(root.path(), args);
        assert!(ok, "{args:?}: {text}");
    }
    // @step And the feature file and the board hold each text verbatim
    let content = fs::read_to_string(&feature).expect("read");
    assert!(content.contains("Given --updated step"), "{content}");
    assert!(content.contains("And --added step"), "{content}");
    let board = fs::read_to_string(root.path().join("spec/work-units.json")).expect("read");
    assert!(board.contains("--work unit title"), "{board}");
}

#[test]
fn flags_and_help_keep_working() {
    // @step Given a project with a bug work unit
    let root = project();
    // @step When I run a command with a flag that takes no value and run another with --help
    fs::write(
        root.path().join("spec/features/x.feature"),
        "Feature: X\n\n  Scenario: S\n    Given a\n",
    )
    .expect("write");
    let (ok, text) = fspec(root.path(), &["generate-coverage", "--dry-run"]);
    let (help_ok, help) = fspec(root.path(), &["add-rule", "--help"]);
    // @step Then the flag is applied and the help is shown
    assert!(ok, "{text}");
    assert!(
        !root
            .path()
            .join("spec/features/x.feature.coverage")
            .exists(),
        "--dry-run was not applied: {text}"
    );
    assert!(help_ok && help.contains("ADD-RULE"), "{help}");
}
