//! Integration tests for BUG-099: Re-merge after conflict resolution infinite loop
//!
//! Feature: spec/features/merge-conflict-resolution-loop.feature
//!
//! These tests exercise the FULL apply_session_changes() flow through real git
//! repos to prove the infinite loop bug exists and verify the fix.
//!
//! The debug session bb90f15f (2026-02-26T23:51:12) showed:
//!   - Base README.md: "**Ma Spec-Driven, Multi-Agent Coding Factory**" (original)
//!     was changed by session to "**Ma Spec-Driven...**" and by main to
//!     "**Those Spec-Driven...**"
//!   - LLM resolved conflict markers by keeping session's version
//!   - apply_session_changes() re-detected conflict on re-merge → infinite loop
//!
//! NO MOCKS — everything uses real git repos with real file I/O.

mod common;

use codelet_git::{apply_session_changes, create_worktree, GitError, FSPEC_WORKTREES_DIR};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Reusable Fixtures
// =============================================================================

/// A conflict scenario fixture: real git repo + worktree with a known conflict.
///
/// After setup, the worktree file contains conflict markers from the first
/// apply_session_changes() call. The fixture is ready for "resolution + re-merge"
/// testing.
struct ConflictFixture {
    /// The temp directory (kept alive for the test's lifetime)
    _tmp_dir: tempfile::TempDir,
    /// Path to the main repository
    repo_path: PathBuf,
    /// Path to the session worktree
    worktree_path: PathBuf,
    /// Session ID used
    session_id: String,
}



/// Create a single-file conflict fixture.
///
/// Sets up: base → session modifies file → main modifies file (overlapping)
/// → first apply_session_changes() called → ConflictError returned
/// → worktree now has conflict markers.
///
/// Returns the fixture ready for resolution testing.
fn setup_single_file_conflict(
    session_id: &str,
    filename: &str,
    base_content: &str,
    session_content: &str,
    main_content: &str,
) -> ConflictFixture {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path().to_path_buf();

    // Write base content and commit
    fs::write(repo_path.join(filename), base_content).expect("write base");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "base state"])
        .current_dir(&repo_path)
        .output()
        .expect("commit");

    // Create session worktree
    create_worktree(&repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Session modifies file
    fs::write(worktree_path.join(filename), session_content).expect("write session");

    // Main also modifies file (overlapping change → conflict)
    fs::write(repo_path.join(filename), main_content).expect("write main");

    // First merge → should return ConflictError and write markers
    let result = apply_session_changes(&repo_path, session_id);
    assert!(
        result.is_err(),
        "First merge should conflict for {}",
        filename
    );
    match result.unwrap_err() {
        GitError::ConflictError { ref files } => {
            assert!(
                files.contains(&filename.to_string()),
                "ConflictError should list {}: {:?}",
                filename,
                files
            );
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }

    // Verify markers were written
    let worktree_content =
        fs::read_to_string(worktree_path.join(filename)).expect("read worktree");
    assert!(
        worktree_content.contains("<<<<<<< session (your changes)"),
        "Worktree should have markers after first merge. Got:\n{}",
        worktree_content
    );

    ConflictFixture {
        _tmp_dir: tmp_dir,
        repo_path,
        worktree_path,
        session_id: session_id.to_string(),
    }
}

/// Create a multi-file conflict fixture.
///
/// Same as single but sets up conflicts in multiple files at once.
fn setup_multi_file_conflict(
    session_id: &str,
    files: &[(&str, &str, &str, &str)], // (filename, base, session, main)
) -> ConflictFixture {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path().to_path_buf();

    // Write all base files and commit
    for (filename, base_content, _, _) in files {
        if let Some(parent) = Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(repo_path.join(parent)).expect("mkdir");
            }
        }
        fs::write(repo_path.join(filename), base_content).expect("write base");
    }
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("stage");
    Command::new("git")
        .args(["commit", "-m", "base state"])
        .current_dir(&repo_path)
        .output()
        .expect("commit");

    // Create session worktree
    create_worktree(&repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Apply session + main modifications
    for (filename, _, session_content, main_content) in files {
        fs::write(worktree_path.join(filename), session_content).expect("write session");
        fs::write(repo_path.join(filename), main_content).expect("write main");
    }

    // First merge → ConflictError
    let result = apply_session_changes(&repo_path, session_id);
    assert!(result.is_err(), "First merge should conflict");

    ConflictFixture {
        _tmp_dir: tmp_dir,
        repo_path,
        worktree_path,
        session_id: session_id.to_string(),
    }
}

