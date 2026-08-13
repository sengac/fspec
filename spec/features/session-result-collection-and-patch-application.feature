@codelet
@session-management
@GIT-015
Feature: Session result collection and patch application
  """
  API in rust/git/src/worktree.rs:
  - get_session_diff(repo_path, session_id) → SessionResult (for review, no side effects)
  - apply_session_changes(repo_path, session_id) → applies changes and removes worktree
  - abort_session(repo_path, session_id) → removes worktree without applying

  Expose NAPI bindings in rust/napi/src/git.rs for TypeScript consumption.
  Also expose GIT-014 worktree primitives (createWorktree, removeWorktree, listWorktrees) that weren't bound yet.

  SessionResult struct: session_id, diff (unified format), files_changed, files_added, files_deleted, base_commit

  Diff compares base_commit tree against worktree WORKING DIRECTORY (captures uncommitted changes).
  Reuses existing diff.rs infrastructure.

  NOTE: worktree-merge-research.md describes complex gix-merge approach that was NOT chosen.
  Actual implementation is simpler: file-based diff/copy, not git merge.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. We don't need complex merge operations. Session worktrees use detached HEAD. When a session completes, we collect the diff and apply it to the main worktree. The user decides what to keep. No git merge needed - just diff + file copy.
  #   2. MUST use pure gitoxide (gix) Rust implementation - NO git CLI commands allowed anywhere
  #   3. Diff collection compares base_commit tree against worktree WORKING DIRECTORY (captures uncommitted changes)
  #   4. Apply copies files directly from session worktree to main worktree - not patch/merge operations
  #   5. get_session_diff returns the diff for user review without any side effects
  #   6. Abort session discards worktree changes and removes the worktree without affecting main repository
  #   7. Conflict detection happens at apply time - files modified in both worktrees return error with conflict details
  #   8. NAPI bindings must be exposed for TypeScript consumption of all session result operations
  #
  # EXAMPLES:
  #   1. Getting session diff with changes returns unified diff showing modified, added, and deleted files
  #   2. Applying session changes copies modified files from session worktree to main worktree
  #   3. Applying session changes copies added files from session worktree to main worktree
  #   4. Applying session changes removes deleted files from main worktree
  #   5. Applying session changes when main has diverged returns conflict error with list of conflicting files
  #   6. Aborting a session removes the worktree directory and git metadata without modifying main repository
  #   7. Getting session diff with no changes returns empty diff and zero counts
  #   8. Getting session diff includes binary file indicators without actual content for binary files
  #
  # QUESTIONS (ANSWERED):
  #   Q: If we adopt Ghost Commits (like Codex), merge operations become unnecessary since ghost commits are restore-only. Should we pivot away from worktree merge complexity?
  #   A: We don't need complex merge operations. Session worktrees use detached HEAD. When a session completes, we collect the diff and apply it to the main worktree as a patch. The user decides what to keep. No git merge needed - just diff + apply.
  #
  # ========================================
  Background: User Story
    As a parent agent
    I want to collect and apply changes from child session worktrees
    So that I can review and integrate work from isolated parallel sessions back into the main repository

  # ========================================
  # SCENARIOS
  # ========================================
  @happy-path
  Scenario: Get session diff with changes
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-001/"
    And the session worktree has modified file "src/main.rs"
    And the session worktree has added file "src/new-feature.rs"
    And the session worktree has deleted file "src/deprecated.rs"
    When I get the session diff for "session-001"
    Then I should receive a SessionResult
    And the SessionResult should contain a unified diff
    And the diff should show "src/main.rs" as modified
    And the diff should show "src/new-feature.rs" as added
    And the diff should show "src/deprecated.rs" as deleted
    And the SessionResult should contain the base_commit
    And the SessionResult should contain files_changed count
    And the SessionResult should contain files_added count
    And the SessionResult should contain files_deleted count
    And the session worktree should still exist

  @happy-path
  Scenario: Apply session changes copies modified files
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-002/"
    And the session worktree has modified file "src/lib.rs" with content "updated content"
    And the main repository "src/lib.rs" has not changed since session creation
    When I apply session changes from "session-002" to main worktree
    Then the main repository "src/lib.rs" should contain "updated content"
    And the session worktree should be removed
    And the git worktree metadata should be cleaned up

  @happy-path
  Scenario: Apply session changes copies added files
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-002a/"
    And the session worktree has added file "src/new-module.rs" with content "new code"
    When I apply session changes from "session-002a" to main worktree
    Then the main repository should have file "src/new-module.rs"
    And the main repository "src/new-module.rs" should contain "new code"
    And the session worktree should be removed

  @happy-path
  Scenario: Apply session changes removes deleted files
    Given I have a git repository with commits
    And the main repository has file "src/old-module.rs"
    And a session worktree exists at ".fspec/worktrees/session-002b/"
    And the session worktree has deleted file "src/old-module.rs"
    When I apply session changes from "session-002b" to main worktree
    Then the main repository should NOT have file "src/old-module.rs"
    And the session worktree should be removed

  @error-handling
  Scenario: Apply session changes when main has diverged
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-003/" based on commit "abc123"
    And the session worktree has modified file "src/conflict.rs"
    And the main repository has also modified "src/conflict.rs" since commit "abc123"
    When I attempt to apply session changes from "session-003"
    Then I should receive a conflict error
    And the error should list "src/conflict.rs" as conflicting
    And the session worktree should NOT be removed
    And the main repository should be unchanged

  @happy-path
  Scenario: Abort session discards changes
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-004/"
    And the session worktree has modified file "src/work-in-progress.rs"
    When I abort the session "session-004"
    Then the ".fspec/worktrees/session-004/" directory should not exist
    And the git worktree metadata should be cleaned up
    And the main repository should be unchanged

  @happy-path
  Scenario: Get session diff with no changes
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-005/"
    And the session worktree has no changes from base_commit
    When I get the session diff for "session-005"
    Then I should receive a SessionResult
    And the SessionResult diff should be empty
    And the SessionResult files_changed should be 0
    And the SessionResult files_added should be 0
    And the SessionResult files_deleted should be 0

  @happy-path
  Scenario: Session diff handles binary files
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-006/"
    And the session worktree has added binary file "assets/image.png"
    When I get the session diff for "session-006"
    Then the diff should indicate "assets/image.png" is a binary file
    And the diff should NOT contain binary content

  @integration
  Scenario: Get session diff without applying
    Given I have a git repository with commits
    And a session worktree exists at ".fspec/worktrees/session-007/"
    And the session worktree has multiple changes
    When I get the session diff for "session-007"
    Then I should receive a unified diff
    And the session worktree should still exist
    And the main repository should be unchanged

  @error-handling
  Scenario: Fail gracefully when session does not exist
    Given I have a git repository with commits
    And no session worktree exists for "nonexistent-session"
    When I attempt to get session diff for "nonexistent-session"
    Then I should receive an error indicating worktree not found

  @error-handling
  Scenario: Fail gracefully when applying non-existent session
    Given I have a git repository with commits
    And no session worktree exists for "nonexistent-session"
    When I attempt to apply session changes from "nonexistent-session"
    Then I should receive an error indicating worktree not found

  @napi
  Scenario: NAPI binding exposes getSessionDiff
    Given the codelet-napi module is loaded
    And a session worktree exists at ".fspec/worktrees/napi-test-1/"
    When I call getSessionDiff via NAPI with session ID "napi-test-1"
    Then I should receive a SessionResult object with diff property

  @napi
  Scenario: NAPI binding exposes applySessionChanges
    Given the codelet-napi module is loaded
    And a session worktree exists at ".fspec/worktrees/napi-test-2/"
    And the session worktree has modified file "src/test.rs"
    When I call applySessionChanges via NAPI with session ID "napi-test-2"
    Then the changes should be applied to the main worktree
    And the session worktree should be removed

  @napi
  Scenario: NAPI binding exposes abortSession
    Given the codelet-napi module is loaded
    And a session worktree exists at ".fspec/worktrees/napi-test-3/"
    When I call abortSession via NAPI with session ID "napi-test-3"
    Then the session worktree should be removed
    And the main repository should be unchanged
