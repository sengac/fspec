@wip
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
