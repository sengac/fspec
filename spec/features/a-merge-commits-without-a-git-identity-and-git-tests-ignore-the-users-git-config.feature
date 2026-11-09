@BUG-582
@done
@git
@bug-fix
@rust
@session-merge
@BUG-599
Feature: codelet-git wt008 tests depend on the user's global git identity
  """
  codelet-git: one helper advances a branch after a commit (lib.rs advance_head) through Repository::edit_references_as with a RefEdit Change::Update {log: RefLog::AndReference, expected: MustExistAndMatch(current target), new: the commit}, passing the commit's committer signature for the reflog, so no reflog write reads the committer from git config. git_commit_signature (the session merge commit) and repo_ops commit use it. Tests: rust/git/tests/bug599_merge_without_git_identity.rs runs in a process whose HOME and XDG_CONFIG_HOME are empty temp dirs and which sets GIT_CONFIG_NOSYSTEM=1 and GIT_CONFIG_GLOBAL=/dev/null; every git test that spawns git sets those two variables on the spawn; rust/git/tests/bug599_git_tests_ignore_user_config.rs scans rust/git/tests for a Command::new("git") that does not.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Advancing a branch after a commit writes its reflog entry with the commit's own committer identity, never an identity looked up in git config; the session merge commit and every other codelet-git commit do this
  #   2. Every codelet-git test that spawns git ignores the user's and the system's git config: the spawn sets GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1, and a test that commits sets a repo-local identity
  #   3. A source scan fails on any codelet-git test that spawns git without both variables, naming the file and line
  #
  # EXAMPLES:
  #   1. With HOME and XDG_CONFIG_HOME empty and GIT_CONFIG_NOSYSTEM=1, merging a session with tracked changes commits to main, and HEAD's newest reflog entry names the merge commit's committer
  #   2. The repo has user.name 'Ada' in .git/config: the merge commit and its reflog entry both name Ada
  #   3. A test in rust/git/tests calling Command::new("git") directly is reported by the scan as <file>:<line>
  #
  # ========================================
  Background: User Story
    As a developer merging an isolated session on a machine with no git identity
    I want to have the merge commit and its reflog entry written with the identity the merge resolved
    So that a merge never fails because git config has no user, and codelet-git's tests never depend on the user's git config

  Scenario: A merge commits on a machine with no git identity
    Given a process whose HOME and XDG_CONFIG_HOME are empty and which reads no system git config
    And a session worktree with a tracked change and a repository with no user identity
    When the session is merged into main
    Then the merge succeeds and main's HEAD is the merge commit
    And HEAD's newest reflog entry names the merge commit's committer

  Scenario: The reflog names the repository's own identity
    Given a repository whose .git/config sets user.name "Ada" and user.email "ada@example.com"
    And a session worktree with a tracked change
    When the session is merged into main
    Then the merge commit's committer is Ada
    And HEAD's newest reflog entry names Ada

  Scenario: Git tests spawn git only through the isolating helper
    Given the codelet-git test sources
    When they are scanned for processes spawning git
    Then every git process sets GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1
