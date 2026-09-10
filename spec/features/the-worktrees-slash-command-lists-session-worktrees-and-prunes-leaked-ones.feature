@done
@session-management
@WT-005
@bug-fix
@session
@git
@tui
@dialog
@session-merge
Feature: The /worktrees slash command lists session worktrees and prunes leaked ones

  """
  The TUI gains a /worktrees slash command that lists session
  worktrees (backend.list_session_worktrees) in a read-only
  SessionWorktreesDialog overlay with a Prune action calling
  backend.prune_orphaned_worktrees, reporting the outcome as a
  scrollback notice.

  The /worktrees listing overlay offers a 'Prune' action that calls
  backend.prune_orphaned_worktrees() and reports the outcome: on
  success with N>0 pruned ids emit '[worktrees] pruned N leaked
  worktree(s)'; on success with 0 pruned emit '[worktrees] no leaked
  worktrees to prune'; on error emit '[error] /worktrees prune: {e}'.
  Pruning never touches in-memory (active) sessions — the backend
  already excludes them via the active-set.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The /worktrees listing overlay must offer a 'Prune' action that calls backend.prune_orphaned_worktrees() and reports the outcome: on success with N>0 pruned ids emit '[worktrees] pruned N leaked worktree(s)'; on success with 0 pruned emit '[worktrees] no leaked worktrees to prune'; on error emit '[error] /worktrees prune: {e}'. Pruning never touches in-memory (active) sessions — the backend already excludes them via the active-set.
  #   2. A new TUI slash command /worktrees (registry entry 'worktrees', description 'List session worktrees and prune leaked ones') routes through the existing RPC surface: backend.list_session_worktrees() populates a read-only listing overlay (SessionWorktreesDialog) showing one row per worktree with session id (active/dirty markers) and worktree path; it does NOT mutate anything.
  #
  # EXAMPLES:
  #   1. User closes an isolated session without merging, then runs /worktrees and sees that session's worktree listed (session id, dirty marker, path); pressing Prune removes it and the conversation shows '[worktrees] pruned 1 leaked worktree(s)'.
  #
  # ========================================
  Background: User Story
    As a developer using an isolated session
    I want to see and clean up leftover worktrees
    So that I don't accumulate stale worktree directories

  # ===========================================
  # /worktrees listing (TUI)
  # ===========================================
  Scenario: The /worktrees slash command lists session worktrees
    Given a repository with two session worktrees for closed sessions
    And a TUI app running with a session
    When I run "/worktrees"
    Then the session worktrees dialog is shown
    And the dialog lists one row per worktree with the session id and worktree path
    And dirty worktrees are marked dirty

  Scenario: The /worktrees slash command with no worktrees shows an empty-state row
    Given a repository with no session worktrees
    And a TUI app running with a session
    When I run "/worktrees"
    Then the session worktrees dialog is shown
    And the dialog shows an empty-state row instead of worktree rows

  # ===========================================
  # Prune (TUI)
  # ===========================================
  Scenario: Pruning from the /worktrees dialog removes leaked worktrees
    Given a repository with one leaked session worktree for a terminated session
    And a TUI app running with the session worktrees dialog open
    When I press Prune in the dialog
    Then the worktree directory is removed from the repository
    And I see the notice "[worktrees] pruned 1 leaked worktree(s)"

  Scenario: Pruning when nothing is leaked reports no leaked worktrees
    Given a repository with no leaked session worktrees
    And a TUI app running with the session worktrees dialog open
    When I press Prune in the dialog
    Then I see the notice "[worktrees] no leaked worktrees to prune"

  Scenario: Prune failures surface an error notice
    Given a TUI app running with the session worktrees dialog open
    And the backend prune call fails with "boom"
    When I press Prune in the dialog
    Then I see the notice "[error] /worktrees prune: boom"
