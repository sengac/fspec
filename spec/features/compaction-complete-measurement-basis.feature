@done
@agent-core
@context-management
@rpc
@compaction
@CMPCT-038
Feature: CompactionComplete overstates reduction — compacted_tokens counts DAG summary only, not real post-compaction context
  """
  New fn apply_pending_dag_and_emit(session, pending_dag, original_tokens, emit) in BOTH inject_summary_handler.rs twins: calls apply_pending_dag; on Some, reads session.token_tracker.input_tokens and calls emit_post_injection_events(emit, original_tokens, post_total). emit_post_injection_events param renamed injected_tokens→compacted_tokens with ratio clamped .max(0.0). Call sites: agent-loop/src/agent_loop.rs:1242 and napi/src/agent_loop.rs:1220 switch to the new wrapper (passing pre_compaction_tokens + handle_output), then set Idle. on_injected closures (agent_loop.rs:689/750) DROP the emit_post_injection_events call, keep set_compaction_progress(None). Existing napi test compaction_post_inject_loading_test.rs scenario 1 re-targets the ordering assertion at the new wrapper. Dead fallback arms (background_output.rs:295-309, napi/agent_loop.rs:1712) untouched; RPC-421 /compact notice untouched.
  """

  Background: User Story
    As a TUI user watching a compaction
    I want to see a CompactionComplete reduction percentage measured against my real post-compaction context size
    So that the COMPACTED badge reflects the true reduction (~60%) instead of a summary-only fantasy (~99%)

  Scenario: compression_ratio is clamped to zero when the post-injection context exceeds the original
    Given a tiny session where surviving reminders plus summary exceed the original token count
    When the agent loop applies the pending DAG and emits the completion chunk
    Then the emitted compression_ratio is exactly 0.0
    And the compression_ratio is never negative on the wire

  Scenario: Running is emitted before CompactionComplete at the apply site
    Given known original and compacted token counts
    When the completion events are emitted for those counts
    Then a SessionStateChange Running chunk is emitted before the CompactionComplete chunk
    And no Idle or Done chunk is emitted by the apply-and-emit step itself

  Scenario: Applying with no pending DAG emits nothing
    Given a session with no pending DAG content
    When the agent loop runs the apply-and-emit step
    Then no chunk is emitted
    And the step reports that nothing was applied
