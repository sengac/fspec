//! Shared test utilities for codelet-rpc integration tests (RPC-362).
//!
//! Mirrors `codelet/git/tests/common/mod.rs::setup_test_repo` so checkpoint
//! transport tests build real git repos without duplicating fixture logic per
//! test. Functions may appear unused per-test (each test file is its own crate).

#![allow(dead_code)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Create a basic test git repository with an initial commit.
pub fn setup_test_repo() -> TempDir {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git user");

    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("Failed to write README");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage files");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create commit");

    tmp_dir
}
