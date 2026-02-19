//! Shared test utilities for codelet-git integration tests
//!
//! Provides common fixtures to avoid code duplication across test files.
//!
//! Note: Functions may appear unused in individual test files since each test
//! file compiles as a separate crate. This is expected behavior.

#![allow(dead_code)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Create a basic test git repository with initial commit
///
/// Creates a repo with README.md and src/main.rs - suitable for most tests.
pub fn setup_test_repo() -> TempDir {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
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

    // Create initial file and commit
    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("Failed to write README");

    // Create src directory with a file (needed by session_result tests)
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("Failed to write main.rs");

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

/// Create a test git repository with multiple source files
///
/// Creates a repo with src/main.rs, src/config.rs, src/old.rs, README.md.
/// Suitable for tests that need to modify, add, or delete files.
pub fn setup_test_repo_with_files() -> TempDir {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
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

    // Create src directory
    fs::create_dir_all(repo_path.join("src")).expect("Failed to create src dir");

    // Create initial files
    fs::write(repo_path.join("src/main.rs"), "fn main() {}\n").expect("Failed to write main.rs");
    fs::write(repo_path.join("src/config.rs"), "// Config\n").expect("Failed to write config.rs");
    fs::write(repo_path.join("src/old.rs"), "// Old file\n").expect("Failed to write old.rs");
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

/// Create a minimal test git repository using gix (no shell commands)
///
/// Suitable for tests that need a bare minimum repo without commits.
pub fn setup_test_repo_gix() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repo using gix
    let _repo = gix::init(repo_path).expect("Failed to init repo");

    // Create initial file
    let file_path = repo_path.join("initial.txt");
    fs::write(&file_path, "initial content").expect("Failed to write initial file");

    temp_dir
}
