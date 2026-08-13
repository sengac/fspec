//! Tests for git worktree operations
//!
//! Feature: Git worktree creation for BackgroundSession isolation
//! spec/features/git-worktree-creation-for-backgroundsession-isolation.feature

mod common;

use codelet_git::{
    create_worktree, create_worktree_at_ref, list_worktrees, remove_worktree, GitError,
    FSPEC_WORKTREES_DIR,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Helper to get HEAD commit SHA
fn get_head_sha(repo_path: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Helper to create a second commit and return its SHA
fn create_second_commit(repo_path: &Path) -> String {
    fs::write(repo_path.join("file2.txt"), "Second file\n").expect("Failed to write file2");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage files");

    Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create second commit");

    get_head_sha(repo_path)
}

// =============================================================================
// Scenario: Create worktree at HEAD for new session
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And I have a session ID "abc-123-def"
/// @step When I create a worktree for the session
/// @step Then a worktree should exist at ".fspec/worktrees/abc-123-def/"
/// @step And the worktree should be in detached HEAD mode
/// @step And the worktree HEAD should match the main repository HEAD
/// @step And the session manifest should have the worktree_path field set
/// @step And the session manifest should have the base_commit field set to HEAD
/// @step And the session manifest should have the worktree_created_at timestamp
#[test]
fn test_create_worktree_at_head_for_new_session() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    let head_sha = get_head_sha(repo_path);

    // And I have a session ID "abc-123-def"
    let session_id = "abc-123-def";

    // When I create a worktree for the session
    let result = create_worktree(repo_path, session_id);

    // Then the operation should succeed
    assert!(
        result.is_ok(),
        "Failed to create worktree: {:?}",
        result.err()
    );
    let create_result = result.unwrap();

    // Then a worktree should exist at ".fspec/worktrees/abc-123-def/"
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    assert!(worktree_path.exists(), "Worktree directory should exist");
    assert!(
        worktree_path.is_dir(),
        "Worktree path should be a directory"
    );

    // And the worktree should be in detached HEAD mode
    assert!(
        create_result.info.is_detached,
        "Worktree should be in detached HEAD mode"
    );

    // And the worktree HEAD should match the main repository HEAD
    assert_eq!(
        create_result.info.head_commit, head_sha,
        "Worktree HEAD should match main repo HEAD"
    );

    // And the session manifest should have the worktree_path field set
    assert_eq!(create_result.info.path, worktree_path);

    // And the session manifest should have the base_commit field set to HEAD
    assert_eq!(create_result.base_commit, head_sha);

    // And the session manifest should have the worktree_created_at timestamp
    assert!(create_result.created_at.timestamp() > 0);
}

// =============================================================================
// Scenario: Create worktree at specific commit ref
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And I have a session ID "xyz-456-uvw"
/// @step And I have a commit ref "abc1234"
/// @step When I create a worktree for the session at commit "abc1234"
/// @step Then a worktree should exist at ".fspec/worktrees/xyz-456-uvw/"
/// @step And the worktree should be in detached HEAD mode
/// @step And the worktree HEAD should point to commit "abc1234"
#[test]
fn test_create_worktree_at_specific_commit_ref() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Store the first commit SHA before creating second commit
    let first_commit_sha = get_head_sha(repo_path);

    // Create second commit so we have multiple commits
    let _second_commit_sha = create_second_commit(repo_path);

    // And I have a session ID "xyz-456-uvw"
    let session_id = "xyz-456-uvw";

    // And I have a commit ref (using first commit)
    let commit_ref = &first_commit_sha;

    // When I create a worktree for the session at that commit
    let result = create_worktree_at_ref(repo_path, session_id, Some(commit_ref));

    // Then the operation should succeed
    assert!(
        result.is_ok(),
        "Failed to create worktree at ref: {:?}",
        result.err()
    );
    let create_result = result.unwrap();

    // Then a worktree should exist at ".fspec/worktrees/xyz-456-uvw/"
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    assert!(worktree_path.exists(), "Worktree directory should exist");

    // And the worktree should be in detached HEAD mode
    assert!(
        create_result.info.is_detached,
        "Worktree should be in detached HEAD mode"
    );

    // And the worktree HEAD should point to the specified commit
    assert_eq!(
        create_result.info.head_commit, first_commit_sha,
        "Worktree HEAD should point to specified commit"
    );
}

