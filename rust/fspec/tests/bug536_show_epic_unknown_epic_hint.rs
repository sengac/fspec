//! BUG-536 — show-epic hints at list-epics for an unknown epic.
//!
//! The TypeScript CLI followed "Epic <id> not found" with "Try: fspec
//! list-epics"; the Rust CLI dropped the hint. It is back for the not-found
//! error only.
//!
//! Feature: spec/features/fix-show-epic-command-returning-undefined.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

const HINT: &str = "Try: fspec list-epics";

/// Run `fspec show-epic <id>`; returns (exit code, stdout + stderr).
fn show_epic(cwd: &Path, epic_id: &str) -> (i32, String) {
    let output = Command::new(fspec_bin())
        .args(["show-epic", epic_id])
        .current_dir(cwd)
        .output()
        .expect("spawn fspec show-epic");
    (
        output.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// A project whose spec/epics.json holds one epic, user-management.
fn project_with_epics() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec")).expect("mkdir spec");
    fs::write(
        root.path().join("spec/epics.json"),
        r#"{"epics": {"user-management": {"id": "user-management", "title": "User Management Features", "createdAt": "2026-11-06T00:00:00.000Z"}}}"#,
    )
    .expect("write epics.json");
    root
}

#[test]
fn show_error_when_epic_does_not_exist() {
    // @step Given I have a project with epics configured
    let root = project_with_epics();

    // @step And the epic "invalid-epic" does not exist
    // (only user-management is registered)

    // @step When I run "fspec show-epic invalid-epic"
    let (code, output) = show_epic(root.path(), "invalid-epic");

    // @step Then the command should exit with code 1
    assert_eq!(code, 1, "{output}");

    // @step And the output should contain "Epic invalid-epic not found"
    assert!(output.contains("Epic invalid-epic not found"), "{output}");

    // @step And the output should suggest running "fspec list-epics"
    assert!(output.contains("fspec list-epics"), "{output}");
}

#[test]
fn show_helpful_suggestion_in_error_message() {
    // @step Given I have a project with epics configured
    let root = project_with_epics();

    // @step And the epic "foundation-document-redesign" does not exist
    // (only user-management is registered)

    // @step When I run "fspec show-epic foundation-document-redesign"
    let (code, output) = show_epic(root.path(), "foundation-document-redesign");

    // @step Then the command should exit with code 1
    assert_eq!(code, 1, "{output}");

    // @step And the output should contain "Try: fspec list-epics"
    let not_found = "Epic foundation-document-redesign not found\n";
    assert!(
        output.contains(&format!("{not_found}{HINT}\n")),
        "the hint must follow the not-found error:\n{output}"
    );
}

#[test]
fn show_epic_details_when_epic_exists() {
    // @step Given I have a project with epics configured
    let root = project_with_epics();

    // @step And an epic "user-management" exists with title "User Management Features"
    // (written by project_with_epics)

    // @step When I run "fspec show-epic user-management"
    let (code, output) = show_epic(root.path(), "user-management");

    // @step Then the command should exit with code 0
    assert_eq!(code, 0, "{output}");

    // @step And the output should contain "Epic: user-management"
    assert!(output.contains("Epic: user-management"), "{output}");

    // @step And the output should contain "Title: User Management Features"
    assert!(
        output.contains("Title: User Management Features"),
        "{output}"
    );

    // @step And the output should not contain "undefined"
    assert!(!output.contains("undefined"), "{output}");
}

#[test]
fn no_list_epics_hint_when_epics_json_is_malformed() {
    // @step Given spec/epics.json is not valid JSON
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec")).expect("mkdir spec");
    fs::write(root.path().join("spec/epics.json"), "{ not json").expect("write epics.json");

    // @step When I run "fspec show-epic auth"
    let (code, output) = show_epic(root.path(), "auth");

    // @step Then the command should exit with code 1
    assert_eq!(code, 1, "{output}");

    // @step And the output should not contain "Try: fspec list-epics"
    assert!(!output.contains(HINT), "{output}");
}