// =============================================================================
// TEST 1: Reproduce the exact bug from debug session bb90f15f
//
// This test PROVES the infinite loop exists in the current code.
// It must FAIL until BUG-099 is fixed.
// =============================================================================

/// Exact reproduction of debug session bb90f15f.
///
/// Scenario: Re-merge after conflict resolution does not enter infinite loop (L52)
#[test]
fn test_bug_099_exact_reproduction_from_debug_session() {
    // @step Given a session worktree with base commit containing "README.md" with "The Spec-Driven"
    // @step And the session has modified "README.md" to "Ma Spec-Driven"
    // @step And the main worktree has modified "README.md" to "Those Spec-Driven"
    let base = "line1\nline2\n**The Spec-Driven, Multi-Agent Coding Factory**\nline4\n";
    let session = "line1\nline2\n**Ma Spec-Driven, Multi-Agent Coding Factory**\nline4\n";
    let main = "line1\nline2\n**Those Spec-Driven, Multi-Agent Coding Factory**\nline4\n";

    // @step When apply_session_changes is called the first time
    let fixture = setup_single_file_conflict(
        "bb90f15f-repro",
        "README.md",
        base,
        session,
        main,
    );

    // @step Then a ConflictError should be returned listing "README.md"
    // (verified inside setup_single_file_conflict)

    // @step And the worktree "README.md" should contain "<<<<<<< session (your changes)"
    // (verified inside setup_single_file_conflict)

    // @step And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"
    // (will be verified after fix is implemented)

    // @step When the user resolves "README.md" by removing conflict markers and keeping "Ma Spec-Driven"
    let resolved = "line1\nline2\n**Ma Spec-Driven, Multi-Agent Coding Factory**\nline4\n";
    fs::write(fixture.worktree_path.join("README.md"), resolved)
        .expect("LLM resolves conflict");

    // @step And apply_session_changes is called again
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then the merge should succeed without returning a ConflictError
    assert!(
        result.is_ok(),
        "BUG-099: Re-merge after LLM resolution should succeed, not loop.\n\
         Got error: {:?}\n\
         This proves the infinite loop bug — detect_conflicts() re-fires\n\
         because worktree('Ma') != base('The') AND main('Those') != base('The')",
        result.err()
    );

    // @step And the main worktree "README.md" should contain "Ma Spec-Driven"
    let main_content =
        fs::read_to_string(fixture.repo_path.join("README.md")).expect("read main");
    assert!(
        main_content.contains("**Ma Spec-Driven, Multi-Agent Coding Factory**"),
        "Main should have LLM's resolved content after successful re-merge"
    );

    // @step And the ".fspec-pending-conflicts" file should be deleted
    let state_file = fixture.worktree_path.join(".fspec-pending-conflicts");
    assert!(
        !state_file.exists(),
        ".fspec-pending-conflicts should be deleted after successful re-merge"
    );
}

// =============================================================================
// TEST 2: Prove markers are regenerated on re-merge (the overwrite problem)
//
// This test shows that the current code overwrites the LLM's resolution
// with fresh markers — proving the second half of the bug.
// =============================================================================

