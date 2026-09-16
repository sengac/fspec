@done
@bug-184
@mux
@watcher
@git
@tui
@bug
@BUG-184
Feature: Mux Files/Checkpoints panes: 10s refresh loses scroll position and lacks loading dialog parity with single-view mode

  """
  BUG-182's GitStateWatcher (codelet-core git_state.rs) polls every 10s and broadcasts a GitState frame {branch, checkpoint_counts, changed_files, checkpoints} only when it differs from the last published snapshot. The TUI's dispatch_git_state.rs R4 re-fetches the visible lazy panes (Changed Files, Checkpoints) on every frame. BUG-184 hardens that path: a list_signature (LoadTracker.list_signature, set whenever the view replaces its list) records the identity of the displayed list — sanitized path/change-type/staged for files; sanitized work_unit_id/name/auto-flag for checkpoints (timestamp EXCLUDED: rows never render it and the fallback 'now' stamp churns without an index sidecar; order DOES participate). A frame leg whose signature matches the displayed list's signature is DROPPED before any re-fetch (no RPC, no dialog flash, no churn). Change-bearing refreshes preserve the list scroll offset (clamped to list_len - visible_rows; shorter-than-window lists fall back to 0) and snap the selection into the preserved window (first visible row for a fallback; last visible row + window scroll when the selection slid past the bottom). Cascade same-key reloads (checkpoints set_files on the same work_unit_id+name; set_diff on the same work_unit_id+name+path) preserve the dependent pane's scroll/selection; a key change resets it. The initial-load loading dialog is untouched — it already renders mux-aware (TUI-106/TUI-108 mux-aware is_view_loading gate); the user decision 'Initial only' means the background refresh stays dialog-free, which the no-op drop + scroll preservation deliver.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A git-state frame whose changed-files leg is byte-identical to the list a displayed ChangedFilesView shows (same sanitized paths + change types + staged flags, same order) must NOT trigger a re-fetch — no RPC, no dialog, no scroll/selection churn
  #   2. A git-state frame whose checkpoints leg is byte-identical to the list a displayed CheckpointsView shows (same sanitized work_unit_id + name + auto flag, same order; the timestamp leg is EXCLUDED because the row rendering never shows it) must NOT trigger a re-fetch
  #   3. When a change-bearing refresh lands on a loaded Files or Checkpoints list, the list scroll offset is PRESERVED as a raw offset and clamped to the fresh list length (max = list_len - visible_rows; a list shorter than the window falls back to the top).
  #   4. A cascade re-load of the SAME key (checkpoints Files pane: same work_unit_id + name; Diff pane: same work_unit_id + name + path) preserves that pane's scroll/selection; only a key CHANGE resets it to the top.
  #   5. After a refresh re-lookup, the selected row must lie inside the preserved scroll window; if the re-looked-up row falls above the window, the selection snaps to the first visible row; if it falls past the bottom, the selection snaps to the last visible row and the window scrolls to keep it visible (highlighted row and scroll window never diverge).
  #
  # EXAMPLES:
  #   1. Steady-state tick: a 10s GitState frame whose changed_files list equals the displayed list (same paths, change types, staged flags, order) → no re-fetch RPC, pane bytes unchanged, scroll + selection intact
  #   2. Change-bearing refresh: two files above the scrolled window get committed (list starts at file02.txt) → the file04.txt selection survives by path, the scroll offset keeps its point (does not jump to the top)
  #   3. List shrinks beneath the window: scrolled to the bottom of 30 files; a refresh lands only 4 files (all shorter than the window) → the window falls back to the top and the (path-stable) selection stays inside the window
  #   4. Selected file committed: scrolled to the bottom, selected file29.txt; a refresh lands a list without it → the selection falls back to the first row and the window snaps to the top (highlighted row and scroll window stay together)
  #   5. Checkpoints steady-state tick: a 10s GitState frame whose checkpoints list renders byte-identically to the displayed list (timestamps may differ — they are never rendered) → no re-fetch, checkpoint scroll + selection intact
  #   6. Cascade same-key reload: a change-bearing checkpoint refresh re-fetches the (re-)selected checkpoint's files and diff → the Files pane keeps its scroll/selection and the Diff pane keeps its scroll (clamped to the new diff length); selecting a DIFFERENT checkpoint resets both panes to the top
  #
  # ========================================

  Background: User Story
    As a TUI user watching live git state in mux mode
    I want to see the Files/Checkpoints panes update every 10 seconds without jumping
    So that a stable view: no dialog flash, no lost scroll position, no flicker

  # ========================================
  # SCENARIOS
  # ========================================

  Scenario: Steady-state tick: a GitState frame matching the displayed checkpoints list is dropped before any re-fetch
    Given a mux layout with a loaded, displayed Checkpoints pane displaying 5 checkpoints with the list scrolled past the top
    When a GitState frame arrives whose checkpoints leg renders identically to the displayed list
    Then no list_checkpoints re-fetch is spawned
    And the checkpoint list scroll offset is unchanged
    And the checkpoint selection remains stable
    And the pane does not flash the loading dialog

  Scenario: Change-bearing refresh: files committed above the window keep the scroll point
    Given a loaded Changed Files pane displaying 30 files with "file04.txt" selected and the list scrolled past the top
    When a change-bearing git-state refresh lands a list in which file00.txt and file01.txt were committed
    Then the file04.txt selection survives by path
    And the file list scroll offset keeps its point (it does not jump to the top)

  Scenario: Change-bearing refresh: a list shorter than the window falls back to the top
    Given a loaded Changed Files pane displaying 30 files scrolled to the bottom with "file29.txt" selected
    When a change-bearing git-state refresh lands a list of 3 files (shorter than the visible window) that does not contain the selection
    Then the file list window falls back to the top
    And the selection lands on the first row (inside the visible window)

  Scenario: Change-bearing refresh: a committed selection keeps the window and lands on the first visible row
    Given a loaded Changed Files pane displaying 30 files scrolled to the bottom with "file29.txt" selected
    When a change-bearing git-state refresh lands a list in which file29.txt was committed (it no longer exists)
    Then the selection moves to the first visible row of the preserved window
    And the window keeps its point (clamped to the last full page of the fresh list)

  Scenario: Cascade same-key reload: the checkpoint Files and Diff panes keep their scroll
    Given a loaded Checkpoints pane with a checkpoint selected whose files list and diff are displayed and scrolled
    When a change-bearing git-state refresh re-fetches the same checkpoint's files and diff (same work-unit + name + path)
    Then the Files pane keeps its scroll offset and selection
    And the Diff pane keeps its scroll offset (clamped to the fresh diff length)

  Scenario: Cascade key change: selecting a different checkpoint resets the dependent panes
    Given a loaded Checkpoints pane with a checkpoint's files and diff displayed and scrolled
    When a refresh re-selects a DIFFERENT checkpoint (different work-unit + name)
    Then the Files pane resets to the first file at the top
    And the Diff pane resets to the top

  Scenario: Steady-state tick: a GitState frame matching the displayed files list is dropped before any re-fetch
    Given a mux layout with a loaded, displayed Changed Files pane displaying 30 files with file04.txt selected and the file list scrolled past the top
    When a GitState frame arrives whose changed_files leg is identical to the displayed list
    Then no changed_files re-fetch is spawned
    And file04.txt remains selected
    And the pane does not flash the loading dialog
    And the file list scroll offset is unchanged


  Scenario: Change-bearing refresh: a committed checkpoint keeps the window and lands on the first visible row
    Given a loaded Checkpoints pane displaying 30 checkpoints scrolled to the bottom with checkpoint-29 selected
    When a change-bearing git-state refresh lands a list in which checkpoint-29 was removed (it no longer exists)
    Then the selection moves to the first visible row of the preserved window
    And the window keeps its point (clamped to the last full page of the fresh list)