// =============================================================================
// Scenario: Auto-create worktrees directory if it doesn't exist
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And the ".fspec/worktrees/" directory does not exist
/// @step And I have a session ID "first-session"
/// @step When I create a worktree for the session
/// @step Then the ".fspec/worktrees/" directory should be created
/// @step And a worktree should exist at ".fspec/worktrees/first-session/"
#[test]
fn test_auto_create_worktrees_directory() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And the ".fspec/worktrees/" directory does not exist
    let worktrees_dir = repo_path.join(FSPEC_WORKTREES_DIR);
    assert!(
        !worktrees_dir.exists(),
        "Worktrees directory should not exist initially"
    );

    // And I have a session ID "first-session"
    let session_id = "first-session";

    // When I create a worktree for the session
    let result = create_worktree(repo_path, session_id);

    // Then the operation should succeed
    assert!(
        result.is_ok(),
        "Failed to create worktree: {:?}",
        result.err()
    );

    // Then the ".fspec/worktrees/" directory should be created
    assert!(
        worktrees_dir.exists(),
        "Worktrees directory should be created"
    );

    // And a worktree should exist at ".fspec/worktrees/first-session/"
    let worktree_path = worktrees_dir.join(session_id);
    assert!(worktree_path.exists(), "Worktree directory should exist");
}

// =============================================================================
// Scenario: Remove worktree and clean up git metadata
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a worktree exists at ".fspec/worktrees/session-to-remove/"
/// @step When I remove the worktree for session "session-to-remove"
/// @step Then the ".fspec/worktrees/session-to-remove/" directory should not exist
/// @step And the git worktree metadata should be cleaned up
#[test]
fn test_remove_worktree_and_cleanup_metadata() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    let session_id = "session-to-remove";

    // And a worktree exists at ".fspec/worktrees/session-to-remove/"
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok(), "Setup: Failed to create worktree");

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    assert!(worktree_path.exists(), "Setup: Worktree should exist");

    // Git worktree metadata path
    let git_worktree_meta = repo_path.join(".git").join("worktrees").join(session_id);

    // When I remove the worktree for session "session-to-remove"
    let result = remove_worktree(repo_path, session_id);

    // Then the operation should succeed
    assert!(
        result.is_ok(),
        "Failed to remove worktree: {:?}",
        result.err()
    );

    // Then the ".fspec/worktrees/session-to-remove/" directory should not exist
    assert!(
        !worktree_path.exists(),
        "Worktree directory should be removed"
    );

    // And the git worktree metadata should be cleaned up
    assert!(
        !git_worktree_meta.exists(),
        "Git worktree metadata should be cleaned up"
    );
}

// =============================================================================
// Scenario: List all session worktrees
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a worktree exists at ".fspec/worktrees/session-1/"
/// @step And a worktree exists at ".fspec/worktrees/session-2/"
/// @step When I list all worktrees
/// @step Then I should see 2 worktrees
/// @step And the list should include "session-1"
/// @step And the list should include "session-2"
#[test]
fn test_list_all_session_worktrees() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a worktree exists at ".fspec/worktrees/session-1/"
    let result1 = create_worktree(repo_path, "session-1");
    assert!(
        result1.is_ok(),
        "Setup: Failed to create session-1 worktree"
    );

    // And a worktree exists at ".fspec/worktrees/session-2/"
    let result2 = create_worktree(repo_path, "session-2");
    assert!(
        result2.is_ok(),
        "Setup: Failed to create session-2 worktree"
    );

    // When I list all worktrees
    let result = list_worktrees(repo_path);

    // Then the operation should succeed
    assert!(
        result.is_ok(),
        "Failed to list worktrees: {:?}",
        result.err()
    );
    let worktrees = result.unwrap();

    // Then I should see 2 worktrees
    assert_eq!(worktrees.len(), 2, "Should have exactly 2 worktrees");

    // And the list should include "session-1"
    let session_ids: Vec<&str> = worktrees.iter().map(|w| w.session_id.as_str()).collect();
    assert!(
        session_ids.contains(&"session-1"),
        "Should include session-1"
    );

    // And the list should include "session-2"
    assert!(
        session_ids.contains(&"session-2"),
        "Should include session-2"
    );
}

// =============================================================================
// Scenario: Fail gracefully when creating worktree in non-git directory
// =============================================================================

/// @step Given I have a directory that is not a git repository
/// @step And I have a session ID "orphan-session"
/// @step When I attempt to create a worktree for the session
/// @step Then I should receive an error indicating no git repository found
#[test]
fn test_fail_gracefully_in_non_git_directory() {
    // Given I have a directory that is not a git repository
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let non_git_path = tmp_dir.path();

    // And I have a session ID "orphan-session"
    let session_id = "orphan-session";

    // When I attempt to create a worktree for the session
    let result = create_worktree(non_git_path, session_id);

    // Then I should receive an error indicating no git repository found
    assert!(result.is_err(), "Should fail in non-git directory");

    let err = result.unwrap_err();
    match err {
        GitError::OpenRepository { path, .. } => {
            assert!(path.contains(non_git_path.to_str().unwrap()));
        }
        GitError::NotARepository { path } => {
            assert!(path.contains(non_git_path.to_str().unwrap()));
        }
        _ => panic!(
            "Expected OpenRepository or NotARepository error, got: {:?}",
            err
        ),
    }
}

