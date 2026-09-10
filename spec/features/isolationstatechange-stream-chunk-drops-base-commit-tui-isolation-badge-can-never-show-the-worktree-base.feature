@done
@session-creation
@tui-component
@bug-fix
@session
@WT-001
Feature: IsolationStateChange stream chunk drops base_commit — TUI isolation badge can never show the worktree base
  """
  Fix is a one-line change in create_isolated_session_with_id (rust/sessions/src/session_manager.rs): switch the IsolationStateChange emit from StreamChunk::isolation_state_change (2-arg, hardcodes base_commit: None) to StreamChunk::isolation_state_change_with_base, threading the base_commit already held in the function (from codelet_git::create_worktree and returned in IsolatedSessionInfo).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. create_isolated_session_with_id must emit IsolationStateChange via StreamChunk::isolation_state_change_with_base(true, Some(worktree_path), Some(base_commit)), threading the base_commit returned by codelet_git::create_worktree
  #   2. De-isolation emit sites (isolation_state_change(false, None)) stay on the 2-arg constructor — when a session is no longer isolated there is no base commit to carry
  #
  # EXAMPLES:
  #   1. SessionManager::create_isolated_session_with_id creates a worktree forked at HEAD abc123; the IsolationStateChange chunk sent on chunks_tx has base_commit == Some("abc123") and the same SHA is returned in IsolatedSessionInfo.base_commit
  #
  # ========================================
  Background: User Story
    As a fspec TUI developer
    I want to see the IsolationStateChange chunk carry the worktree's base commit when an isolated session is created
    So that the TUI isolation badge and any client logic can show which commit the worktree was forked from without a second round-trip

  Scenario: Isolated session creation chunk carries the worktree base commit
    Given a fresh git repository with one committed file
    When an isolated session is created via create_isolated_session_with_id against that repository
    Then the IsolationStateChange chunk received on the chunks broadcast has is_isolated true, the worktree path, and a base_commit equal to the repository's HEAD at fork time

  Scenario: Non-isolated session creation still emits a base_commit-free IsolationStateChange
    Given a SessionManager with a subscriber on the chunks broadcast
    When a non-isolated session is created via create_session_with_id
    Then the IsolationStateChange chunk received on the chunks broadcast has is_isolated false, no worktree path, and base_commit None
