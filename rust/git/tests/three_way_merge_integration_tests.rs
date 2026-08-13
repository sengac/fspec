//! Integration tests for BUG-098: Three-way merge conflict marker generation
//!
//! Feature: spec/features/merge-conflict-markers.feature
//!
//! These tests exercise the FULL apply_session_changes() flow through
//! real git repos with real worktrees — verifying that conflict markers
//! are actually written to worktree files, not just that the merge
//! algorithm works in isolation.
//!
//! NO MOCKS — everything uses real git repos with real file I/O.

mod common;

use codelet_git::{apply_session_changes, create_worktree, GitError, FSPEC_WORKTREES_DIR};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to get HEAD commit SHA
fn get_head_sha(repo_path: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// =============================================================================
// Scenario: Conflicting text file gets standard git conflict markers written
//           to worktree (full integration through apply_session_changes)
// =============================================================================

/// @step Given a session worktree with base commit containing "README.md"
/// @step And the session has modified line 7 of "README.md" from "The Spec-Driven" to "Da Spec-Driven"
/// @step And the main worktree has modified line 7 of "README.md" from "The Spec-Driven" to "The Spec-Driven (v2.0)"
/// @step When apply_session_changes is called
/// @step Then the worktree "README.md" should contain "<<<<<<< session (your changes)"
/// @step And the worktree "README.md" should contain "======="
/// @step And the worktree "README.md" should contain ">>>>>>> main"
/// @step And a ConflictError should be returned listing "README.md"
#[test]
fn test_apply_writes_conflict_markers_to_worktree_file() {
    // Given a git repository with a README.md at a known base content
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let base_content = "line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven\nline8\n";
    fs::write(repo_path.join("README.md"), base_content).expect("write base");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "base state"])
        .current_dir(repo_path)
        .output()
        .expect("commit");

    let _base_sha = get_head_sha(repo_path);

    // Create a session worktree
    let session_id = "conflict-markers-test";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Session modifies line 7
    let session_content = "line1\nline2\nline3\nline4\nline5\nline6\nDa Spec-Driven\nline8\n";
    fs::write(worktree_path.join("README.md"), session_content).expect("write session");

    // Main also modifies line 7
    let main_content = "line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven (v2.0)\nline8\n";
    fs::write(repo_path.join("README.md"), main_content).expect("write main");

    // When apply_session_changes is called
    let result = apply_session_changes(repo_path, session_id);

    // Then ConflictError is returned listing "README.md"
    assert!(result.is_err(), "Should return error for conflicting file");
    match result.unwrap_err() {
        GitError::ConflictError { files } => {
            assert!(
                files.contains(&"README.md".to_string()),
                "ConflictError should list README.md: {:?}",
                files
            );
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }

    // And the worktree README.md should contain conflict markers
    let worktree_content =
        fs::read_to_string(worktree_path.join("README.md")).expect("read worktree");
    assert!(
        worktree_content.contains("<<<<<<< session (your changes)"),
        "Worktree file should contain session marker. Got:\n{}",
        worktree_content
    );
    assert!(
        worktree_content.contains("======="),
        "Worktree file should contain separator. Got:\n{}",
        worktree_content
    );
    assert!(
        worktree_content.contains(">>>>>>> main"),
        "Worktree file should contain main marker. Got:\n{}",
        worktree_content
    );
    assert!(
        worktree_content.contains("Da Spec-Driven"),
        "Worktree file should contain session change. Got:\n{}",
        worktree_content
    );
    assert!(
        worktree_content.contains("The Spec-Driven (v2.0)"),
        "Worktree file should contain main change. Got:\n{}",
        worktree_content
    );
}

// =============================================================================
// Scenario: Non-overlapping changes in same file merge cleanly without
//           conflict markers (full integration)
// =============================================================================

