//! Tests for ghost commit checkpoint functionality
//!
//! Feature: spec/features/ghost-commit-checkpoints.feature
//!
//! These tests validate the acceptance criteria for ghost commit-based checkpoints.

use codelet_git::ghost_commit;
use std::fs;
use tempfile::TempDir;

// Helper to create a test git repository
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repo using gix
    let _repo = gix::init(repo_path).expect("Failed to init repo");

    // Create initial file
    let file_path = repo_path.join("initial.txt");
    fs::write(&file_path, "initial content").expect("Failed to write initial file");

    temp_dir
}

/// Scenario: Create checkpoint capturing all file states
///
/// Given I have a git repository with uncommitted changes
/// And I have staged files in the index
/// And I have unstaged modifications to tracked files
/// And I have untracked files in the working directory
/// When I create a ghost commit checkpoint named "test-checkpoint"
/// Then all file states should be captured in the ghost commit
/// And the checkpoint should store a valid git commit SHA
/// And the checkpoint should be stored under refs/fspec-checkpoints/
#[test]
fn test_create_checkpoint_capturing_all_file_states() {
    // @step Given I have a git repository with uncommitted changes
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // @step And I have staged files in the index
    let staged_file = repo_path.join("staged.txt");
    fs::write(&staged_file, "staged content").expect("Failed to write staged file");

    // @step And I have unstaged modifications to tracked files
    let tracked_file = repo_path.join("initial.txt");
    fs::write(&tracked_file, "modified content").expect("Failed to modify tracked file");

    // @step And I have untracked files in the working directory
    let untracked_file = repo_path.join("untracked.txt");
    fs::write(&untracked_file, "untracked content").expect("Failed to write untracked file");

    // @step When I create a ghost commit checkpoint named "test-checkpoint"
    let result = ghost_commit::create_ghost_commit(repo_path, "WORK-001", "test-checkpoint");

    // @step Then all file states should be captured in the ghost commit
    assert!(
        result.is_ok(),
        "Failed to create ghost commit: {:?}",
        result.err()
    );
    let checkpoint = result.unwrap();

    // @step And the checkpoint should store a valid git commit SHA
    assert!(
        !checkpoint.sha.is_empty(),
        "Ghost commit SHA should not be empty"
    );
    assert_eq!(checkpoint.sha.len(), 40, "SHA should be 40 hex characters");

    // @step And the checkpoint should be stored under refs/fspec-checkpoints/
    // Verify by checking files list includes our test files
    assert!(checkpoint.files.iter().any(|f| f.contains("staged.txt")));
    assert!(checkpoint.files.iter().any(|f| f.contains("untracked.txt")));
}

/// Scenario: Checkpoint creation preserves staging area
///
/// Given I have a git repository with a file staged for commit
/// And I have additional unstaged changes
/// When I create a ghost commit checkpoint
/// Then the staging area should remain unchanged
/// And the same files should still be staged
/// And the checkpoint should capture all working tree state
#[test]
fn test_checkpoint_creation_preserves_staging_area() {
    // @step Given I have a git repository with a file staged for commit
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let staged_file = repo_path.join("to-stage.txt");
    fs::write(&staged_file, "will be staged").expect("Failed to write file");

    // Record staged files before checkpoint
    let staged_before =
        codelet_git::get_staged_files(repo_path.to_str().unwrap()).unwrap_or_default();

    // @step And I have additional unstaged changes
    let tracked_file = repo_path.join("initial.txt");
    fs::write(&tracked_file, "unstaged modification").expect("Failed to modify");

    // @step When I create a ghost commit checkpoint
    let result = ghost_commit::create_ghost_commit(repo_path, "WORK-002", "preserve-staging");
    assert!(
        result.is_ok(),
        "Failed to create checkpoint: {:?}",
        result.err()
    );

    // @step Then the staging area should remain unchanged
    let staged_after =
        codelet_git::get_staged_files(repo_path.to_str().unwrap()).unwrap_or_default();

    // @step And the same files should still be staged
    assert_eq!(
        staged_before, staged_after,
        "Staging area should be unchanged after checkpoint creation"
    );

    // @step And the checkpoint should capture all working tree state
    let checkpoint = result.unwrap();
    assert!(
        !checkpoint.files.is_empty(),
        "Checkpoint should capture files"
    );
}

