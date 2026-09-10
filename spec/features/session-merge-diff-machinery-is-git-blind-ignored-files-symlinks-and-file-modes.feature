@done
@WT-006
@wt-006
@git
@session-merge
@bug-fix
@rust
Feature: Session merge/diff machinery is git-blind: ignored files, symlinks, and file modes
  """
  codelet-git's session diff/merge pipeline operated on raw directory
  scans instead of git state, which produced three classes of defects
  (audit: spec/attachments/WT-006/wt-006.md):

  (a) Gitignored untracked files were silently invisible. An isolated
      session that produced build artifacts (target/, node_modules/,
      logs) reported "nothing to merge" while the worktree was full of
      new files, and the merge-side scan used for conflict detection
      was missing those files from its picture of main.

  (b) Symlinks could not round-trip. get_tree_files dropped Link tree
      entries, so the diff/apply/merge machinery treated every
      committed symlink as "deleted in the worktree" and the merge
      deletion sweep removed the symlink from the main worktree.

  (c) File modes were flattened. build_tree_from_index /
      build_tree_from_files only preserved the executable flag and
      ghost checkpoints recorded every non-executable file as 0o644.

  Fix: the diff/merge/checkpoint machinery now works on a richer
  tree snapshot that distinguishes blobs, symlinks (by target),
  gitlinks (atomically — never deleted, never materialized), and
  the executable bit, and get_session_diff reports gitignored
  untracked files in a dedicated `files_ignored` list so "nothing to
  merge" is honest about ignored artifacts. Merge and checkpoints
  apply only git-representable (tracked or stageable) changes —
  gitignored files are never copied, overwritten, or deleted in the
  main worktree as a side effect of a merge.

  Out of scope (separate defects): git_add/git_commit inside a
  linked worktree (hardcoded .git/index joins), and full submodule
  round-tripping (gitlinks are treated atomically only).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. get_session_diff / inspect_session_changes report gitignored untracked files
  #      in a separate files_ignored list: they never count toward
  #      files_changed/added/deleted, never appear in the unified diff,
  #      and never affect merge conflict detection (merge stays tracked-only),
  #      so "nothing to merge" is honest about ignored artifacts
  #   2. Merging a session applies only git-representable (tracked or stageable)
  #      changes: gitignored files are never copied, overwritten, or deleted in the
  #      main worktree as a side effect of a merge
  #   3. Symlinks are first-class in worktree creation and the session diff/merge
  #      pipeline: committed symlink tree entries are materialized as real symlinks
  #      in the worktree, the diff treats them as changed/deleted when their target
  #      path changes, and merging back to main recreates the symlink (not a plain
  #      file containing the target text)
  #   4. The executable bit is preserved across session merge and ghost-checkpoint
  #      round trips — git's full blob-mode space is 100644/100755, so "mode
  #      preservation" means the executable flag; non-executable blobs are
  #      normalized to 0o644 by git convention
  #
  # EXAMPLES:
  #   1. A worktree with only build artifacts (a .gitignored target/ dir with
  #      files) → get_session_diff reports files_changed/added/deleted empty AND
  #      files_ignored listing the artifacts; the /merge-worktree inspect reports
  #      0 files changed so the TUI says 'nothing to merge' honestly
  #   2. A worktree with an executable script (mode 100755) committed at base,
  #      unchanged in the session, merged to main → the merged file keeps its
  #      executable bit
  #   3. A repo with a committed symlink 'config/link' → 'real/target' → create an
  #      isolated session, modify a regular file in it and merge → 'config/link'
  #      in main is still a symlink pointing at 'real/target' (not a plain text
  #      file containing the target)
  #
  # ASSUMPTIONS:
  #   1. Submodules (gitlinks) are treated atomically, not materialized: a base-tree
  #      gitlink is never reported as deleted, merge never touches the main
  #      submodule directory, and a changed/added gitlink OID is reported as a
  #      conflict (the worktree can never clone submodule content). Full submodule
  #      round-tripping is a follow-up card.
  #   2. git_add/git_commit working in a linked worktree (hardcoded .git/index
  #      joins, wt-006 audit item d) is a separate defect and is NOT in scope for
  #      this card; this card covers the diff/merge/checkpoint machinery only.
  #
  # ========================================
  Background: User Story
    As a developer using isolated sessions (or an AI agent in one)
    I want to diff, merge, checkpoint, stage and commit a worktree that contains symlinks, ignored artifacts, and special file modes
    So that the session diff/merge machinery represents git state faithfully so nothing is silently lost, deleted, or misreported

  # ========================================
  # Part A: Gitignored files are surfaced honestly
  # ========================================

  @git
  @session-merge
  Scenario: Session diff reports build artifacts as ignored rather than as changes
    Given a git repository whose .gitignore ignores "build/"
    And a session worktree where tracked file "src/main.rs" is unchanged and "build/out.bin" was created
    When I get the session diff for that session
    Then files_changed, files_added and files_deleted are all empty
    And files_ignored contains "build/out.bin"
    And the unified diff does not mention "build/out.bin"

  @git
  @session-merge
  Scenario: Merging a session with only ignored artifacts does not touch the main worktree
    Given a git repository whose .gitignore ignores "build/"
    And a gitignored file "build/local.txt" created in the MAIN worktree after the session started
    And a session worktree whose only change is the ignored file "build/session.bin"
    When I merge that session into main
    Then the merge reports no modified, added or deleted files
    And the session worktree is removed
    And "build/local.txt" in main still exists with its content

  @git
  @session-merge
  Scenario: inspect_session_changes reports zero changed files for a worktree whose only delta is ignored
    Given a session worktree whose only delta is gitignored content
    When I inspect the session's pending changes
    Then files_changed is 0
    And the ignored files are surfaced in the summary's files_ignored list

  # ========================================
  # Part B: Symlinks round-trip through worktree creation, diff, and merge
  # ========================================

  @git
  @session-management
  Scenario: Worktree checkout materializes committed symlinks as real symlinks
    Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    When I create a session worktree at HEAD
    Then "links/config" in the worktree is a symlink
    And the symlink target of "links/config" in the worktree is "real/config.txt"

  @git
  @session-merge
  Scenario: Merging a session preserves committed symlinks in main
    Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    And a session worktree where tracked file "src/main.rs" has been modified
    When I merge that session into main
    Then "links/config" in main still exists
    And "links/config" in main is a symlink
    And the symlink target of "links/config" in main is "real/config.txt"

  @git
  @session-merge
  Scenario: A symlink whose target changed in the session is applied to main as a symlink
    Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    And a session worktree where "links/config" was re-pointed to "real/other.txt"
    When I merge that session into main
    Then "links/config" in main is a symlink
    And the symlink target of "links/config" in main is "real/other.txt"

  @git
  @session-merge
  Scenario: A symlink deleted in the session is deleted from main
    Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    And a session worktree where "links/config" has been deleted
    When I merge that session into main
    Then "links/config" no longer exists in main

  @git
  @session-merge
  Scenario: Session diff reports a re-pointed symlink as changed, not deleted
    Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    And a session worktree where "links/config" was re-pointed to "real/other.txt"
    When I get the session diff for that session
    Then files_changed contains "links/config"
    And files_deleted does not contain "links/config"

  # ========================================
  # Part C: File modes (the executable bit) survive merge and checkpoints
  # ========================================

  @git
  @session-merge
  Scenario: Merging a session preserves the executable bit of an unchanged executable file
    Given a git repository with an executable file "bin/tool" committed at mode 100755
    And a session worktree where tracked file "src/main.rs" has been modified
    When I merge that session into main
    Then "bin/tool" in main still exists
    And "bin/tool" in main is executable

  @git
  @session-merge
  Scenario: The executable bit added in a session is applied to main
    Given a git repository with a regular (non-executable) file "bin/tool"
    And a session worktree where "bin/tool" has been made executable
    When I merge that session into main
    Then "bin/tool" in main is executable

  @git
  @checkpoint-management
  Scenario: A ghost checkpoint round-trip preserves the executable bit and content
    Given a git repository with an executable file "bin/tool"
    When I create a ghost checkpoint and then modify "bin/tool" content and remove its executable bit
    And I restore the checkpoint with force
    Then "bin/tool" has its original content
    And "bin/tool" in the working tree is executable again

  # ========================================
  # Part D: Submodules (gitlinks) are atomic
  # ========================================

  @git
  @session-management
  Scenario: A committed submodule is reported as present (not deleted) and survives a merge
    Given a git repository with a committed submodule (gitlink) "vendor/lib"
    And a session worktree where tracked file "src/main.rs" has been modified
    When I get the session diff for that session
    Then files_deleted does not contain "vendor/lib"
    When I merge that session into main
    Then "vendor/lib" in main still exists with its content

  # ========================================
  # (TUI behavior — /merge-worktree with only ignored changes — lives in
  #  spec/features/merge-worktree-ignored-only-nothing-to-merge.feature so
  #  the 1:1 feature↔test-file mapping holds per file.)
  # ========================================
