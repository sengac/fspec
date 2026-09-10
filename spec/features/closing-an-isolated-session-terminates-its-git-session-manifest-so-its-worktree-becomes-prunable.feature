@done
@session-management
@WT-005
@bug-fix
@session
@git
Feature: Closing an isolated session terminates its git-session manifest so its worktree becomes prunable

  """
  Closing an isolated session (the 'Close session' exit path,
  SessionManager::destroy_session) must mark the codelet-git session
  manifest (~/.fspec/git-sessions/<id>.json) terminated=true so the
  session's worktree becomes prunable. The <project>/.fspec persistence
  manifest stays on disk (/resume parity) and the worktree directory
  stays on disk (recovery case) — closing only flips the terminated
  flag.

  Terminate-on-close is best-effort: a missing git manifest (worktree
  already merged/discarded) is a silent no-op; a failure (no home dir)
  is logged via tracing::warn and never fails the close; non-isolated
  sessions skip the step entirely.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Closing an isolated session (SessionManager::destroy_session, the 'Close session' exit path) must mark the codelet-git session manifest terminated=true (codelet_git::terminate_session) so the session becomes prunable. The <project>/.fspec persistence manifest MUST remain on disk (PARITY FIX — /resume support) and the worktree directory MUST remain on disk (recovery case) — closing only flips the terminated flag.
  #   2. Terminate-on-close is best-effort: if terminate_session fails (e.g. no home dir), destroy_session still succeeds — the error is logged via tracing::warn and never fails the close. If the git manifest is already gone (worktree was merged/discarded earlier) the step is a silent no-op. Non-isolated sessions (no worktree) skip the step entirely.
  #
  # EXAMPLES:
  #   1. User closes an isolated session whose worktree was already merged earlier via /merge-worktree (manifest already deleted by the merge): closing the session still succeeds silently — terminate-on-close finds no manifest and is a no-op.
  #   2. User closes a NON-isolated session: the close still succeeds, no git-session manifest is created or mutated for that session.
  #
  # ========================================
  Background: User Story
    As a developer using an isolated session
    I want to close the session without leaking its worktree
    So that closed sessions become prunable and I can still recover work from them

  # ===========================================
  # Terminate-on-close (SessionManager::destroy_session)
  # ===========================================
  Scenario: Closing an isolated session marks its git-session manifest terminated and keeps the worktree recoverable
    Given a git repository with one committed file
    And an isolated session created for that repository
    When I close the session via destroy_session
    Then the close succeeds
    And the git-session manifest for the session has terminated true
    And the worktree directory still exists on disk
    And the session persistence manifest still exists on disk

  Scenario: Closing an isolated session whose worktree was already merged is a silent no-op for termination
    Given a git repository with one committed file
    And an isolated session created for that repository
    And the session worktree was merged and its git-session manifest deleted
    When I close the session via destroy_session
    Then the close succeeds
    And no git-session manifest is created or left behind for the session

  Scenario: Closing a non-isolated session never touches git-session manifests
    Given a SessionManager with a non-isolated session
    When I close the session via destroy_session
    Then the close succeeds
    And no git-session manifest exists for the session

  Scenario: A terminate failure does not fail the close
    Given an isolated session whose git-session manifest cannot be updated
    When I close the session via destroy_session
    Then the close succeeds
    And the failure is logged as a warning
