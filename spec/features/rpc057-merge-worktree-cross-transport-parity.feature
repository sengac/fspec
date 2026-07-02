@done
@tui
@RPC-057
@rpc
@rust
@parity
Feature: /merge-worktree cross-transport parity
  """
  Both EmbeddedFspecBackend (in-process embedded transport) and
  WebSocketFspecBackend (tarpc over WebSocket) must land identically on
  the same StubSessionManagerHandle for every new RPC method introduced
  by RPC-057:

  * merge_session_worktree
  * discard_session_worktree
  * prune_orphaned_worktrees
  * list_session_worktrees
  * inspect_session_changes

  Mirrors the RPC-049 / RPC-050 / RPC-054 / RPC-055 / RPC-056
  cross-transport parity tests — each transport invocation increments
  the same per-stub counter and returns the same payload.
  """

  Background: User Story
    As a developer porting the AgentView to Rust
    I want both transports to land identically on the SessionManagerHandle for the /merge-worktree RPCs
    So that the WebSocket and embedded paths cannot diverge as the feature grows

  Scenario: Embedded and WebSocket merge_session_worktree both reach the stub
    Given a StubSessionManagerHandle seeded with a MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When merge_session_worktree is called via the embedded transport with session_id "s-1" and MergeStrategy::FastForward
    And merge_session_worktree is called via the WebSocket transport with session_id "s-1" and MergeStrategy::FastForward
    Then the stub's merge_session_worktree_calls counter equals 2
    And both calls return MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") }

  Scenario: Embedded and WebSocket discard_session_worktree both reach the stub
    Given a StubSessionManagerHandle seeded to return Ok(()) for discard_session_worktree behind both transports
    When discard_session_worktree is called via the embedded transport with session_id "s-1"
    And discard_session_worktree is called via the WebSocket transport with session_id "s-1"
    Then the stub's discard_session_worktree_calls counter equals 2
    And both calls return Ok(())

  Scenario: Embedded and WebSocket prune_orphaned_worktrees both reach the stub
    Given a StubSessionManagerHandle seeded with pruned session ids ["sess-a", "sess-b"] behind both transports
    When prune_orphaned_worktrees is called via the embedded transport
    And prune_orphaned_worktrees is called via the WebSocket transport
    Then the stub's prune_orphaned_worktrees_calls counter equals 2
    And both calls return ["sess-a", "sess-b"]

  Scenario: Embedded and WebSocket list_session_worktrees both reach the stub
    Given a StubSessionManagerHandle seeded with two SessionWorktreeInfo rows behind both transports
    When list_session_worktrees is called via the embedded transport
    And list_session_worktrees is called via the WebSocket transport
    Then the stub's list_session_worktrees_calls counter equals 2
    And both calls return a Vec of length 2
    And each entry has identical session_id, worktree_path, base_commit, head_commit, dirty fields across the two transports

  Scenario: Embedded and WebSocket inspect_session_changes both reach the stub
    Given a StubSessionManagerHandle seeded with SessionChangesSummary { files_changed: 3, insertions: 12, deletions: 5, commits: ["abc1234"] } behind both transports
    When inspect_session_changes is called via the embedded transport with session_id "s-1"
    And inspect_session_changes is called via the WebSocket transport with session_id "s-1"
    Then the stub's inspect_session_changes_calls counter equals 2
    And both calls return SessionChangesSummary { files_changed: 3, insertions: 12, deletions: 5, commits: ["abc1234"] }