/// Scenario: Re-merge without fix produces double-nested markers corruption (L121)
///
/// This test verifies that the fix PREVENTS the corruption.
/// With fix: re-merge succeeds (worktree cleaned up, no corruption).
/// Without fix: markers-inside-markers corruption would occur.
#[test]
fn test_bug_099_markers_regenerated_overwrite_resolution() {
    // @step Given a session worktree with pending conflicts listing "README.md"
    let base = "line1\nline2\n**The Spec-Driven**\nline4\n";
    let session = "line1\nline2\n**Ma Spec-Driven**\nline4\n";
    let main = "line1\nline2\n**Those Spec-Driven**\nline4\n";

    let fixture = setup_single_file_conflict(
        "overwrite-repro",
        "README.md",
        base,
        session,
        main,
    );

    // @step And the worktree "README.md" still contains "<<<<<<< session (your changes)" markers
    // LLM resolves: keeps session version (no markers)
    let resolved = "line1\nline2\n**Ma Spec-Driven**\nline4\n";
    fs::write(fixture.worktree_path.join("README.md"), resolved)
        .expect("LLM resolves");

    // Verify: no markers in worktree after resolution
    let before_remerge =
        fs::read_to_string(fixture.worktree_path.join("README.md")).expect("read");
    assert!(
        !before_remerge.contains("<<<<<<<"),
        "After LLM resolution, worktree should NOT have markers"
    );

    // @step When apply_session_changes is called without the pending-conflicts check
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then the worktree "README.md" would contain double-nested markers
    // @step And the markers would be nested as "<<<<<<< session" inside "<<<<<<< session"
    // @step And this corruption makes the file unresolvable by the LLM
    //
    // BUG-099 FIX: With the fix, re-merge succeeds — no corruption occurs.
    // The worktree is cleaned up (removed) after successful apply.
    // Without the fix, the result would be Err and worktree would contain
    // double-nested markers: <<<<<<< session inside <<<<<<< session.
    assert!(
        result.is_ok(),
        "BUG-099: Re-merge should succeed after resolution (preventing double-nested corruption).\n\
         Current bug: markers regenerated, overwriting LLM's work."
    );

    // After successful merge, main should have the resolved content (not corrupted markers)
    let main_content =
        fs::read_to_string(fixture.repo_path.join("README.md")).expect("read main");
    assert!(
        main_content.contains("**Ma Spec-Driven**"),
        "Main should have resolved content, not corrupted double-nested markers. Got: {}",
        main_content
    );
    assert!(
        !main_content.contains("<<<<<<<"),
        "Main should NOT contain any conflict markers after successful resolution"
    );
}

// =============================================================================
// TEST 3: First merge creates .fspec-pending-conflicts state file
// (Feature scenario: "First merge creates pending conflict state file")
// =============================================================================

/// Scenario: First merge creates pending conflict state file alongside markers (L67)
#[test]
fn test_first_merge_creates_pending_conflicts_state_file() {
    // @step Given a session worktree with base commit containing "README.md" with "original"
    // @step And the session has modified "README.md" to "session version"
    // @step And the main worktree has modified "README.md" to "main version"
    // @step And no ".fspec-pending-conflicts" file exists in the worktree
    let base = "original content\n";
    let session = "session version\n";
    let main = "main version\n";

    // @step When apply_session_changes is called
    let fixture = setup_single_file_conflict(
        "pending-state-test",
        "README.md",
        base,
        session,
        main,
    );

    // @step Then a ConflictError should be returned listing "README.md"
    // (verified inside setup_single_file_conflict)

    // @step And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"
    let state_file = fixture.worktree_path.join(".fspec-pending-conflicts");
    assert!(
        state_file.exists(),
        "BUG-099: .fspec-pending-conflicts should be created after first conflict.\n\
         Expected at: {}",
        state_file.display()
    );

    // It should list README.md
    let state_content = fs::read_to_string(&state_file).expect("read state file");
    assert!(
        state_content.contains("README.md"),
        "State file should list README.md. Got: {}",
        state_content
    );
}

// =============================================================================
// TEST 4: Resolved file accepted without re-running three-way merge
// (Feature scenario: "Resolved conflict file is accepted")
// =============================================================================

