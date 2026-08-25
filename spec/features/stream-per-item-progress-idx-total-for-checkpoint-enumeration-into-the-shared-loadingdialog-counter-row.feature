@tui
@ui-refinement
@done
@TUI-109
Feature: Stream per-item progress (idx/total) for checkpoint enumeration into the shared LoadingDialog counter row
  """
  Implementation:
  - Wire type CheckpointsProgress{loaded,total,done} in rpc-types; transport traits (transport/mod.rs) gain checkpoints_progress_rx() - a broadcast receiver alongside work_units_rx/chunks_rx/logs_rx (embedded: direct broadcast; websocket: forward a new message kind). Server: git's list_all_ghost_checkpoints gains a streaming/callback variant; rpc/src/checkpoints.rs gains collect_checkpoints_stream wired to FspecServiceImpl::list_checkpoints emitting progress; the CLI's non-streaming collect_checkpoints delegates to it with a no-op callback. App: subscriber task folds CheckpointsProgress into the CheckpointsView LoadingDialog via set_progress (slot provided by TUI-106), stale-dropped once list stage flushes. Scenarios extend TUI-107's feature file (checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Progress is streamed item-by-item (broadcast channel, same pattern as work_units_rx/chunks_rx/logs_rx) from the server-side enumeration; the counter row shows the number collected so far with the total once it is known (total is only known after enumeration completes, so intermediate frames may show a pending total - rendered as '(loaded/…)' until then)
  #   2. The non-streaming CLI path is byte-identical: 'fspec list-checkpoints' output does not change; the non-streaming collect function delegates to the streaming one with a no-op callback (two front doors, one source of truth - no duplicated enumeration logic)
  #   3. Transports that do not forward progress events (e.g. websocket) degrade automatically to spinner + stage label only (TUI-107 behavior) - no timeout, no extra logic required; a CheckpointsLoaded fold always takes precedence over any late progress event (stale-drop: progress events arriving after the list has folded in are ignored and the dialog does not re-appear)
  #
  # EXAMPLES:
  #   1. I open the Checkpoints view on a repo with 150 checkpoints across 10 work units; the loading dialog's counter starts at (1/…), climbs visibly as enumeration proceeds, and shows the final (150/150) just before the list appears
  #
  # ========================================
  Background: User Story
    As a fspec TUI user on a repo with many checkpoints
    I want to see the checkpoint-list loading dialog count items as they are collected
    So that know the load has real progress and won't hang - the spinner now has a (processed/total) counter

  Scenario: The loading dialog counter advances from (0/…) to (N/total) before the list folds
    Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    Then the loading dialog counter row shows (1/…) while the total is still unknown
    And the counter row climbs through the intermediate values as each progress event folds
    And the counter row shows the final (150/150) just before the list appears
    When the CheckpointsLoaded fold arrives with 150 checkpoints
    Then the list renders and the loading dialog is dismissed

  Scenario: Capped enumeration shows (200/250) - truncation does not hide progress
    Given a repository with 250 checkpoints
    When the streaming enumeration collects items with the 200-entry cap applied
    Then the progress counter reaches (200/250) - loaded stops at the cap while the total reflects the full enumeration
    And the returned list contains exactly 200 entries

  Scenario: A late progress event after CheckpointsLoaded is stale-dropped
    Given a Checkpoints view whose list has already flushed via the CheckpointsLoaded fold
    When a late CheckpointsProgress event with done=true arrives after the fold
    Then the view stays in the list presentation state and is not loading
    And the loading dialog is not re-painted - progress events after the list fold are ignored

  Scenario: A transport that does not forward progress degrades to spinner plus stage label
    Given a Checkpoints view whose list load is in flight on a transport that never emits progress events
    When the loading dialog is painted while the list load is still in flight
    Then the dialog shows the spinner and the stage label "Loading checkpoint list…"
    And no counter row is painted - the TUI-107 behavior is preserved with no timeout or extra logic

  Scenario: The non-streaming CLI path is byte-identical
    Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    When the CLI list-checkpoints command runs against a repository with checkpoints
    Then the output is byte-identical to the pre-streaming behavior
    And the existing list_checkpoints tests pass unmodified