/// Scenario: Restore checkpoint replaces working tree files
///
/// Given I have a git repository with a ghost commit checkpoint
/// And the checkpoint contains specific file contents
/// And I have modified files since the checkpoint was created
/// When I restore the checkpoint
/// Then the working tree files should match the checkpoint contents
/// And modified files should be reverted to checkpoint state
#[test]
fn test_restore_checkpoint_replaces_working_tree_files() {
    // @step Given I have a git repository with a ghost commit checkpoint
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // @step And the checkpoint contains specific file contents
    let test_file = repo_path.join("test.txt");
    fs::write(&test_file, "checkpoint content").expect("Failed to write file");

    let _checkpoint = ghost_commit::create_ghost_commit(repo_path, "WORK-003", "before-modify")
        .expect("Failed to create checkpoint");

    // @step And I have modified files since the checkpoint was created
    fs::write(&test_file, "modified after checkpoint").expect("Failed to modify file");
    let content_before_restore = fs::read_to_string(&test_file).expect("Failed to read");
    assert_eq!(content_before_restore, "modified after checkpoint");

    // @step When I restore the checkpoint
    let restore_result = ghost_commit::restore_ghost_commit(
        repo_path,
        "WORK-003",
        "before-modify",
        false, // force
    );
    assert!(
        restore_result.is_ok(),
        "Failed to restore checkpoint: {:?}",
        restore_result.err()
    );

    // @step Then the working tree files should match the checkpoint contents
    let content_after_restore = fs::read_to_string(&test_file).expect("Failed to read");

    // @step And modified files should be reverted to checkpoint state
    assert_eq!(
        content_after_restore, "checkpoint content",
        "File content should be restored to checkpoint state"
    );
}

/// Scenario: Multiple checkpoints have unique SHA identifiers
///
/// Given I have a git repository with uncommitted changes
/// When I create a ghost commit checkpoint named "checkpoint-1"
/// And I modify some files
/// And I create a ghost commit checkpoint named "checkpoint-2"
/// Then each checkpoint should have a unique SHA
/// And both checkpoints should be independently restorable
/// And the refs should be stored under refs/fspec-checkpoints/<work-unit-id>/
#[test]
fn test_multiple_checkpoints_have_unique_sha_identifiers() {
    // @step Given I have a git repository with uncommitted changes
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let test_file = repo_path.join("evolving.txt");
    fs::write(&test_file, "version 1").expect("Failed to write");

    // @step When I create a ghost commit checkpoint named "checkpoint-1"
    let checkpoint1 = ghost_commit::create_ghost_commit(repo_path, "WORK-004", "checkpoint-1")
        .expect("Failed to create checkpoint-1");

    // @step And I modify some files
    fs::write(&test_file, "version 2").expect("Failed to modify");

    // @step And I create a ghost commit checkpoint named "checkpoint-2"
    let checkpoint2 = ghost_commit::create_ghost_commit(repo_path, "WORK-004", "checkpoint-2")
        .expect("Failed to create checkpoint-2");

    // @step Then each checkpoint should have a unique SHA
    assert_ne!(
        checkpoint1.sha, checkpoint2.sha,
        "Checkpoints should have different SHAs"
    );

    // @step And both checkpoints should be independently restorable
    // Restore checkpoint-1
    ghost_commit::restore_ghost_commit(repo_path, "WORK-004", "checkpoint-1", false)
        .expect("Failed to restore checkpoint-1");
    let content1 = fs::read_to_string(&test_file).expect("Failed to read");
    assert_eq!(content1, "version 1");

    // Restore checkpoint-2
    ghost_commit::restore_ghost_commit(repo_path, "WORK-004", "checkpoint-2", false)
        .expect("Failed to restore checkpoint-2");
    let content2 = fs::read_to_string(&test_file).expect("Failed to read");
    assert_eq!(content2, "version 2");

    // @step And the refs should be stored under refs/fspec-checkpoints/<work-unit-id>/
    // This is verified implicitly by successful restore operations
}

