//! Three-way merge with conflict marker generation
//!
//! Provides three-way text merging that produces standard git conflict markers
//! when changes overlap, and cleanly merges non-overlapping changes.
//!
//! BUG-098: This module was extracted from session_result.rs for separation
//! of concerns. It handles the merge logic independently of session/worktree
//! operations.

use crate::error::Result;
use crate::utils::is_binary_content;
use diffy::{ConflictStyle, MergeOptions};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Result of a three-way merge attempt on a single file
#[derive(Debug, Clone, PartialEq)]
pub enum MergeOutcome {
    /// Merge succeeded cleanly — no conflict markers
    Clean(String),
    /// Merge has conflicts — content contains conflict markers
    Conflict(String),
}

/// Perform a three-way merge on text content.
///
/// Given the common base, the session's version, and main's version,
/// produces either a cleanly merged result or content with standard
/// git conflict markers:
///
/// ```text
/// <<<<<<< session (your changes)
/// session content
/// =======
/// main content
/// >>>>>>> main
/// ```
///
/// # Arguments
/// * `base` - The common ancestor content
/// * `session` - The session's version (ours)
/// * `main` - The main worktree's version (theirs)
pub fn three_way_merge_text(base: &str, session: &str, main: &str) -> MergeOutcome {
    let mut opts = MergeOptions::new();
    opts.set_conflict_style(ConflictStyle::Merge);

    match opts.merge(base, session, main) {
        Ok(merged) => MergeOutcome::Clean(merged),
        Err(conflict_text) => {
            // Replace diffy's default labels with our custom labels
            let custom = conflict_text
                .replace("<<<<<<< ours", "<<<<<<< session (your changes)")
                .replace(">>>>>>> theirs", ">>>>>>> main");
            MergeOutcome::Conflict(custom)
        }
    }
}

