@BUG-582
@done
@git
@session
@session-merge
@rust
@critical
@WT-017
Feature: Merging a worktree never merges shared spec state
  """
  Product owner decision 2026-11-07 (option a, shared state root). An isolated session reads and writes the main checkout's shared spec state through its state-root pointer (WT-015), so its worktree's checked-out copies of shared files (board and ID counters, epics, prefixes, tags and TAGS.md, foundation, attachments, config, schedules, virtual-hook scripts, stage permissions, blocklist) are stale and must never reach main. codelet-git's merge leaves them out of the session diff, conflict detection, apply and merge commit, using codelet_common::spec_paths::is_shared_state_path, the same definition the resolver uses. A worktree copy that differs from the session's base means something wrote around the resolver, so the merge is refused before anything is copied, naming the files. Features, coverage files, fspec-hooks.json and code merge as before.
  Tests: rust/git/tests/wt017_merge_never_merges_shared_state.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The session diff, conflict detection, apply and merge commit leave out every shared-state path, taken from the one definition in codelet-common (is_shared_state_path, WT-015); git keeps no list of its own
  #   2. Shared paths never appear in the inspect counts or diff, the conflict list or the merge commit, even when the main checkout's copy changed after the session started
  #   3. If the worktree's copy of a shared file differs from the session's base (modified, added or deleted), the merge is refused before anything is copied or committed, naming every such file, and the worktree stays for a retry
  #   4. Features, coverage files, fspec-hooks.json and code merge exactly as before
  #   5. WT-014's merge-lock scenarios merge a branch spec file (spec/features/*) instead of spec/work-units.json, since the board lives in the one shared state root and only spec/features travel with the branch (product owner, 2026-11-07, WT-015; supervisor confirmed 2026-11-07)
  #
  # EXAMPLES:
  #   1. Main commits a board change and edits its board again after the session starts; the session changes src/lib.rs and adds spec/features/login.feature; the merge applies and commits both, main's spec/work-units.json keeps main's content, and the merge commit does not touch it
  #   2. The session's worktree copy of spec/work-units.json was edited directly; merging fails naming spec/work-units.json, main's files and HEAD are unchanged, and the worktree still exists
  #   3. The session deleted its copy of spec/tags.json; merging fails naming spec/tags.json
  #   4. Inspecting a session that changed only code lists no shared paths even though main's board changed since the session started
  #
  # ========================================
  Background: User Story
    As a developer merging an isolated session back into the main checkout
    I want to have the merge carry only branch files (code, features, coverage, hooks) and never the worktree's copies of shared spec state
    So that a merge can never roll back or overwrite the board, IDs, tags, foundation or config that every session shares

  Scenario: A merge carries the session's code and features and leaves main's board alone
    Given a temp git repo with spec/work-units.json, a feature and src/lib.rs committed, and an isolated session
    And main committed a board change and then changed its board again in the working tree
    And the session changed src/lib.rs and added spec/features/login.feature
    When the session is inspected and merged
    Then the inspect lists src/lib.rs and spec/features/login.feature and no shared path
    And main has the session's src/lib.rs and spec/features/login.feature
    And main's spec/work-units.json still holds main's working-tree content
    And the merge commit changes src/lib.rs and spec/features/login.feature only

  Scenario: A worktree copy of a shared file that was changed refuses the merge
    Given a temp git repo with spec/work-units.json, spec/tags.json and src/lib.rs committed, and an isolated session
    And the session changed src/lib.rs, edited its copy of spec/work-units.json and deleted its copy of spec/tags.json
    When the session is merged
    Then the merge fails naming spec/tags.json and spec/work-units.json
    And main's files and HEAD are unchanged
    And the session worktree still exists

  Scenario: The merge takes shared state from the resolver's definition
    Given the codelet-git sources
    When they are scanned for the shared-state definition
    Then the merge filters paths with codelet_common::spec_paths::is_shared_state_path
    And no codelet-git source names a shared spec file