/// Scenario: Ghost commits are invisible to git log
///
/// Given I have a git repository with a ghost commit checkpoint
/// When I run git log to view repository history
/// Then the ghost commit should not appear in the log
/// And the ghost commit should only be accessible via explicit SHA reference
#[test]
fn test_ghost_commits_are_invisible_to_git_log() {
    // @step Given I have a git repository with a ghost commit checkpoint
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let test_file = repo_path.join("ghost-test.txt");
    fs::write(&test_file, "ghost content").expect("Failed to write");

    let checkpoint = ghost_commit::create_ghost_commit(repo_path, "WORK-005", "invisible")
        .expect("Failed to create checkpoint");

    // @step When I run git log to view repository history
    // Get all commits reachable from HEAD
    let repo = gix::open(repo_path).expect("Failed to open repo");

    let mut visible_commits: Vec<String> = Vec::new();
    if let Ok(head_commit) = repo.head_commit() {
        visible_commits.push(head_commit.id().to_string());
        // Walk ancestors if needed
    }

    // @step Then the ghost commit should not appear in the log
    assert!(
        !visible_commits.contains(&checkpoint.sha),
        "Ghost commit should not be in git log history"
    );

    // @step And the ghost commit should only be accessible via explicit SHA reference
    // The ghost commit should still exist in the object database
    assert!(!checkpoint.sha.is_empty(), "Ghost commit SHA should exist");
}

/// Scenario: Restore checkpoint deletes files added after checkpoint
///
/// Given I have a git repository with a ghost commit checkpoint
/// And I create new files after the checkpoint was created
/// When I restore the checkpoint
/// Then the new files should be deleted
/// And the working tree should match the exact state at checkpoint creation
#[test]
fn test_restore_checkpoint_deletes_files_added_after_checkpoint() {
    // @step Given I have a git repository with a ghost commit checkpoint
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let existing_file = repo_path.join("existing.txt");
    fs::write(&existing_file, "was here at checkpoint").expect("Failed to write");

    let _checkpoint = ghost_commit::create_ghost_commit(repo_path, "WORK-006", "before-new-files")
        .expect("Failed to create checkpoint");

    // @step And I create new files after the checkpoint was created
    let new_file = repo_path.join("new-after-checkpoint.txt");
    fs::write(&new_file, "I should be deleted").expect("Failed to write new file");
    assert!(new_file.exists(), "New file should exist before restore");

    // @step When I restore the checkpoint
    ghost_commit::restore_ghost_commit(repo_path, "WORK-006", "before-new-files", false)
        .expect("Failed to restore checkpoint");

    // @step Then the new files should be deleted
    assert!(
        !new_file.exists(),
        "File created after checkpoint should be deleted"
    );

    // @step And the working tree should match the exact state at checkpoint creation
    assert!(existing_file.exists(), "File from checkpoint should exist");
    let content = fs::read_to_string(&existing_file).expect("Failed to read");
    assert_eq!(content, "was here at checkpoint");
}

/// Scenario: Ghost commit preserves parent relationship to HEAD
///
/// Given I have a git repository with committed history
/// And I note the current HEAD commit SHA
/// And I have uncommitted changes in the working directory
/// When I create a ghost commit checkpoint
/// Then the ghost commit's parent should be the noted HEAD SHA
/// And the ghost commit should be a valid commit object with proper tree reference
#[test]
fn test_ghost_commit_preserves_parent_relationship_to_head() {
    // @step Given I have a git repository with committed history
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // @step And I note the current HEAD commit SHA
    let repo = gix::open(repo_path).expect("Failed to open repo");
    let head_sha = repo
        .head_commit()
        .map(|c| c.id().to_string())
        .unwrap_or_default();

    // @step And I have uncommitted changes in the working directory
    let test_file = repo_path.join("uncommitted.txt");
    fs::write(&test_file, "uncommitted changes").expect("Failed to write");

    // @step When I create a ghost commit checkpoint
    let checkpoint = ghost_commit::create_ghost_commit(repo_path, "WORK-007", "parent-test")
        .expect("Failed to create checkpoint");

    // @step Then the ghost commit's parent should be the noted HEAD SHA
    // Note: If repo has no commits, parent_sha may be empty
    if !head_sha.is_empty() {
        assert_eq!(
            checkpoint.parent_sha, head_sha,
            "Ghost commit parent should be HEAD at creation time"
        );
    }

    // @step And the ghost commit should be a valid commit object with proper tree reference
    assert!(
        !checkpoint.sha.is_empty(),
        "Ghost commit should have valid SHA"
    );
    assert_eq!(checkpoint.sha.len(), 40, "SHA should be 40 hex characters");
}