/// @step Given a session worktree with base commit containing "src/app.ts"
/// @step And the session has modified lines 10-15 of "src/app.ts"
/// @step And the main worktree has modified lines 40-50 of "src/app.ts" with no overlap
/// @step When apply_session_changes is called
/// @step Then "src/app.ts" should be copied to the main worktree with both changes merged
/// @step And no ConflictError should be returned
/// @step And "src/app.ts" should NOT contain conflict markers
#[test]
fn test_apply_auto_merges_non_overlapping_changes() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create a file with 20 lines of content at base
    let mut base_lines: Vec<String> = (1..=20).map(|i| format!("line{}", i)).collect();
    let base_content = base_lines.join("\n") + "\n";
    fs::create_dir_all(repo_path.join("src")).expect("mkdir");
    fs::write(repo_path.join("src/app.ts"), &base_content).expect("write base");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "add app.ts"])
        .current_dir(repo_path)
        .output()
        .expect("commit");

    let session_id = "auto-merge-test";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Session modifies lines 2-3 (top)
    let mut session_lines = base_lines.clone();
    session_lines[1] = "SESSION_EDIT_2".to_string();
    session_lines[2] = "SESSION_EDIT_3".to_string();
    fs::write(
        worktree_path.join("src/app.ts"),
        session_lines.join("\n") + "\n",
    )
    .expect("write session");

    // Main modifies lines 18-19 (bottom, no overlap)
    base_lines[17] = "MAIN_EDIT_18".to_string();
    base_lines[18] = "MAIN_EDIT_19".to_string();
    fs::write(repo_path.join("src/app.ts"), base_lines.join("\n") + "\n").expect("write main");

    // When apply_session_changes is called
    let result = apply_session_changes(repo_path, session_id);

    // Then no ConflictError should be returned
    assert!(
        result.is_ok(),
        "Non-overlapping changes should merge cleanly: {:?}",
        result.err()
    );

    // And main should have both changes merged
    let main_content = fs::read_to_string(repo_path.join("src/app.ts")).expect("read main");
    assert!(
        main_content.contains("SESSION_EDIT_2"),
        "Main should contain session edit"
    );
    assert!(
        main_content.contains("MAIN_EDIT_18"),
        "Main should contain main edit"
    );
    assert!(
        !main_content.contains("<<<<<<<"),
        "Auto-merged file should not contain conflict markers"
    );
}

// =============================================================================
// Scenario: File added in both session and main with different content
//           gets conflict markers (full integration)
// =============================================================================

/// @step Given a session worktree with base commit that does NOT contain "utils.ts"
/// @step And the session has added "utils.ts" with content "export const x = 1;"
/// @step And the main worktree has also added "utils.ts" with content "export const x = 2;"
/// @step When apply_session_changes is called
/// @step Then the worktree "utils.ts" should contain "<<<<<<< session (your changes)"
/// @step And a ConflictError should be returned listing "utils.ts"
#[test]
fn test_apply_new_file_conflict_writes_markers() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let session_id = "new-file-conflict";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Session adds utils.ts
    fs::write(worktree_path.join("utils.ts"), "export const x = 1;\n").expect("write session");

    // Main also adds utils.ts with different content
    fs::write(repo_path.join("utils.ts"), "export const x = 2;\n").expect("write main");

    let result = apply_session_changes(repo_path, session_id);

    // ConflictError should be returned listing "utils.ts"
    assert!(result.is_err());
    match result.unwrap_err() {
        GitError::ConflictError { files } => {
            assert!(files.contains(&"utils.ts".to_string()));
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }

    // Worktree file should have conflict markers
    let content = fs::read_to_string(worktree_path.join("utils.ts")).expect("read worktree");
    assert!(
        content.contains("<<<<<<< session (your changes)"),
        "New-file conflict should have session marker. Got:\n{}",
        content
    );
    assert!(
        content.contains(">>>>>>> main"),
        "New-file conflict should have main marker. Got:\n{}",
        content
    );
}

// =============================================================================
// Scenario: Binary file conflict is reported without writing conflict markers
//           (full integration)
// =============================================================================

/// @step Given a session worktree with base commit containing binary file "logo.png"
/// @step And the session has modified "logo.png" with new binary content
/// @step And the main worktree has also modified "logo.png" with different binary content
/// @step When apply_session_changes is called
/// @step Then a ConflictError should be returned listing "logo.png"
/// @step And the worktree "logo.png" should NOT contain conflict markers
/// @step And the worktree "logo.png" should retain the session version
#[test]
fn test_apply_binary_conflict_no_markers() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create a binary file at base
    let base_binary: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x01, 0x02, 0x03];
    fs::write(repo_path.join("logo.png"), &base_binary).expect("write base binary");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "add binary"])
        .current_dir(repo_path)
        .output()
        .expect("commit");

    let session_id = "binary-conflict";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Session modifies binary file
    let session_binary: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0xAA, 0xBB, 0xCC];
    fs::write(worktree_path.join("logo.png"), &session_binary).expect("write session binary");

    // Main also modifies binary file
    let main_binary: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0xDD, 0xEE, 0xFF];
    fs::write(repo_path.join("logo.png"), &main_binary).expect("write main binary");

    let result = apply_session_changes(repo_path, session_id);

    // ConflictError with "logo.png"
    assert!(result.is_err());
    match result.unwrap_err() {
        GitError::ConflictError { files } => {
            assert!(files.contains(&"logo.png".to_string()));
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }

    // Binary file in worktree should NOT have conflict markers
    let file_content = fs::read(worktree_path.join("logo.png")).expect("read worktree binary");
    let as_str = String::from_utf8_lossy(&file_content);
    assert!(
        !as_str.contains("<<<<<<<"),
        "Binary file should NOT have conflict markers"
    );

    // Should retain the session version (unchanged by merge attempt)
    assert_eq!(
        file_content, session_binary,
        "Binary file should retain session version"
    );
}