/// Scenario: Resolved conflict file is accepted without re-running three-way merge (L77)
#[test]
fn test_resolved_file_accepted_on_remerge() {
    // @step Given a session worktree with pending conflicts listing "README.md"
    let base = "line1\nline2\noriginal\nline4\n";
    let session = "line1\nline2\nsession-edit\nline4\n";
    let main = "line1\nline2\nmain-edit\nline4\n";

    let fixture = setup_single_file_conflict(
        "resolved-accept",
        "README.md",
        base,
        session,
        main,
    );

    // @step And the worktree "README.md" does NOT contain "<<<<<<< " markers
    let resolved = "line1\nline2\nmanual-merge-result\nline4\n";
    fs::write(fixture.worktree_path.join("README.md"), resolved)
        .expect("write resolution");

    // @step When apply_session_changes is called
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then the merge should succeed
    assert!(
        result.is_ok(),
        "Resolved file should be accepted on re-merge: {:?}",
        result.err()
    );

    // @step And the worktree "README.md" content should be copied to main as the final resolution
    let main_content =
        fs::read_to_string(fixture.repo_path.join("README.md")).expect("read main");
    assert!(
        main_content.contains("manual-merge-result"),
        "Main should have LLM's resolution as final answer. Got: {}",
        main_content
    );

    // @step And the ".fspec-pending-conflicts" file should be deleted
    let state_file = fixture.worktree_path.join(".fspec-pending-conflicts");
    assert!(
        !state_file.exists(),
        ".fspec-pending-conflicts should be deleted after successful re-merge"
    );
}

// =============================================================================
// TEST 5: Unresolved file (markers still present) → re-return error, no overwrite
// (Feature scenario: "Unresolved file with markers still present")
// =============================================================================

/// Scenario: Unresolved file with markers still present is reported without regenerating markers (L88)
#[test]
fn test_unresolved_file_returns_error_without_overwrite() {
    // @step Given a session worktree with pending conflicts listing "README.md"
    let base = "line1\noriginal\nline3\n";
    let session = "line1\nsession-v\nline3\n";
    let main = "line1\nmain-v\nline3\n";

    let fixture = setup_single_file_conflict(
        "unresolved-test",
        "README.md",
        base,
        session,
        main,
    );

    // @step And the worktree "README.md" still contains "<<<<<<< session (your changes)" markers
    let markers_after_first =
        fs::read_to_string(fixture.worktree_path.join("README.md")).expect("read");
    assert!(markers_after_first.contains("<<<<<<<"));

    // @step When apply_session_changes is called
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then a ConflictError should be returned listing "README.md"
    assert!(result.is_err(), "Unresolved file should still error");

    // @step And the worktree "README.md" should NOT have its markers regenerated
    // @step And the worktree "README.md" content should be byte-identical to before the re-merge call
    let markers_after_second =
        fs::read_to_string(fixture.worktree_path.join("README.md")).expect("read");
    assert_eq!(
        markers_after_first, markers_after_second,
        "BUG-099: Markers should NOT be regenerated when file still has markers.\n\
         First:\n{}\nSecond:\n{}",
        markers_after_first, markers_after_second
    );

    // @step And the ".fspec-pending-conflicts" file should still exist
    let state_file = fixture.worktree_path.join(".fspec-pending-conflicts");
    assert!(
        state_file.exists(),
        ".fspec-pending-conflicts should still exist for unresolved conflicts"
    );
}

// =============================================================================
// TEST 6: Multi-file partial resolution reports only unresolved files
// (Feature scenario: "Multi-file conflict with partial resolution")
// =============================================================================