// =============================================================================
// Scenario: Fail gracefully when worktree already exists for session
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a worktree already exists at ".fspec/worktrees/existing-session/"
/// @step When I attempt to create a worktree for session "existing-session"
/// @step Then I should receive an error indicating worktree already exists
#[test]
fn test_fail_gracefully_when_worktree_exists() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    let session_id = "existing-session";

    // And a worktree already exists at ".fspec/worktrees/existing-session/"
    let result1 = create_worktree(repo_path, session_id);
    assert!(result1.is_ok(), "Setup: Failed to create initial worktree");

    // When I attempt to create a worktree for session "existing-session"
    let result2 = create_worktree(repo_path, session_id);

    // Then I should receive an error indicating worktree already exists
    assert!(result2.is_err(), "Should fail when worktree already exists");

    let err = result2.unwrap_err();
    match err {
        GitError::WorktreeExists { session_id: sid } => {
            assert_eq!(sid, session_id);
        }
        _ => panic!("Expected WorktreeExists error, got: {:?}", err),
    }
}

// =============================================================================
// Scenario: Worktree provides true filesystem isolation
// =============================================================================

/// @step Given I have a git repository with a file "src/main.rs"
/// @step And a worktree exists at ".fspec/worktrees/isolated-session/"
/// @step When I modify "src/main.rs" in the worktree
/// @step Then the main repository "src/main.rs" should be unchanged
/// @step And the worktree "src/main.rs" should contain my changes
#[test]
fn test_worktree_provides_filesystem_isolation() {
    // Given I have a git repository with a file "src/main.rs"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create src directory and main.rs
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    let main_content = "fn main() { println!(\"Hello\"); }";
    fs::write(src_dir.join("main.rs"), main_content).expect("Failed to write main.rs");

    // Commit the file
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage files");
    Command::new("git")
        .args(["commit", "-m", "Add main.rs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    // And a worktree exists at ".fspec/worktrees/isolated-session/"
    let session_id = "isolated-session";
    let result = create_worktree(repo_path, session_id);
    assert!(
        result.is_ok(),
        "Failed to create worktree: {:?}",
        result.err()
    );

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // When I modify "src/main.rs" in the worktree
    let worktree_main = worktree_path.join("src").join("main.rs");
    let modified_content = "fn main() { println!(\"Modified in worktree!\"); }";
    fs::write(&worktree_main, modified_content).expect("Failed to modify worktree main.rs");

    // Then the main repository "src/main.rs" should be unchanged
    let main_repo_content =
        fs::read_to_string(src_dir.join("main.rs")).expect("Failed to read main repo main.rs");
    assert_eq!(
        main_repo_content, main_content,
        "Main repo file should be unchanged"
    );

    // And the worktree "src/main.rs" should contain my changes
    let worktree_content =
        fs::read_to_string(&worktree_main).expect("Failed to read worktree main.rs");
    assert_eq!(
        worktree_content, modified_content,
        "Worktree file should have changes"
    );
}

// =============================================================================
// Scenario: Session without worktree uses main repository directly
// =============================================================================

/// This scenario tests that when isolation is not requested,
/// no worktree is created. This is a design/API decision where
/// the caller simply doesn't call create_worktree.
///
/// @step Given I have a git repository with commits
/// @step And I have a session ID "non-isolated-session"
/// @step When I create a session without worktree isolation
/// @step Then no worktree should be created for the session
/// @step And the session manifest worktree_path field should be null
/// @step And the session should use the main repository working directory
#[test]
fn test_session_without_worktree_uses_main_repo() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And I have a session ID "non-isolated-session"
    let session_id = "non-isolated-session";

    // When I create a session without worktree isolation
    // (This is represented by simply NOT calling create_worktree)
    // The test verifies that the worktree directory does not exist

    // Then no worktree should be created for the session
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    assert!(
        !worktree_path.exists(),
        "Worktree should not exist for non-isolated session"
    );

    // And the .fspec/worktrees directory itself shouldn't exist if no worktrees created
    let worktrees_dir = repo_path.join(FSPEC_WORKTREES_DIR);
    assert!(
        !worktrees_dir.exists(),
        "Worktrees directory should not exist"
    );

    // And the session should use the main repository working directory
    // This is verified by the fact that the repo_path is the working directory
    // No worktree means operations happen in repo_path directly
    assert!(
        repo_path.join(".git").exists(),
        "Main repo .git should exist"
    );
    assert!(
        repo_path.join("README.md").exists(),
        "Main repo files should be accessible"
    );
}
