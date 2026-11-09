//! WT-015: a broken state-root pointer fails every fspec CLI command,
//! readers included.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! Product owner, 2026-11-08: a `.fspec/state-root` pointer that names a
//! folder that does not exist makes every command fail with an error naming
//! the pointer file. The CLI bridges resolve the project root through
//! `common::spec_project_root`, which now resolves the state root as well,
//! so a reader such as show-feature or list-feature-tags never prints an
//! empty result for a copy whose shared state cannot be found.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::process::Command;

use serde_json::json;

mod common;

use common::fspec_bin;

/// Scenario: A broken pointer fails every reader command too, naming the pointer file
#[test]
fn a_broken_pointer_fails_every_reader_command_too_naming_the_pointer_file() {
    // @step Given a plain copy whose state-root pointer names a folder that does not exist
    let dir = tempfile::tempdir().expect("tempdir");
    let base = dir.path().canonicalize().expect("canonical tempdir");
    let copy = base.join("copy");
    let write = |rel: &str, body: &str| {
        let path = copy.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    };
    write(".gitignore", ".fspec/\n");
    write(
        "spec/work-units.json",
        &json!({
            "meta": {"version": "0.7.1", "lastUpdated": "2026-11-08T00:00:00.000Z"},
            "workUnits": {},
            "states": {"backlog": [], "specifying": [], "testing": [],
                       "implementing": [], "validating": [], "done": [], "blocked": []}
        })
        .to_string(),
    );
    write(
        "spec/features/login.feature",
        "@WT-015\nFeature: Login\n\n  Scenario: Sign in\n    Given a user\n",
    );
    write(
        ".fspec/state-root",
        &json!({"stateRoot": base.join("missing"), "sessionId": "s1"}).to_string(),
    );
    let pointer = copy.join(".fspec").join("state-root").display().to_string();

    // @step When list-work-units, show-feature, list-feature-tags, context or any other command runs in the copy
    let commands: [&[&str]; 5] = [
        &["list-work-units"],
        &["show-work-unit", "STORY-001"],
        &["show-feature", "login"],
        &["list-feature-tags", "spec/features/login.feature"],
        &["create-story", "STORY", "A story"],
    ];
    for args in commands {
        let output = Command::new(fspec_bin())
            .args(args)
            .current_dir(&copy)
            .output()
            .expect("spawn fspec");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // @step Then each fails naming the copy's .fspec/state-root
        assert!(
            !output.status.success(),
            "{args:?} succeeded with a broken pointer:\n{stdout}"
        );
        assert!(stderr.contains(&pointer), "{args:?}: {stderr}");

        // @step And none prints an empty board or an empty result
        assert!(stdout.trim().is_empty(), "{args:?} printed:\n{stdout}");
    }
}
