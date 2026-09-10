@done
@wt-008
@git
@session
@session-management
@bug-fix
@rust
@WT-008
Feature: /merge-worktree does not commit anything and the 'FastForward' strategy is silently ignored

  """
  merge_session (codelet-git) gains a strategy gate: non-FastForward strategies return Err before any copy. On success, after apply_session_changes, a commit is created in the MAIN repo by building a tree from the post-copy worktree snapshot (snapshot-aware: blobs + exec bits, symlink Link entries, gitlink Commit entries — gitignored files excluded), parented on HEAD, authored from repo user.name/user.email (fallback fspec/fspec@local), message 'fspec: merge session <id>', and the SHA is returned in MergeResult.merge_commit which surfaces through MergeOutcome.merge_commit. A merge with no tracked changes commits nothing (merge_commit None).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A successful /merge-worktree commits the merged changes in the MAIN repo (git add -A + commit) after the working-tree copy succeeds, and MergeOutcome.merge_commit carries the new commit SHA so the TUI notice can render it
  #   2. Only MergeStrategy::FastForward is supported; any other strategy (Squash, ThreeWay) returns an explicit error (MergeStatus/Err UnsupportedStrategy) instead of silently behaving as FastForward
  #   3. The merge commit uses the repo's git user.name/user.email config when set, and falls back to a fixed fspec identity otherwise — never fails a merge because the repo lacks identity config
  #   4. The merge commit message is 'fspec: merge session <session-id>' and is authored/committed at merge time; the commit is created in the MAIN repo and becomes the new HEAD of the checked-out branch
  #
  # EXAMPLES:
  #   1. Merge an isolated session with tracked changes using the default FastForward strategy → the changes are committed in the main repo, the success notice shows the new commit SHA, and the main working tree is clean (git status clean) after the merge
  #   2. Merge an isolated session requesting the Squash strategy → an explicit 'unsupported strategy' error is returned and nothing is committed, copied, or merged
  #   3. Merge an isolated session whose only delta is gitignored build artifacts (no tracked changes) → the merge reports nothing to merge and does NOT create a commit (no empty commit is produced)
  #   4. Merge a session in a repo with user.name/user.email set in git config → the merge commit is authored by that identity; in a repo with no identity config the commit still succeeds with a fallback fspec identity
  #
  # ========================================

  Background: User Story
    As a developer using isolated sessions
    I want to merge an isolated session back into main with /merge-worktree
    So that the merge commits the changes to a real commit (SHA surfaced in the result) and unsupported strategies are rejected instead of silently behaving as the default

  Scenario: Merging a session with tracked changes commits them to main
    Given a temp git repo with one committed file, and an isolated session that has committed a modification to that file in its worktree
    When the session is merged with the default FastForward strategy
    Then the merge reports success with a real merge commit SHA and the main repo's HEAD points at a new commit whose message is 'fspec: merge session <id>' and whose tree contains the session's change, with the main working tree clean


  Scenario: Requesting an unsupported merge strategy is rejected before anything is applied
    Given a temp git repo with one committed file, and an isolated session with a committed change in its worktree
    When the session is merged requesting the Squash strategy
    Then the merge returns an explicit unsupported-strategy error and nothing is committed, copied, or merged (the main repo HEAD is unchanged and the session worktree is intact)


  Scenario: Merging a session with only ignored changes creates no commit
    Given a temp git repo with one committed file, and an isolated session whose worktree only contains gitignored build artifacts (no tracked changes)
    When the session is merged with the default FastForward strategy
    Then the merge reports nothing-to-merge (NoChanges), creates no commit (the main repo HEAD is unchanged), and surfaces no merge commit SHA


  Scenario: The merge commit uses the repo's configured identity
    Given a temp git repo with user.name and user.email set in its git config, and an isolated session with a committed change in its worktree
    When the session is merged with the default FastForward strategy
    Then the merge commit is authored by the configured user.name/user.email

