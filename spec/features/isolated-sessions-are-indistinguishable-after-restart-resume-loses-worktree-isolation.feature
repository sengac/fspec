@done
@wt-007
@session
@git
@session-management
@bug-fix
@rust
@WT-007
Feature: Isolated sessions are indistinguishable after restart — /resume loses worktree isolation

  """
  create_session_from_manifest reads the codelet-git session manifest via codelet_git::read_manifest(uuid) and, when worktree_path + the <repo>/.git/worktrees/<id> admin dir both exist, populates SessionCreationParams { worktree_path, base_commit, isolation: Some(IsolationContext) } and emits IsolationStateChange(true, worktree, base) after insert, mirroring create_isolated_session_with_id. When the worktree is gone, it marks the git manifest terminated (WT-005 prune hook) and resumes non-isolated.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A resumed isolated session gets exactly the isolation surface the live create path gets: worktree_path + base_commit on the BackgroundSession, IsolationContext injected into environment reminders, IsolationStateChange(true, worktree, base) chunk emitted after insert, and the footer poller started with the worktree cwd
  #   2. Re-isolation is only applied when BOTH the worktree directory and the git worktree admin dir (<repo>/.git/worktrees/<id>) still exist; if either is gone the session resumes non-isolated with a warning log and the git manifest is marked terminated so /worktrees prune can reclaim strays
  #   3. On resume (create_session_from_manifest), the codelet-git session manifest at ~/.fspec/git-sessions/<id>.json is consulted: if it exists and has a worktree_path, that path is the candidate for re-isolating the session
  #   4. Resuming a session whose git manifest has no worktree_path (a never-isolated session) resumes exactly as today — non-isolated, no extra chunks, no behavior change
  #
  # EXAMPLES:
  #   1. A session created isolated (worktree at .fspec-worktrees/<id>, manifest at ~/.fspec/git-sessions/<id>.json with worktree_path) → restart → /resume <id> → the BackgroundSession has worktree_path = Some, base_commit = Some, an IsolationStateChange(true, worktree, base) chunk is broadcast, and the footer poller runs with the worktree cwd
  #   2. A formerly-isolated session whose worktree dir was manually deleted (but the git manifest still claims a worktree_path) → /resume → the session resumes non-isolated, a warning is logged, and the git manifest is marked terminated=true so /worktrees prune can reclaim it
  #   3. A never-isolated session (no git manifest, or git manifest with worktree_path = null) → /resume → resumes non-isolated exactly as before the fix (IsolationStateChange(false, None), footer poller with project cwd)
  #
  # ========================================

  Background: User Story
    As a developer using isolated sessions
    I want to resume a formerly-isolated session after a restart
    So that the session comes back isolated in its worktree instead of silently reattaching to the main project

  Scenario: Resuming a formerly-isolated session restores worktree isolation
    Given a temp git repo with a persisted session manifest for an isolated session (git manifest at git-sessions/<id>.json with worktree_path and base_commit, and both the worktree dir and the .git/worktrees/<id> admin dir exist)
    When the manager resumes the session via create_session_from_manifest
    Then the resumed session carries worktree_path and base_commit and the IsolationStateChange(true, worktree, base) chunk is broadcast


  Scenario: Resuming an isolated session whose worktree was deleted falls back to non-isolated
    Given a persisted isolated session whose git manifest still claims a worktree_path, but the worktree directory has been deleted
    When the manager resumes the session via create_session_from_manifest
    Then the session resumes non-isolated with a warning and the git manifest is marked terminated so /worktrees prune can reclaim it


  Scenario: Resuming a never-isolated session keeps the non-isolated resume behavior
    Given a persisted session with no git-session manifest (or one whose worktree_path is null)
    When the manager resumes the session via create_session_from_manifest
    Then the session resumes non-isolated exactly as before (IsolationStateChange(false, None), footer poller with the project cwd)