/// Write conflict markers into worktree files for detected conflicts.
///
/// For each conflicting file:
/// - Text files: perform three-way merge, write result to worktree
/// - Binary files: skip (no markers possible), keep session version
///
/// Returns the list of files that have actual unresolvable conflicts
/// (auto-merged files are removed from the conflict list).
///
/// # Arguments
/// * `worktree_path` - Path to the session worktree directory
/// * `potential_conflicts` - Files detected as diverged (both sides changed)
/// * `base_tree_files` - Content of each file at the base commit
/// * `worktree_files` - Content of each file in the session worktree
/// * `main_files` - Content of each file in the main worktree
pub fn write_conflict_markers(
    worktree_path: &Path,
    potential_conflicts: &[String],
    base_tree_files: &HashMap<String, Vec<u8>>,
    worktree_files: &HashMap<String, Vec<u8>>,
    main_files: &HashMap<String, Vec<u8>>,
) -> Result<Vec<String>> {
    let mut actual_conflicts = Vec::new();

    for path in potential_conflicts {
        let session_content = worktree_files.get(path);
        let main_content = main_files.get(path);
        let base_content = base_tree_files.get(path);

        // Get the raw bytes for each version (empty if not present)
        let session_bytes = session_content.map(|v| v.as_slice()).unwrap_or(&[]);
        let main_bytes = main_content.map(|v| v.as_slice()).unwrap_or(&[]);
        let base_bytes = base_content.map(|v| v.as_slice()).unwrap_or(&[]);

        // Skip three-way merge for binary files — they remain as conflicts
        if is_binary_content(session_bytes)
            || is_binary_content(main_bytes)
            || is_binary_content(base_bytes)
        {
            actual_conflicts.push(path.clone());
            continue;
        }

        // Convert to UTF-8 (lossy) for text merge
        let base_str = String::from_utf8_lossy(base_bytes);
        let session_str = String::from_utf8_lossy(session_bytes);
        let main_str = String::from_utf8_lossy(main_bytes);

        match three_way_merge_text(&base_str, &session_str, &main_str) {
            MergeOutcome::Clean(merged) => {
                // Auto-resolved — write merged content to worktree
                let dest = worktree_path.join(path);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&dest, merged.as_bytes())?;
                // NOT a conflict — don't add to actual_conflicts
            }
            MergeOutcome::Conflict(conflict_text) => {
                // Write conflict markers to worktree file
                let dest = worktree_path.join(path);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&dest, conflict_text.as_bytes())?;
                actual_conflicts.push(path.clone());
            }
        }
    }

    actual_conflicts.sort();
    Ok(actual_conflicts)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: spec/features/merge-conflict-markers.feature
    //
    // This test module validates the acceptance criteria for BUG-098:
    // three-way merge with conflict markers in session worktrees.

    // =========================================================================
    // Scenario: Conflicting text file gets standard git conflict markers
    // =========================================================================

    #[test]
    fn test_overlapping_text_conflict_produces_markers() {
        // @step Given a session worktree with base commit containing "README.md"
        let base = "line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven\nline8\n";

        // @step And the session has modified line 7 of "README.md" from "The Spec-Driven" to "Da Spec-Driven"
        let session = "line1\nline2\nline3\nline4\nline5\nline6\nDa Spec-Driven\nline8\n";

        // @step And the main worktree has modified line 7 of "README.md" from "The Spec-Driven" to "The Spec-Driven (v2.0)"
        let main = "line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven (v2.0)\nline8\n";

        // @step When apply_session_changes is called
        let result = three_way_merge_text(base, session, main);

        // @step Then the worktree "README.md" should contain "<<<<<<< session (your changes)"
        // @step And the worktree "README.md" should contain "======="
        // @step And the worktree "README.md" should contain ">>>>>>> main"
        // @step And the worktree "README.md" should contain "Da Spec-Driven"
        // @step And the worktree "README.md" should contain "The Spec-Driven (v2.0)"
        match result {
            MergeOutcome::Conflict(content) => {
                assert!(
                    content.contains("<<<<<<< session (your changes)"),
                    "Missing session marker in:\n{}",
                    content
                );
                assert!(
                    content.contains("======="),
                    "Missing separator in:\n{}",
                    content
                );
                assert!(
                    content.contains(">>>>>>> main"),
                    "Missing main marker in:\n{}",
                    content
                );
                assert!(
                    content.contains("Da Spec-Driven"),
                    "Missing session content in:\n{}",
                    content
                );
                assert!(
                    content.contains("The Spec-Driven (v2.0)"),
                    "Missing main content in:\n{}",
                    content
                );
            }
            MergeOutcome::Clean(content) => {
                panic!("Expected conflict but got clean merge:\n{}", content);
            }
        }
    }

    // =========================================================================
    // Scenario: Non-overlapping changes in same file merge cleanly
    // =========================================================================

    #[test]
    fn test_non_overlapping_changes_merge_cleanly() {
        // @step Given a session worktree with base commit containing "src/app.ts"
        let base = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n\
                    line11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\nline19\nline20\n\
                    line21\nline22\nline23\nline24\nline25\nline26\nline27\nline28\nline29\nline30\n\
                    line31\nline32\nline33\nline34\nline35\nline36\nline37\nline38\nline39\nline40\n\
                    line41\nline42\nline43\nline44\nline45\n";

        // @step And the session has modified lines 10-15 of "src/app.ts"
        let session = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nSESSION_EDIT\n\
                      SESSION_EDIT\nSESSION_EDIT\nSESSION_EDIT\nSESSION_EDIT\nSESSION_EDIT\nline16\nline17\nline18\nline19\nline20\n\
                      line21\nline22\nline23\nline24\nline25\nline26\nline27\nline28\nline29\nline30\n\
                      line31\nline32\nline33\nline34\nline35\nline36\nline37\nline38\nline39\nline40\n\
                      line41\nline42\nline43\nline44\nline45\n";

        // @step And the main worktree has modified lines 40-50 of "src/app.ts" with no overlap
        let main = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n\
                   line11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\nline19\nline20\n\
                   line21\nline22\nline23\nline24\nline25\nline26\nline27\nline28\nline29\nline30\n\
                   line31\nline32\nline33\nline34\nline35\nline36\nline37\nline38\nline39\nMAIN_EDIT\n\
                   MAIN_EDIT\nMAIN_EDIT\nMAIN_EDIT\nMAIN_EDIT\nMAIN_EDIT\n";

        // @step When apply_session_changes is called
        let result = three_way_merge_text(base, session, main);

        // @step Then "src/app.ts" should be copied to the main worktree with both changes merged
        // @step And no ConflictError should be returned
        // @step And "src/app.ts" should NOT contain conflict markers
        match result {
            MergeOutcome::Clean(content) => {
                assert!(
                    content.contains("SESSION_EDIT"),
                    "Merged content should include session changes"
                );
                assert!(
                    content.contains("MAIN_EDIT"),
                    "Merged content should include main changes"
                );
                assert!(
                    !content.contains("<<<<<<<"),
                    "Clean merge should not contain conflict markers"
                );
            }
            MergeOutcome::Conflict(content) => {
                panic!("Expected clean merge but got conflict:\n{}", content);
            }
        }
    }

    // =========================================================================
    // Scenario: File added in both session and main with different content
    // =========================================================================

    #[test]
    fn test_new_file_both_sides_produces_conflict() {
        // @step Given a session worktree with base commit that does NOT contain "utils.ts"
        let base = "";

        // @step And the session has added "utils.ts" with content "export const x = 1;"
        let session = "export const x = 1;\n";

        // @step And the main worktree has also added "utils.ts" with content "export const x = 2;"
        let main = "export const x = 2;\n";

        // @step When apply_session_changes is called
        let result = three_way_merge_text(base, session, main);

        // @step Then the worktree "utils.ts" should contain "<<<<<<< session (your changes)"
        // @step And the worktree "utils.ts" should contain ">>>>>>> main"
        // @step And a ConflictError should be returned listing "utils.ts"
        match result {
            MergeOutcome::Conflict(content) => {
                assert!(
                    content.contains("<<<<<<< session (your changes)"),
                    "Missing session marker for new-file conflict:\n{}",
                    content
                );
                assert!(
                    content.contains(">>>>>>> main"),
                    "Missing main marker for new-file conflict:\n{}",
                    content
                );
            }
            MergeOutcome::Clean(content) => {
                panic!(
                    "Expected conflict for divergent new files but got clean merge:\n{}",
                    content
                );
            }
        }
    }

    // =========================================================================
    // Scenario: Binary file conflict is reported without writing conflict markers
    // =========================================================================

    #[test]
    fn test_binary_files_not_merged() {
        // @step Given a session worktree with base commit containing binary file "logo.png"
        let base_content: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x01, 0x02];

        // @step And the session has modified "logo.png" with new binary content
        let session_content: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x03, 0x04];

        // @step And the main worktree has also modified "logo.png" with different binary content
        let main_content: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x05, 0x06];

        // @step When apply_session_changes is called
        let worktree_dir = tempfile::tempdir().unwrap();
        let worktree_path = worktree_dir.path();

        // Create the worktree file with session content
        std::fs::write(worktree_path.join("logo.png"), &session_content).unwrap();

        let mut base_tree = HashMap::new();
        base_tree.insert("logo.png".to_string(), base_content);

        let mut worktree_files = HashMap::new();
        worktree_files.insert("logo.png".to_string(), session_content.clone());

        let mut main_files = HashMap::new();
        main_files.insert("logo.png".to_string(), main_content);

        let actual_conflicts = write_conflict_markers(
            worktree_path,
            &["logo.png".to_string()],
            &base_tree,
            &worktree_files,
            &main_files,
        )
        .unwrap();

        // @step Then a ConflictError should be returned listing "logo.png"
        assert!(
            actual_conflicts.contains(&"logo.png".to_string()),
            "Binary file should still be listed as conflicting"
        );

        // @step And the worktree "logo.png" should NOT contain conflict markers
        let file_content = std::fs::read(worktree_path.join("logo.png")).unwrap();
        let content_str = String::from_utf8_lossy(&file_content);
        assert!(
            !content_str.contains("<<<<<<<"),
            "Binary files should not have conflict markers"
        );

        // @step And the worktree "logo.png" should retain the session version
        assert_eq!(
            file_content, session_content,
            "Binary file should retain session version"
        );
    }

    // =========================================================================
    // Scenario: Identical changes from session and main do not produce a conflict
    // =========================================================================

    #[test]
    fn test_identical_changes_not_a_conflict() {
        // @step Given a session worktree with base commit containing "README.md"
        let base = "line1\nline2\nline3\nline4\nline5\nline6\nThe\nline8\n";

        // @step And the session has modified line 7 of "README.md" from "The" to "Da"
        let session = "line1\nline2\nline3\nline4\nline5\nline6\nDa\nline8\n";

        // @step And the main worktree has also modified line 7 of "README.md" from "The" to "Da"
        let main = "line1\nline2\nline3\nline4\nline5\nline6\nDa\nline8\n";

        // @step When apply_session_changes is called
        let result = three_way_merge_text(base, session, main);

        // @step Then no ConflictError should be returned
        // @step And "README.md" should be applied to the main worktree without conflict markers
        match result {
            MergeOutcome::Clean(content) => {
                assert!(
                    content.contains("Da"),
                    "Merged content should contain the identical change"
                );
                assert!(
                    !content.contains("<<<<<<<"),
                    "Identical changes should not produce conflict markers"
                );
            }
            MergeOutcome::Conflict(content) => {
                panic!(
                    "Identical changes should NOT produce a conflict:\n{}",
                    content
                );
            }
        }
    }

    // =========================================================================
    // Scenario: write_conflict_markers filters auto-resolved files
    // =========================================================================

    #[test]
    fn test_write_conflict_markers_filters_auto_resolved() {
        // Test that write_conflict_markers returns only files with actual
        // conflicts, filtering out those that auto-merge cleanly.
        let worktree_dir = tempfile::tempdir().unwrap();
        let worktree_path = worktree_dir.path();

        // File A: overlapping conflict (will have markers)
        let base_a = b"base line\n".to_vec();
        let session_a = b"session line\n".to_vec();
        let main_a = b"main line\n".to_vec();

        // File B: non-overlapping (will merge cleanly)
        let base_b = b"top\nmiddle\nbottom\n".to_vec();
        let session_b = b"SESSION_TOP\nmiddle\nbottom\n".to_vec();
        let main_b = b"top\nmiddle\nMAIN_BOTTOM\n".to_vec();

        // Write session versions to worktree
        std::fs::write(worktree_path.join("a.txt"), &session_a).unwrap();
        std::fs::write(worktree_path.join("b.txt"), &session_b).unwrap();

        let mut base_tree = HashMap::new();
        base_tree.insert("a.txt".to_string(), base_a);
        base_tree.insert("b.txt".to_string(), base_b);

        let mut worktree_files = HashMap::new();
        worktree_files.insert("a.txt".to_string(), session_a);
        worktree_files.insert("b.txt".to_string(), session_b);

        let mut main_files = HashMap::new();
        main_files.insert("a.txt".to_string(), main_a);
        main_files.insert("b.txt".to_string(), main_b);

        let actual_conflicts = write_conflict_markers(
            worktree_path,
            &["a.txt".to_string(), "b.txt".to_string()],
            &base_tree,
            &worktree_files,
            &main_files,
        )
        .unwrap();

        // Only a.txt should be a real conflict; b.txt should auto-merge
        assert!(
            actual_conflicts.contains(&"a.txt".to_string()),
            "Overlapping changes should remain as conflict"
        );
        assert!(
            !actual_conflicts.contains(&"b.txt".to_string()),
            "Non-overlapping changes should be auto-resolved, not a conflict"
        );

        // b.txt in worktree should contain both changes merged
        let b_content = std::fs::read_to_string(worktree_path.join("b.txt")).unwrap();
        assert!(
            b_content.contains("SESSION_TOP"),
            "Auto-merged b.txt should contain session change"
        );
        assert!(
            b_content.contains("MAIN_BOTTOM"),
            "Auto-merged b.txt should contain main change"
        );
    }

    // =========================================================================
    // Scenario: Re-running merge after resolving conflict markers succeeds
    // =========================================================================

    #[test]
    fn test_re_merge_after_resolution_succeeds() {
        // @step Given a session worktree where conflict markers were previously written to "README.md"
        // Simulate: first merge produced markers, user has now resolved them
        let base = "line1\nline2\nThe Spec-Driven\nline4\n";

        // @step And the user has resolved the conflict markers in "README.md" by editing the worktree file
        // After resolution, the worktree has the user's chosen content
        let session_resolved = "line1\nline2\nDa Spec-Driven (v2.0)\nline4\n";

        // @step And the main worktree "README.md" matches the resolved version
        let main = "line1\nline2\nDa Spec-Driven (v2.0)\nline4\n";

        // @step When apply_session_changes is called again
        let result = three_way_merge_text(base, session_resolved, main);

        // @step Then no ConflictError should be returned
        // @step And the resolved "README.md" should be applied to the main worktree
        match result {
            MergeOutcome::Clean(content) => {
                assert!(
                    content.contains("Da Spec-Driven (v2.0)"),
                    "Resolved content should be present in clean merge"
                );
                assert!(
                    !content.contains("<<<<<<<"),
                    "Re-merge after resolution should not have conflict markers"
                );
            }
            MergeOutcome::Conflict(content) => {
                panic!(
                    "Re-merge after resolution should succeed cleanly:\n{}",
                    content
                );
            }
        }
    }
}
