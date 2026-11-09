//! CLI-007 — the generic `create-work-unit` command is gone; work units are
//! created with create-story, create-bug or create-task.
//!
//! Feature: spec/features/replace-generic-create-work-unit-with-type-specific-commands.feature
//!
//! The type-specific commands and their guidance are proved by
//! create-story-rust-port, create-bug-rust-port and create-task-rust-port.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

mod common;

use common::fspec_bin;

#[test]
fn the_removed_create_work_unit_command_fails_as_unknown() {
    // @step Given I have fspec v2.0 installed
    let root = tempfile::tempdir().expect("tempdir");

    // @step When I run fspec with the removed command 'create-work-unit AUTH "Login"'
    let output = Command::new(fspec_bin())
        .args(["create-work-unit", "AUTH", "Login"])
        .current_dir(root.path())
        .output()
        .expect("spawn fspec create-work-unit");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // @step Then the command should fail with the error "error: unknown command 'create-work-unit'"
    assert!(
        stderr
            .lines()
            .any(|line| line == "error: unknown command 'create-work-unit'"),
        "stderr: {stderr}"
    );

    // @step And the exit code should be 1
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");

    // @step And no spec/work-units.json should be written
    assert!(
        !root.path().join("spec/work-units.json").exists(),
        "an unknown command must not create the board"
    );
}
