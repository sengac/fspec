@done
@bug-fix
@session
@git
@rpc
@session-merge
@WT-003
Feature: Session worktree RPC ops resolve repo path via process cwd, so /merge-worktree fails when TUI was launched outside the project root

  """
  handle_impl.rs (RPC-057 surface) resolves repo_path via std::env::current_dir() at call time. Fix: session-scoped ops (merge/discard/inspect) look up the in-memory BackgroundSession and use its `project` field; when the id is not in memory they fall back to codelet_git::read_manifest(session_id).project_root before the process cwd. list/prune have no session id: they enumerate the distinct projects of all in-memory sessions (deduplicated) and run per root, aggregating results; prune's active-session set is the ids of ALL in-memory sessions (no project filter, no persisted-only entries).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. merge_session_worktree, discard_session_worktree and inspect_session_changes must resolve the repository path from the session's own recorded project root (in-memory BackgroundSession.project), NOT from std::env::current_dir() at call time
  #   2. prune_orphaned_worktrees must build its active-session set from the ids of ALL in-memory sessions (no project-path filter, no persisted-only sessions) — a session that is alive in this process must never be pruned, regardless of which process directory the TUI was launched from
  #   3. Session-scoped ops (merge/discard/inspect) with a session id that is not in memory must fall back to the git-session manifest's project_root (~/.fspec/git-sessions/<id>.json) before falling back to the process cwd, so a merge right after a restart of the TUI still targets the right repository
  #   4. list_session_worktrees and prune_orphaned_worktrees (no session id on the wire) must enumerate the distinct project roots of all in-memory sessions (deduplicated), run the git operation per root, and aggregate the results — never driven by the process cwd
  #
  # EXAMPLES:
  #   1. A session for project X has a worktree with changes; the process cwd is X/rust (a subdirectory, not a repo root); calling merge_session_worktree succeeds and the changes land in X (today it fails with NotARepository)
  #   2. A session for project X has a worktree; the process cwd is an unrelated directory (e.g. the home dir); discard_session_worktree removes X/.fspec/worktrees/<id> and its git metadata (today it fails with NotARepository)
  #   3. A session for project X has a worktree with pending changes; the process cwd is a foreign directory; inspect_session_changes returns the file/diff summary from X's worktree (today it errors with NotARepository)
  #   4. Two sessions live in different project roots (X and Y); the process cwd is foreign; list_session_worktrees returns the union of worktrees from both X and Y (today it lists only the cwd project, which is neither)
  #   5. Project X has a worktree whose session was closed (manifest terminated) while an active session lives in project Y; the process cwd is foreign; prune_orphaned_worktrees removes X's orphaned worktree and reports its session id, but Y's active session worktree is left untouched
  #
  # ========================================

  Background: User Story
    As a developer running the fspec binary from any directory
    I want to run the session-worktree RPC ops (merge, discard, inspect, list, prune)
    So that the operations target the session's own project root instead of the process cwd, so they work no matter where the TUI was launched from

  Scenario: merge_session_worktree succeeds from a subdirectory cwd
    Given an in-memory session for project X with an isolated worktree containing a modified file
    And the process cwd is X/rust, a subdirectory of the repository that is NOT a repo root itself
    When merge_session_worktree is called with the session id
    Then the merge resolves the repository from the session's project root X and returns a success outcome with the changed files
    And the modified file's new content is present in X's main working tree

  Scenario: discard_session_worktree removes the session worktree from a foreign cwd
    Given an in-memory session for project X with an isolated worktree
    And the process cwd is an unrelated directory outside any repository
    When discard_session_worktree is called with the session id
    Then discard succeeds and the session's worktree directory and its git metadata are removed from project X

  Scenario: inspect_session_changes returns the worktree summary from a foreign cwd
    Given an in-memory session for project X with an isolated worktree containing modified, added, and deleted files
    And the process cwd is an unrelated directory outside any repository
    When inspect_session_changes is called with the session id
    Then the call returns Ok with a non-zero file count and non-zero insertion/deletion counts derived from the worktree diff in project X

  Scenario: list_session_worktrees returns the union of worktrees across all session projects
    Given two in-memory isolated sessions, one rooted in project X and one in project Y, each with its own worktree
    And the process cwd is a directory that is not the root of either repository
    When list_session_worktrees is called
    Then the result contains one entry for each session worktree in both project X and project Y

  Scenario: prune_orphaned_worktrees prunes the orphan and never touches the active session
    Given project X contains a worktree whose session was closed and its git-session manifest is marked terminated
    And an active in-memory session lives in project Y with its own worktree
    And the process cwd is a directory that is not the root of either repository
    When prune_orphaned_worktrees is called
    Then the call returns the orphaned session id as pruned and the orphaned worktree directory is removed from project X
    And the active session's worktree in project Y is left untouched

  Scenario: merge resolves the repository from the git-session manifest after a TUI restart
    Given project X contains a session worktree with changes whose git-session manifest records X as its project root
    And no in-memory session exists for that worktree's session id
    And the process cwd is an unrelated directory outside any repository
    When merge_session_worktree is called with that session id
    Then the merge succeeds by resolving the repository from the manifest's project root X
    And the worktree's changes are applied to X's main working tree

