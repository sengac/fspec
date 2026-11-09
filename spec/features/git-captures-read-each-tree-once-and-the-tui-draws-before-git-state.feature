@done
@BUG-657
@BUG-603
@bug
@git
@performance
@watcher
@tui
Feature: Git captures read each tree once and the TUI draws before git state arrives
  """
  rust/git/src/head_tree.rs (new): ObjectSource trait (read_tree(id) -> decoded gix::objs::Tree), implemented for gix::Repository; head_entries(source, root_tree_id) walks the HEAD tree depth-first once and returns the blob/link/gitlink entries in tree order plus a path map. status.rs staged_files_with_repo uses it in place of the per-path lookup_entry_by_path and the second collect_head_blob_paths walk. capture.rs gains capture_changed_files_with(repo, source) (the counting seam); capture_changed_files(dir) opens the repo and passes it as its own source. rpc/src/changed_files.rs (a copy of the capture with three cold opens) delegates to capture_changed_files.
  rust/core/src/git_state.rs GitStateWatcher: the constructor stores GitState::default() with an empty signature and dispatches the first capture onto the blocking pool under the existing in-flight guard (publishing through re_snapshot_and_publish, so the capture hook runs for it too); the poll interval starts at now + interval. Without a tokio runtime the constructor captures inline (there is no pool to use).
  rust/fspec-tui/src/app/bootstrap.rs: checkpoint counts and workspace info move into spawned tasks that send their Action on action_tx (body in a new app/bootstrap_git_counts.rs to respect the 300-LoC ceiling); list_work_units stays awaited so the first frame already has the board.
  Test seam: rust/git/tests/bug657_capture_reads_each_tree_once.rs builds a large loose-object fixture with the git CLI (gc.auto=0, user config ignored as in BUG-599) and counts reads through a CountingSource wrapping the opened repository; rust/core/tests/bug657_git_state_watcher_starts_off_thread.rs holds the first capture back with the capture hook (a channel), never a clock.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Root cause (measured 2026-11-08): the staged pass of every changed-files capture (rust/git/src/status.rs staged_files_with_repo) resolved each of the 13,858 index paths from the root HEAD tree with gix lookup_entry_by_path, which reads and inflates every tree on the path again; with no object cache that is about 34,500 loose-object opens per capture (spec/features, 381 KB, inflated 4,894 times), and the git-state watcher runs one at startup and one every 10 s
  #   2. A changed-files capture reads each HEAD tree object at most once: it walks the HEAD tree once into a path-to-entry map and checks every index entry against that map; no index path is ever resolved from the root tree
  #   3. Every tree a capture reads goes through one object-source seam (codelet_git::status::ObjectSource), so a test can count the reads exactly; production passes the opened repository
  #   4. The staged list, its A/M/D letters and its order are unchanged by the single walk: staged entries in index order, then staged deletions in HEAD-tree order; submodule (gitlink) entries are compared but never listed as deletions
  #   5. The git-state watcher never captures on the thread that constructs it: the first snapshot is captured on the blocking pool and broadcast when it lands, and until then snapshot() is the empty GitState; the first periodic poll fires one interval after start, so startup runs one capture, not two back to back
  #   6. The TUI draws the board before any git-derived value arrives: bootstrap seeds the board from list_work_units and loads the checkpoint counts and the workspace info (git branch) in background tasks that send CheckpointCountsLoaded / WorkspaceInfoLoaded (or the failure) on the action bus
  #
  # EXAMPLES:
  #   1. A fixture repo with 1,200 tracked files under 300 nested directories (spec/features, spec/attachments/<CARD>/, rust/<crate>/src/...), all objects loose: one changed-files capture through a counting object source reads every HEAD tree exactly once (reads == number of trees in HEAD), where the per-path lookup read them thousands of times
  #   2. In that fixture with a staged edit, a staged new file, a staged deletion and an unstaged edit, the capture through the counting source returns the same changed-files list as before (same paths, letters, staged flags and order)
  #   3. A GitStateWatcher constructed in a runtime whose capture is held back returns at once with snapshot() == GitState::default(); when the capture is released the first frame is broadcast with the real branch and changed files, and exactly one capture has run before the first poll interval elapses
  #   4. An App whose backend's checkpoint_counts and get_workspace_info never return finishes bootstrap with the board seeded from list_work_units; when they return, CheckpointCountsLoaded and WorkspaceInfoLoaded arrive on the action bus and fill the header and footer
  #   5. On ~/projects/fspec (13,858 index entries, 2,175 HEAD trees, 18,880 loose objects) a fresh capture opened about 34,556 loose objects before the fix and opens each loose HEAD tree at most once after it
  #
  # ========================================
  Background: User Story
    As a fspec user opening the TUI in a large repository
    I want to see the board drawn right away while git-derived state loads in the background
    So that the TUI is usable in a repo with thousands of tracked files and a large loose-object store

  Scenario: A changed-files capture reads every HEAD tree exactly once in a large loose-object repo
    Given a fixture repository with 1,200 tracked files in 305 directories, ten of them identical, whose objects are all loose
    When one changed-files capture runs through a counting object source
    Then every tree object of the HEAD commit is read exactly once, a tree at several paths included
    And no other tree is read

  Scenario: The single walk returns the same changed-files list
    Given the fixture repository with a staged edit, a staged new file, a staged deletion and an unstaged edit
    When one changed-files capture runs through a counting object source
    Then it lists the staged edit as M, the staged new file as A and the staged deletion as D, all staged, in that order
    And it then lists the unstaged edit as M, not staged
    And the entries match what git status --porcelain reports for the same repository

  Scenario: The git-state watcher captures its first snapshot off the constructing thread
    Given a git repository with a staged new file
    And a GitStateWatcher whose capture hook holds every capture back
    When the watcher is constructed
    Then the constructor returns while the first capture is held back
    And snapshot() is the empty GitState
    When the held-back capture is released
    Then the first frame is broadcast with the branch and the staged new file
    And exactly one capture has run before the first poll interval elapses

  Scenario: The TUI bootstrap seeds the board without waiting for git-derived values
    Given an App whose backend holds checkpoint_counts and get_workspace_info back
    When App::bootstrap is invoked
    Then it returns with the board seeded from list_work_units
    When the backend releases checkpoint_counts and get_workspace_info
    Then CheckpointCountsLoaded and WorkspaceInfoLoaded arrive on the action bus and fill the board header and the workspace footer
