@mux
@watcher
@git
@tui
@rpc
@bug
@BUG-182
Feature: Mux views don't work: frozen loading dialog, no loads, no git polling; add centralized git polling

  """
  GitStateWatcher (codelet-core, new module git_state.rs next to work_units) = ONE centralized git polling mechanism per the user's DRY/SOLID directive. It replaces CheckpointsWatcher (BUG-181) as the checkpoint-count source: same debounced .git notify-debouncer watch PLUS a 10-second tokio::time::interval that re-runs a `GitState` snapshot (git_branch via codelet_git::status::get_current_branch, checkpoint_counts via codelet_git::ghost_commit::count_checkpoints, changed_files via the same collect as rpc/src/changed_files.rs, checkpoints via the same enumerate as rpc/src/checkpoints.rs). Fresh snapshot broadcast on a tokio::sync::broadcast channel (capacity 64) ONLY when the snapshot differs from the last published one (dedup — avoids churn on every 10s tick when nothing changed). Non-repo cwd → empty GitState + root non-recursive watch for .git creation (degrades like BUG-181). The 10s timer uses tokio::time::interval with MissedTickBehavior::Skip. CheckpointsWatcher is removed; BUG-181's checkpoint-counts scenarios re-land on the GitState stream (its feature file's scenarios are updated, @bug-181 tag retained as superseded).

  WATCHER-LEVEL contract (fs-watch publish, periodic-poll catch, dedup) lives in spec/features/git-state-watcher-core.feature (1:1 test-file mapping — its tests are rust/core/tests/git_state_watcher.rs). This feature covers the TUI/transport scenarios: the chrome-bar fold, the bootstrap subscriber, the mux-pane initial loads, selection stability on refresh, the mux-aware redraw gate, and refresh-drop semantics (tests: rust/fspec-tui/tests/bug182_git_state_watcher.rs).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: A new codelet-core GitStateWatcher is the ONE centralized git polling mechanism (DRY/SOLID — it replaces CheckpointsWatcher as the checkpoint-count source so there is exactly one watcher). It keeps the BUG-181 debounced .git fs-watch AND adds a 10-second tokio timer that re-runs the git snapshot (branch + checkpoint counts + changed-files list + checkpoints list). A fresh snapshot is broadcast on a tokio broadcast channel ONLY when the snapshot differs from the last published one; a non-repo cwd degrades to empty state and watches the root for .git creation; watcher setup failure degrades to a static snapshot.
  #   2. R2: The SharedFspecService exposes git_state_changed_rx() + git_state_snapshot() mirroring the BUG-181 checkpoint_counts_changed_rx()/checkpoint_counts_snapshot() pair; the embedded transport forwards them on FspecBackend::git_state_changed_rx() (default = closed receiver so the WebSocket transport + MockBackend compile unchanged and degrade to bootstrap values + the existing RefreshCheckpointCounts re-poll, documented like TUI-109).
  #   3. R3: The App bootstrap spawns the git-state subscriber task on git_state_changed_rx (the BUG-181 checkpoint-changed subscriber is REPLACED by this stream, so the subscriber total stays 7). It folds each GitState frame onto the existing writer paths — BoardStore::set_checkpoint_counts (via CheckpointCountsLoaded), AgentViewStore::set_workspace (via WorkspaceInfoLoaded with the fresh git_branch) — so there is exactly one writer path per store field (no second writer, DRY).
  #   4. R4: The GitState frame feeds the lazy views: when a frame arrives and the Changed Files view is visible (active single view OR a rendered mux pane), the App re-fetches the changed-files list through the existing Action::ChangedFilesLoaded path with a brief 'Refreshing changed files…' loading stage; same for the Checkpoints view (list → files → diff cascade, mirroring the open flow). The initial-load stage is never double-started: a refresh whose view is still in its initial load is dropped.
  #   5. R5: The run loop's redraw gate is mux-aware: Navigator::is_view_loading() returns true iff ANY loaded-then-loading lazy view is visible — the active single view (existing behavior) OR, in ViewMode::Mux, each rendered mux pane of kind ChangedFiles / Checkpoints that has a LoadTracker stage in flight. Without this the loading dialog's 80ms-cadence braille spinner freezes in mux mode because the 16ms tick stops drawing when nothing else demands a frame.
  #   6. R6: Refreshing a loaded list keeps the existing selection stable — the changed-files view re-selects the SAME file path (not index) and the checkpoints view re-selects the SAME (work_unit_id, name) after a refresh, so the user's context survives a git poll. If the selected item no longer exists (file deleted / checkpoint gone) the selection falls back to the first row and the diff/files re-load for it. Scroll offsets are preserved where the selection maps to a stable row.
  #   7. R7: Entering mux mode (any path: /mux on, /mux default, /mux <count>, /mux <kinds>, config-dialog commit) triggers an initial load for every rendered lazy pane that has not loaded yet — the ChangedFiles pane runs the existing open flow (reset view + spawn changed_files) and the Checkpoints pane runs its (reset view + spawn list_checkpoints + re-poll counts) WITHOUT flipping the whole view (R8 semantics: panes load in place). Already-loaded views are not reset (no gratuitous reload on /mux on/off cycling).
  #
  # EXAMPLES:
  #   1. TUI runs in mux mode with a Files pane; the user's agent commits a new file in the repo; within ~10 seconds the Files pane shows the new file without the user re-opening the view (fs-watch event → re-fetch).
  #   2. In the Files mux pane the user has a file selected while a 10-second poll lands a refresh: the same file remains selected (by path), the diff pane still shows its diff, and the scroll positions of the file list and diff panes are preserved.
  #   3. The user switches from the `main` branch to a feature branch via another terminal; the chrome bar of the agent view (bottom right) shows the new branch within one poll (≤ ~10 s) without restarting the TUI.
  #   4. Entering mux mode with a Files pane shows the animated loading dialog (braille spinner advancing at 80ms cadence) while the file list loads — exactly like the single-view F key flow — and the dialog dismisses to the real file list when the load flushes.
  #   5. In the Checkpoints mux pane the user has a checkpoint selected; a poll refresh lands while a checkpoint created by another terminal exists: the new checkpoint appears in the list, the user's previous checkpoint selection is kept (by work-unit + name), and its files/diff re-load.
  #
  # ========================================

  Background: User Story
    As a developer supervising agents in the fspec TUI mux mode
    I want to see the Changed Files and Checkpoints mux panes load, stay live, and animate like their single-view counterparts, with the chrome bar's git branch always current
    So that mux mode is a trustworthy live view of the repository instead of frozen, un-loaded panes

  Scenario: The chrome bar git branch indicator updates when the branch changes without a TUI restart
    Given an App on the embedded transport whose AgentViewStore holds WorkspaceInfo with git_branch main from bootstrap
    When the branch is switched to feature-x from another terminal
    Then the AgentViewStore workspace git_branch is feature-x and the BoardStore checkpoint counts are folded from the same GitState frame without a TUI restart


  Scenario: The bootstrap git-state subscriber folds frames into the BoardStore and AgentViewStore
    Given an App constructed with a backend whose git-state push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 } and the AgentViewStore workspace git_branch is None
    When a GitState frame with checkpoint_counts { manual: 1, auto: 0 } and git_branch Some(main) is pushed onto the channel
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } and app.agent_view_store().workspace().git_branch() is Some(main)


  Scenario: Entering mux mode loads un-loaded lazy panes in place
    Given an App whose changed-files and checkpoints views have not loaded yet
    When mux mode is entered with a config that renders Board, Agent, ChangedFiles and Checkpoints panes
    Then the ChangedFiles and Checkpoints panes run their initial open flows in place (reset view + spawn the list load) without flipping the whole view out of Mux


  Scenario: Refreshing a loaded changed-files view keeps the selection stable by path
    Given a loaded changed-files view with file b.txt selected out of [a.txt, b.txt, c.txt]
    When a git-state refresh lands the new file list [a.txt, b.txt, d.txt]
    Then b.txt remains selected (re-selected by path, not index) and its diff reloads


  Scenario: Refreshing a loaded checkpoints view keeps the selection stable by work-unit and name
    Given a loaded checkpoints view with checkpoint AUTH-001/alpha selected
    When a git-state refresh lands a checkpoint list that still contains AUTH-001/alpha
    Then AUTH-001/alpha remains selected (re-selected by work-unit + name) and its files and diff reload


  Scenario: The redraw gate stays open while a rendered mux lazy pane is loading
    Given a Navigator in ViewMode::Mux whose rendered ChangedFiles pane has a LoadTracker stage in flight and no other view is loading
    When Navigator::is_view_loading is queried
    Then it returns true so the 16ms tick keeps drawing and the loading dialog braille spinner does not freeze in mux mode


  Scenario: A git-state refresh of a view still in its initial load is dropped
    Given an App whose changed-files view is still in its initial load (the list stage has not flushed)
    When a git-state frame arrives while the initial load is in flight
    Then the refresh is dropped (no double-start of the initial load) and the in-flight initial load completes un-impeded