/// Scenario: Multi-file conflict with partial resolution reports only unresolved files (L98)
#[test]
fn test_multi_file_partial_resolution() {
    // @step Given a session worktree with pending conflicts listing "README.md" and "config.yml"
    let fixture = setup_multi_file_conflict(
        "partial-resolve",
        &[
            (
                "README.md",
                "readme base\n",
                "readme session\n",
                "readme main\n",
            ),
            (
                "config.yml",
                "config base\n",
                "config session\n",
                "config main\n",
            ),
        ],
    );

    // @step And the worktree "README.md" does NOT contain "<<<<<<< " markers
    fs::write(
        fixture.worktree_path.join("README.md"),
        "readme resolved by LLM\n",
    )
    .expect("resolve README.md");

    // @step And the worktree "config.yml" still contains "<<<<<<< " markers
    let config_content =
        fs::read_to_string(fixture.worktree_path.join("config.yml")).expect("read");
    assert!(
        config_content.contains("<<<<<<<"),
        "config.yml should still have markers"
    );

    // @step When apply_session_changes is called
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then a ConflictError should be returned listing only "config.yml"
    assert!(result.is_err(), "Should error for unresolved config.yml");
    match result.unwrap_err() {
        GitError::ConflictError { files } => {
            assert!(
                files.contains(&"config.yml".to_string()),
                "Should list config.yml: {:?}",
                files
            );
            // @step And the ConflictError should NOT list "README.md"
            assert!(
                !files.contains(&"README.md".to_string()),
                "Should NOT list README.md (it's resolved): {:?}",
                files
            );
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }
}

// =============================================================================
// TEST 7: Resolution matching main exactly succeeds
// (Feature scenario: "Resolution matching main exactly succeeds on re-merge")
// =============================================================================

/// Scenario: Resolution matching main exactly succeeds on re-merge (L107)
#[test]
fn test_resolution_matching_main_succeeds() {
    // @step Given a session worktree with pending conflicts listing "README.md"
    let base = "line1\noriginal\nline3\n";
    let session = "line1\nsession-edit\nline3\n";
    let main = "line1\nmain-edit\nline3\n";

    let fixture = setup_single_file_conflict(
        "match-main-test",
        "README.md",
        base,
        session,
        main,
    );

    // @step And the worktree "README.md" has been resolved to match main exactly
    let resolved = "line1\nmain-edit\nline3\n";
    fs::write(fixture.worktree_path.join("README.md"), resolved)
        .expect("resolve to match main");

    // @step When apply_session_changes is called
    let result = apply_session_changes(&fixture.repo_path, &fixture.session_id);

    // @step Then the merge should succeed
    assert!(
        result.is_ok(),
        "Resolution matching main should succeed: {:?}",
        result.err()
    );

    // @step And the ".fspec-pending-conflicts" file should be deleted
    let state_file = fixture.worktree_path.join(".fspec-pending-conflicts");
    assert!(
        !state_file.exists(),
        ".fspec-pending-conflicts should be deleted"
    );
}

// =============================================================================
// TEST 8: .fspec-pending-conflicts excluded from collect_worktree_files
// (Feature scenario: "Pending conflicts state file is not collected")
//
// collect_worktree_files is private, so we test indirectly via get_session_diff
// which uses it internally. If .fspec-pending-conflicts were collected, it would
// appear as "added" in the diff (since it's not in the base commit).
// =============================================================================

/// Scenario: Pending conflicts state file is not collected as a worktree file (L115)
#[test]
fn test_pending_conflicts_excluded_from_worktree_collection() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let session_id = "collect-exclude-test";
    create_worktree(repo_path, session_id).expect("create worktree");
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // @step Given a session worktree with a ".fspec-pending-conflicts" file present
    fs::write(
        worktree_path.join(".fspec-pending-conflicts"),
        r#"{"files":["README.md"]}"#,
    )
    .expect("write state file");

    // Also make an actual change so the diff has something
    fs::write(
        worktree_path.join("README.md"),
        "# Modified README\n",
    )
    .expect("modify readme");

    // @step When worktree files are collected for diff or apply
    let diff = codelet_git::get_session_diff(repo_path, session_id)
        .expect("get session diff");

    // @step Then ".fspec-pending-conflicts" should NOT appear in the collected file list
    let all_files: Vec<&String> = diff
        .files_changed
        .iter()
        .chain(diff.files_added.iter())
        .chain(diff.files_deleted.iter())
        .collect();

    assert!(
        !all_files.iter().any(|f| f.contains("fspec-pending-conflicts")),
        "BUG-099: .fspec-pending-conflicts should be excluded from collected files.\n\
         files_changed: {:?}\n\
         files_added: {:?}\n\
         files_deleted: {:?}",
        diff.files_changed,
        diff.files_added,
        diff.files_deleted
    );

    // The diff SHOULD contain the README.md change
    assert!(
        !diff.diff.is_empty(),
        "Diff should still capture real changes"
    );
}
