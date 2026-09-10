@done
@git
@session-management
@WT-011
Feature: Worktree metadata hardcodes <repo>/.git assumptions (list_worktrees HEAD path, commindir) and isolated_session.rs is dead production code
  """
  Fix is in codelet-git (worktree.rs): list_worktrees switches from repo_path.join(".git/worktrees") to open_repo(repo_path)?.git_dir().join("worktrees"). write_worktree_metadata writes the "commondir" metadata file as the relative path from worktree_git_dir back to repo.git_dir() ("../.." when the worktree git dir is <git_dir>/worktrees/<sid>) — verified the spelling was already correct (no typo); the value is structurally "../.." for both standard and linked git-dir layouts. The dead codelet_git::IsolatedSessionInfo type (isolated_session.rs) and its lib.rs re-export are deleted; its integration tests in git/tests/*.rs are rewritten against create_worktree/create_worktree_at_ref, and the napi importability test is removed. Tests create a linked-git-dir repo with a gitfile (git init in repo/ then move .git to elsewhere/repo.git and leave a gitfile) to pin the non-standard layout. Pure gitoxide - no git CLI shell-outs (worktree.rs contract).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. list_worktrees must read each worktree's HEAD from repo.git_dir().join("worktrees")/<sid>/HEAD — the same resolution create_worktree_at_ref and remove_worktree already use — so repos whose git dir is not <repo>/.git report a correct head_commit and is_detached
  #   2. write_worktree_metadata writes the "commondir" metadata file (the WT-011 attachment "commindir typo" was a false positive - the code already writes "commondir"; verified via byte inspection). The "../.." value is structurally always correct because the worktree git dir is <git_dir>/worktrees/<sid> - this scenario set pins that invariant for standard AND linked git-dir layouts
  #   3. codelet_git::IsolatedSessionInfo (git/src/isolated_session.rs) has zero production call sites and duplicates the wire codelet_rpc_types::IsolatedSessionInfo — it must be deleted along with its lib.rs re-export, and the codelet-git tests plus the napi importability test that exercise it must be rewritten to use create_worktree/create_worktree_at_ref directly
  #
  # EXAMPLES:
  #   1. Repo whose git dir is /repo/.git (standard): list_worktrees after create_worktree reports head_commit = the base commit SHA and is_detached = true; commondir resolves to /repo/.git
  #   2. Repo with a linked git dir (gitfile: repo/.git -> /elsewhere/repo.git): create_worktree writes commondir = '../..' relative to /elsewhere/repo.git/worktrees/<sid> resolving back to /elsewhere/repo.git, and list_worktrees still reports the correct head_commit
  #   3. After the cleanup, `cargo grep 'codelet_git::IsolatedSessionInfo'` finds no production-code hits — only the rpc-types wire type remains — and codelet-git + codelet-napi still build and pass their tests
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to create/list git worktrees on repos whose git dir is not <repo>/.git
    So that worktree plumbing no longer hardcodes the standard layout and the dead isolation-info type is removed

  Scenario: create_worktree writes a commondir that resolves to the real git dir
    Given a git repository whose git dir is /elsewhere/repo.git and whose repo/.git is a gitfile pointing to it
    When create_worktree is called for session "s-1"
    Then the commondir file at /elsewhere/repo.git/worktrees/s-1/commondir contains a relative path that resolves back to /elsewhere/repo.git

  Scenario: create_worktree writes the standard commondir for a standard-layout repo
    Given a git repository with the standard layout (git dir = repo/.git)
    When create_worktree is called for session "s-1"
    Then the commondir file at repo/.git/worktrees/s-1/commondir contains exactly "../.." (the standard-layout value is preserved)

  Scenario: list_worktrees reports the correct HEAD for a repo whose git dir is linked
    Given a git repository whose git dir is /elsewhere/repo.git and whose repo/.git is a gitfile pointing to it, with a session worktree for session "s-1" already created via create_worktree
    When list_worktrees is called against the repository root
    Then list_worktrees reports the worktree for session "s-1" with head_commit equal to the base commit SHA and is_detached true (not "unknown")

  Scenario: list_worktrees still reports the correct HEAD for a standard-layout repo
    Given a git repository with the standard layout (git dir = repo/.git), with a session worktree for session "s-1" already created via create_worktree
    When list_worktrees is called against the repository root
    Then list_worktrees reports the worktree for session "s-1" with head_commit equal to the base commit SHA and is_detached true

  Scenario: the codelet-git test suite passes without IsolatedSessionInfo
    Given the worktree-metadata fix has been applied (isolated_session.rs deleted)
    When the codelet-git test suite is run
    Then all git tests pass with create_worktree/create_worktree_at_ref as the only isolation-info path (IsolatedSessionInfo is gone)
