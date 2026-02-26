@done
@GIT-036
Feature: Merge worktree slash command with auto-close session workflow
  """
  Architecture:

  Handler: AgentView.tsx handleSubmit, slash command code path (~line 3095).
  Pattern: check `userMessage === '/merge-worktree'`, setInputValue('').

  Handler flow:
  1. Check `isIsolated` from useIsIsolated() sessionStore hook.
  If false → setConversation type:'status' error, return.
  2. Call inspectSessionChanges(currentProjectRef.current, currentSessionId) → SessionResultJs.
  SessionResultJs: { sessionId, diff, filesChanged[], filesAdded[], filesDeleted[], baseCommit }.
  If all three file arrays empty → setConversation type:'status' "Nothing to merge", return.
  3. Call mergeSessionChanges(currentProjectRef.current, currentSessionId) → MergeResultJs.
  MergeResultJs: { sessionId, filesModified[], filesAdded[], filesDeleted[] }.
  On success: show merge summary, then cleanupCurrentSessionHandler() + destroySession() + onExit().
  On error: check error.message for 'Conflict', show conflict file list, session stays open.

  Rust merge algorithm (codelet_git::merge_session in session_status.rs):
  1. get_session_diff() compares base_commit tree vs worktree working directory.
  2. apply_session_changes() detects conflicts (file modified in both session AND main since base_commit,
  or new file in session that exists in main with different content).
  No conflicts: copies modified/added files to main, removes deleted files, removes worktree dir.
  Conflicts: throws GitError::ConflictError { files } with worktree intact.
  3. delete_manifest() removes ~/.fspec/git-sessions/<session-id>.json.

  Conflict error format reaching TypeScript:
  "Conflict detected: [\"file1.ts\", \"file2.ts\"] have been modified in both session and main worktree"

  Two separate cleanups on success:
  - mergeSession() NAPI removes the git worktree directory and session manifest.
  - destroySession() destroys the Rust BackgroundSession, detaches work unit, clears sessionStore,
  unsubscribes from GlobalSessionStreamManager.

  Inspect-before-merge is required because mergeSession() would succeed and remove the
  worktree even with no changes. inspectSessionChanges() is the gate for rule [9].

  NAPI wrappers (from sessionService.ts):
  - inspectSessionChanges(repoPath, sessionId) → wraps inspectSession() NAPI
  - mergeSessionChanges(repoPath, sessionId) → wraps mergeSession() NAPI

  Removal targets:
  - slashCommands.ts: remove 'sessions' entry, add 'merge-worktree' entry
  - AgentView.tsx: remove /sessions handler, showSessionManagementPanel state,
  SessionManagementPanel import, and its render block
  - Delete SessionManagementPanel.tsx and its test files
  """

  Background: User Story
    As a developer using an isolated session
    I want to merge my worktree changes back to the main project and close the session in one step
    So that I have a clear, intent-driven workflow to finish my isolated work

  # --- /merge-worktree command: happy path ---
  Scenario: Successful merge closes session and returns to board
    Given I am in an active isolated session with modified files
    When I type "/merge-worktree"
    Then inspectSessionChanges should be called to check for changes
    And mergeSessionChanges should be called to apply changes to the main worktree
    And I should see a merge summary status message showing counts of files modified, added, and deleted
    And cleanupCurrentSessionHandler should be called
    And destroySession should be called with the current session ID
    And onExit should be called to return to the board view

  # --- /merge-worktree command: error paths ---
  Scenario: Merge worktree in non-isolated session shows error
    Given I am in an active session that is not isolated
    When I type "/merge-worktree"
    Then I should see a status message "This command is only available in isolated sessions"
    And no merge or inspect calls should be made
    And the session should remain active

  Scenario: Merge worktree with no changes shows nothing to merge
    Given I am in an active isolated session with no modified files
    When I type "/merge-worktree"
    Then inspectSessionChanges should be called and return empty file arrays
    And I should see a status message "Nothing to merge"
    And mergeSessionChanges should not be called
    And the session should remain active

  Scenario: Merge worktree with conflicts keeps session open
    Given I am in an active isolated session with modified files
    And the main worktree has conflicting changes to the same files
    When I type "/merge-worktree"
    Then inspectSessionChanges should be called to check for changes
    And mergeSessionChanges should be called and throw a Conflict error
    And I should see a status message listing the conflicting file paths
    And destroySession should not be called
    And the session should remain active for conflict resolution

  # --- Slash command registry ---
  Scenario: /merge-worktree command is registered in slash command registry
    Given the slash command registry in slashCommands.ts
    Then the "merge-worktree" command should be in the SLASH_COMMANDS array
    And its description should be "Merge worktree changes and close session"
    And it should not have requiresSession set to false

  Scenario: /sessions command is removed from slash command registry
    Given the slash command registry in slashCommands.ts
    Then no entry with name "sessions" should exist in the SLASH_COMMANDS array

  # --- Component removal ---
  Scenario: SessionManagementPanel component and tests are removed
    Given the codebase after this change
    Then the file "src/tui/components/SessionManagementPanel.tsx" should not exist
    And the file "src/tui/components/__tests__/SessionManagementPanel.test.tsx" should not exist
    And the file "src/tui/components/__tests__/SessionManagementPanelKeyboard.test.tsx" should not exist
    And AgentView should not import SessionManagementPanel
    And AgentView should not contain showSessionManagementPanel state
    And AgentView should not contain a render block for SessionManagementPanel
