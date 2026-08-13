@done
@gitoxide-integration
@BUG-098
Feature: Merge conflict markers never written to worktree files — LLM told to resolve markers that don't exist
  """
  Implementation uses the `diffy` crate for three-way merge (diffy::merge). Add `diffy = "1"` to rust/git/Cargo.toml. The `similar` crate already present only supports two-way diff — not three-way merge with conflict markers.
  New function `write_conflict_markers()` in session_result.rs: takes base/session/main content for each conflicting file, runs diffy::merge(), writes result to worktree file. Called BEFORE returning ConflictError in apply_session_changes().
  detect_conflicts() currently compares raw bytes. For the three-way merge, we need to also consider: (1) identical changes = not a real conflict, (2) non-overlapping changes in same file = auto-merge, no conflict. diffy handles this automatically when its merge result has no conflicts.
  The TypeScript layer (conflictLlmContext.ts, mergeWorktreeHandler.ts) needs NO changes. The fix is entirely in the Rust layer — write markers into worktree files so that the existing 'read file + resolve markers' instruction to the LLM actually works.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When apply_session_changes() detects conflicting files, it must perform a three-way merge and write standard git conflict markers into the worktree files BEFORE returning ConflictError
  #   2. Conflict markers must follow standard git format: <<<<<<< session (your changes) / ======= / >>>>>>> main
  #   3. Binary files must be skipped during three-way merge — no conflict markers written for binary content
  #   4. For files added in both session and main with different content, write conflict markers with empty base (two-way conflict)
  #   5. The worktree files must be overwritten with the merged content containing conflict markers — the LLM reads from the worktree path
  #   6. Files where both session and main made identical changes (no actual conflict) should NOT get conflict markers
  #   7. The ConflictError must still be returned after writing markers — the error signals the TypeScript handler to show conflict guidance
  #
  # EXAMPLES:
  #   1. Session edits line 7 of README.md ('The' → 'Da'), main also changed line 7 ('The Spec-Driven' → 'The Spec-Driven (v2.0)') → worktree README.md gets <<<<<<< session / Da Spec-Driven / ======= / The Spec-Driven (v2.0) / >>>>>>> main
  #   2. Session edits src/app.ts (lines 10-15), main edits src/app.ts (lines 40-50, no overlap) → three-way merge succeeds cleanly, no conflict markers needed, file NOT in conflict list
  #   3. Session adds new file utils.ts, main also adds utils.ts with different content → conflict markers written with empty base (two-way diff)
  #   4. Session modifies logo.png (binary), main also modifies logo.png → binary file listed as conflicting but NO conflict markers written (binary can't be merged), error message notes binary conflicts
  #   5. Session and main both change line 7 of README.md identically ('The' → 'Da') → detect_conflicts() should NOT report this as a conflict (both made same change)
  #   6. After conflict markers are written, LLM reads the file, sees <<<<<<< markers, resolves them, saves, runs /merge-worktree again → second merge succeeds
  #
  # ========================================
  Background: User Story
    As a developer
    I want to merge worktree changes when conflicts exist
    So that see actual conflict markers in the files so I can resolve them

  # Example 1: Overlapping changes produce conflict markers
  Scenario: Conflicting text file gets standard git conflict markers written to worktree
    Given a session worktree with base commit containing "README.md"
    And the session has modified line 7 of "README.md" from "The Spec-Driven" to "Da Spec-Driven"
    And the main worktree has modified line 7 of "README.md" from "The Spec-Driven" to "The Spec-Driven (v2.0)"
    When apply_session_changes is called
    Then the worktree "README.md" should contain "<<<<<<< session (your changes)"
    And the worktree "README.md" should contain "======="
    And the worktree "README.md" should contain ">>>>>>> main"
    And the worktree "README.md" should contain "Da Spec-Driven"
    And the worktree "README.md" should contain "The Spec-Driven (v2.0)"
    And a ConflictError should be returned listing "README.md"

  # Example 2: Non-overlapping changes in same file auto-merge cleanly
  Scenario: Non-overlapping changes in same file merge cleanly without conflict markers
    Given a session worktree with base commit containing "src/app.ts"
    And the session has modified lines 10-15 of "src/app.ts"
    And the main worktree has modified lines 40-50 of "src/app.ts" with no overlap
    When apply_session_changes is called
    Then "src/app.ts" should be copied to the main worktree with both changes merged
    And no ConflictError should be returned
    And "src/app.ts" should NOT contain conflict markers

  # Example 3: Both sides add same file with different content
  Scenario: File added in both session and main with different content gets conflict markers
    Given a session worktree with base commit that does NOT contain "utils.ts"
    And the session has added "utils.ts" with content "export const x = 1;"
    And the main worktree has also added "utils.ts" with content "export const x = 2;"
    When apply_session_changes is called
    Then the worktree "utils.ts" should contain "<<<<<<< session (your changes)"
    And the worktree "utils.ts" should contain ">>>>>>> main"
    And a ConflictError should be returned listing "utils.ts"

  # Example 4: Binary files are listed as conflicting but no markers written
  Scenario: Binary file conflict is reported without writing conflict markers
    Given a session worktree with base commit containing binary file "logo.png"
    And the session has modified "logo.png" with new binary content
    And the main worktree has also modified "logo.png" with different binary content
    When apply_session_changes is called
    Then a ConflictError should be returned listing "logo.png"
    And the worktree "logo.png" should NOT contain conflict markers
    And the worktree "logo.png" should retain the session version

  # Example 5: Identical changes from both sides are not a conflict
  Scenario: Identical changes from session and main do not produce a conflict
    Given a session worktree with base commit containing "README.md"
    And the session has modified line 7 of "README.md" from "The" to "Da"
    And the main worktree has also modified line 7 of "README.md" from "The" to "Da"
    When apply_session_changes is called
    Then no ConflictError should be returned
    And "README.md" should be applied to the main worktree without conflict markers

  # Example 6: Re-merge after resolving markers succeeds
  Scenario: Re-running merge after resolving conflict markers succeeds
    Given a session worktree where conflict markers were previously written to "README.md"
    And the user has resolved the conflict markers in "README.md" by editing the worktree file
    And the main worktree "README.md" matches the resolved version
    When apply_session_changes is called again
    Then no ConflictError should be returned
    And the resolved "README.md" should be applied to the main worktree