// =============================================================================
// Scenario: Identical changes from session and main do not produce a conflict
//           (full integration)
// =============================================================================

/// @step Given a session worktree with base commit containing "README.md"
/// @step And the session has modified line 7 of "README.md" from "The" to "Da"
/// @step And the main worktree has also modified line 7 of "README.md" from "The" to "Da"
/// @step When apply_session_changes is called
/// @step Then no ConflictError should be returned
/// @step And "README.md" should be applied to the main worktree without conflict markers
#[test]
fn test_apply_identical_changes_no_conflict() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let base_content = "line1\nline2\nline3\nline4\nline5\nline6\nThe\nline8\n";
    fs::write(repo_path.join("README.md"), base_content).expect("write base");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "identical base"])
        .current_dir(repo_path)
        .output()
        .expect("commit");

    let session_id = "identical-changes";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Both session and main make the identical change
    let changed_content = "line1\nline2\nline3\nline4\nline5\nline6\nDa\nline8\n";
    fs::write(worktree_path.join("README.md"), changed_content).expect("write session");
    fs::write(repo_path.join("README.md"), changed_content).expect("write main");

    let result = apply_session_changes(repo_path, session_id);

    // Should succeed (identical changes auto-resolve)
    assert!(
        result.is_ok(),
        "Identical changes should not conflict: {:?}",
        result.err()
    );

    // Main should have the content without conflict markers
    let main_content = fs::read_to_string(repo_path.join("README.md")).expect("read main");
    assert!(main_content.contains("Da"), "Should contain the change");
    assert!(
        !main_content.contains("<<<<<<<"),
        "Should not have conflict markers"
    );
}

// =============================================================================
// Scenario: Re-running merge after resolving conflict markers succeeds
//           (full integration)
// =============================================================================

/// @step Given a session worktree where conflict markers were previously written to "README.md"
/// @step And the user has resolved the conflict markers in "README.md" by editing the worktree file
/// @step And the main worktree "README.md" matches the resolved version
/// @step When apply_session_changes is called again
/// @step Then no ConflictError should be returned
/// @step And the resolved "README.md" should be applied to the main worktree
#[test]
fn test_apply_re_merge_after_resolution_succeeds() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let base_content = "line1\nline2\nThe Spec-Driven\nline4\n";
    fs::write(repo_path.join("README.md"), base_content).expect("write base");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "re-merge base"])
        .current_dir(repo_path)
        .output()
        .expect("commit");

    let session_id = "re-merge-test";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // First merge: create a conflict
    fs::write(
        worktree_path.join("README.md"),
        "line1\nline2\nDa Spec-Driven\nline4\n",
    )
    .expect("write session v1");
    fs::write(
        repo_path.join("README.md"),
        "line1\nline2\nThe Spec-Driven (v2.0)\nline4\n",
    )
    .expect("write main v1");

    let result = apply_session_changes(repo_path, session_id);
    assert!(result.is_err(), "First merge should conflict");

    // Verify conflict markers were written to worktree
    let worktree_content =
        fs::read_to_string(worktree_path.join("README.md")).expect("read worktree");
    assert!(
        worktree_content.contains("<<<<<<<"),
        "Conflict markers should be in worktree after first merge"
    );

    // User resolves the conflict: pick the merged content
    let resolved = "line1\nline2\nDa Spec-Driven (v2.0)\nline4\n";
    fs::write(worktree_path.join("README.md"), resolved).expect("write resolved");

    // Main also has this content now (user applied it)
    fs::write(repo_path.join("README.md"), resolved).expect("write main resolved");

    // Re-merge should succeed (session == main → identical changes → clean)
    let result2 = apply_session_changes(repo_path, session_id);
    assert!(
        result2.is_ok(),
        "Re-merge after resolution should succeed: {:?}",
        result2.err()
    );
}
