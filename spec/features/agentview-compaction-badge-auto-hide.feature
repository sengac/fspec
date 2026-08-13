@RPC-420
@store
@rpc
@header
@agent-view
@tui
@RPC-417
Feature: COMPACTED header badge never auto-hides after 10s (TS parity gap)
  """
  TS reference: src/tui/components/AgentView.tsx:1333-1336 (state), 1459-1466 (useEffect setTimeout 10000ms -> setCompactionReduction(null)), 5291 (prop passed to SessionHeader). Feature tag TUI-044. Sibling TUI-031 uses same 10s pattern for tokens/sec.
  Idiomatic Rust timer pattern to follow: rust/fspec-tui/src/app/dispatch_reconnect.rs — arm_reconnect_dismiss (tokio::spawn + sleep(DISMISS_DELAY) -> action_tx.send(Action::ClearReconnectNotice{session_id,seq})), abort_reconnect_dismiss (handle.abort()), handle_clear_reconnect_notice (seq-guard no-op). Replicate as arm/abort/handle for compaction.
  Files to touch: (1) components Action enum — add variant ClearCompactionReduction { session_id: SessionId, seq: u64 }; (2) app/dispatch.rs match arm routing it to handler; (3) app/dispatch_stream_chunks.rs CompactionComplete branch — after set_compaction_reduction, bump per-session seq + runtime-guarded arm of the timer; (4) new/existing app module hosting arm_compaction_hide/handle_clear_compaction_reduction (keep files <300 LoC — consider a new app/dispatch_compaction_hide.rs); (5) store/agent_view/chrome_state.rs + agent_view.rs — add per-session seq map (compaction_reduction_seq_by_session: HashMap<SessionId,u64>) with bump/get accessors; clear_compaction_reduction MUST also bump the seq.
  Timer handle storage: keep per-session JoinHandles in a HashMap<SessionId, JoinHandle<()>> on App (abort prior handle when re-arming that session). Aborting is an optimization; correctness rests on the seq-guard so a stale fire is always a no-op. AGENTS.md forbids extra unwrap/todo; use if-let and let _ = send patterns like dispatch_reconnect.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When StreamChunk::CompactionComplete sets the per-session compaction_reduction, an auto-hide timer MUST be armed that clears that session's compaction_reduction after 10 seconds — mirroring TS TUI-044 (setTimeout 10000ms in AgentView.tsx useEffect).
  #   2. The auto-hide MUST use the codebase's idiomatic timer pattern (dispatch_reconnect.rs): tokio::spawn + tokio::time::sleep(10s) sending a self-addressed Action::ClearCompactionReduction { session_id, seq } on the App action channel — NOT an Instant-elapsed check polled during render/tick.
  #   3. The clear MUST be guarded by a per-session monotonic seq: if a newer CompactionComplete re-armed the timer (bumping the seq) before the old timer fired, the stale Action::ClearCompactionReduction MUST be a silent no-op so the newer badge survives. Only clear when the fired seq still matches the session's current seq.
  #   4. Auto-hide is PER-SESSION: clearing session s-1's COMPACTED badge (by timer expiry) MUST NOT clear or affect session s-2's badge or seq.
  #   5. Arming the timer MUST be runtime-guarded (tokio::runtime::Handle::try_current().is_ok()) exactly like spawn_fspec_command_runner, so existing synchronous #[test] paths (e.g. the RPC-100 tests) that dispatch CompactionComplete without a tokio runtime do NOT panic. Under no runtime, the badge simply persists (no timer) — pre-existing behaviour is preserved for those tests.
  #   6. The existing SessionStateChange::Cleared reset path MUST continue to clear the compaction_reduction immediately (before 10s) AND bump/invalidate the session's seq so any still-pending auto-hide timer becomes a stale no-op.
  #
  # EXAMPLES:
  #   1. Compaction completes for s-1 (ratio 60.0 -> COMPACTED 60%). Header shows [80%: COMPACTED 60%]. ~10s later the auto-hide Action::ClearCompactionReduction fires; next render shows plain [80%] with no COMPACTED suffix.
  #   2. Compaction completes for s-1 (recording its auto-hide sequence number). Before 10s elapse, a second CompactionComplete arrives for s-1 (a newer auto-hide sequence, new reduction). The now-superseded ClearCompactionReduction carrying the earlier sequence fires and is ignored (seq mismatch); header still shows the newer COMPACTED value.
  #   3. Two sessions s-1 and s-2 both complete compaction. The auto-hide timer for s-1 fires (clears s-1's badge to plain [X%]) while s-2's badge still shows COMPACTED — clearing one session does not affect the other.
  #   4. Compaction completes for s-1. Before 10s, the user runs /clear (SessionStateChange::Cleared). The badge disappears immediately (header shows [0%]) and when the original auto-hide timer later fires it is a no-op (seq already invalidated) — no panic, no effect.
  #   5. Under tokio test time control (start_paused): dispatch CompactionComplete, advance the virtual clock by 10s, drain the action channel -> an Action::ClearCompactionReduction was emitted and after dispatch+render the badge is gone. This proves the real timer arms and fires at 10s without waiting in wall-clock time.
  #
  # ========================================
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to have the [X%: COMPACTED Y%] header badge disappear automatically ~10 seconds after a compaction completes
    So that the header returns to the plain [X%] form on its own, matching the TypeScript original, instead of leaving a stale COMPACTED suffix pinned until I run /clear

  Scenario: The COMPACTED badge auto-hides after the 10-second timer fires
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with fill_percentage 80) has been dispatched
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    And the SessionHeader text contains "[80%: COMPACTED 60%]"
    When the 10-second auto-hide timer elapses and the queued Action::ClearCompactionReduction for "s-1" is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[80%]"
    And the SessionHeader text does NOT contain "COMPACTED"

  Scenario: A stale auto-hide action is ignored when a newer compaction re-armed the timer
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched, recording its auto-hide sequence number
    And a second Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 70.0) has been dispatched, superseding the first with a newer auto-hide sequence
    When the now-superseded Action::ClearCompactionReduction for "s-1" (the earlier sequence) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED 70%"

  Scenario: Auto-hiding one session's badge does not affect another session's badge
    Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    And Action::ChunkReceived("s-2", StreamChunk::CompactionComplete with compression_ratio 70.0) has been dispatched
    When the Action::ClearCompactionReduction for "s-1" is dispatched
    And the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    Then the SessionHeader text does NOT contain "COMPACTED"
    When the App dispatches Action::SessionNext to focus "s-2" and re-renders
    Then the SessionHeader text contains "COMPACTED 70%"

  Scenario: A /clear before 10 seconds hides the badge immediately and neutralises the pending timer
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    When Action::ChunkReceived("s-1", StreamChunk::SessionStateChange { state: Cleared }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[0%]"
    And the SessionHeader text does NOT contain "COMPACTED"
    When the original pending Action::ClearCompactionReduction for "s-1" with the pre-clear seq is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[0%]"

  Scenario: Under paused tokio time the real timer arms and emits the clear action at 10 seconds
    Given session "s-1" is open in AgentView with "s-1" focused under a start_paused tokio runtime
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    When the virtual clock is advanced by 10 seconds
    And the pending actions on the App action channel are drained and dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then an Action::ClearCompactionReduction for "s-1" was emitted on the channel
    And the SessionHeader text does NOT contain "COMPACTED"
