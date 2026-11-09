@done
@session-merge
@git
@session
@bug-fix
@rust
@WT-013
Feature: The worktree merge commit reverts main's commits to files the session never touched
  """
  commit_merged_session (rust/git/src/merge_commit.rs) builds the merge commit tree from main's HEAD tree and overlays only the session delta (SessionResult files_added, files_changed, files_deleted) through overlay_session_delta. Each delta path is first removed from every entry kind and then re-added from the post-apply snapshot (blob with its executable bit, or symlink), so type and mode changes round-trip. Gitlinks keep HEAD's entry because a session can never change a submodule OID (WT-006). Before this fix the whole post-apply worktree snapshot was overlaid; that snapshot carries every tracked path at base content, so every file main committed after the session's base commit was reverted in the merge commit and deleted files came back. Clearing an executable bit was lost in three places, all fixed: collect_worktree_files (tree_utils.rs) now reads a tracked file's bit from disk on Unix instead of OR-ing it with the index mode, apply_snapshot_to_main (session_diff.rs) writes a changed file with the worktree's bit only, and the merge commit no longer only adds executables.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The merge commit tree is main's HEAD tree with only the session's added, modified and deleted paths applied; every other path keeps HEAD's entry
  #   2. A path main changed, added or deleted after the session's base commit, and the session never touched, keeps main's HEAD entry in the merge commit
  #   3. Each session path takes its full entry from the applied state (content, file kind and executable bit), so a session's mode change, including clearing the executable bit, lands in the commit
  #   4. After the merge, git status in main is clean for both the session's paths and the paths main changed
  #
  # EXAMPLES:
  #   1. Main commits a change to notes.txt and to spec/work-units.json after the session starts; the session edits only hello.txt; after the merge HEAD:notes.txt and HEAD:spec/work-units.json are main's versions, HEAD:hello.txt is the session's, and git status is clean
  #   2. Main deletes old.txt in a commit after the session starts; the session never touches it; after the merge old.txt is absent from HEAD and from the working tree
  #   3. The session clears the executable bit of run.sh that was executable at base; after the merge HEAD:run.sh has mode 100644
  #
  # ========================================
  Background: User Story
    As a developer merging an isolated session back into main
    I want to get a merge commit that contains only the session's own changes on top of main's HEAD
    So that commits main made while the session was running are never reverted in history

  Scenario: Main's commits to files the session never touched survive the merge
    Given a temp git repo with hello.txt, notes.txt and spec/work-units.json committed, and an isolated session created from that commit
    And main then commits a change to notes.txt and to spec/work-units.json
    And the session edits only hello.txt
    When the session is merged
    Then HEAD:notes.txt and HEAD:spec/work-units.json hold main's committed versions
    And HEAD:hello.txt holds the session's version
    And git status in main is clean

  Scenario: A file main deleted after the session started stays deleted
    Given a temp git repo with hello.txt and old.txt committed, and an isolated session created from that commit
    And main then commits the deletion of old.txt
    And the session edits only hello.txt
    When the session is merged
    Then old.txt is absent from HEAD and from main's working tree
    And git status in main is clean

  Scenario: Clearing the executable bit in the session lands in the merge commit
    Given a temp git repo with an executable run.sh committed, and an isolated session created from that commit
    And the session clears the executable bit of run.sh
    When the session is merged
    Then HEAD:run.sh has mode 100644
    And git status in main is clean
