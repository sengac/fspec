@done
@context-management
@rpc
@ts-parity
@rust
@compaction
@RPC-418
@BUG-432
@BUG-516
Feature: Rust TUI /compact command lands on a no-op stub - no real compaction performed
  """
  Fix lives in rust/sessions/src/handle_impl.rs compact_session (~line 261). Mirror NAPI reference session_compact in rust/napi/src/session_bindings.rs:3038.
  Use execute_compaction from codelet_cli::interactive_helpers with None (manual, no resume prompt). Bridge sync->async with block_in_place + Handle::current().block_on or the existing loop_block_on helper. Drop the inner lock before send_input('Continue').
  BUG-516 (product owner, option A): the in-view hand-off above is superseded. Since BUG-214 every turn refuses to start while a compaction working view blocks ordinary inference, so the 'Continue' builder turn could never run. compact_session now validates the session, enters Compacting and queues an internal manual-compaction request. The agent loop runs it as the same compactor sub-agent round that automatic pre-prompt compaction uses (codelet_cli::compactor_sub_agent). There is no working view, no 'Continue' message and no ordinary inference turn. Success commits the summary and emits CompactionComplete; failure emits 'Compaction failed: <reason>' and leaves the conversation unchanged. The BUG-214 gate is untouched.
  Tests go in rust/sessions/tests/rpc418_compact_session.rs, following rpc081_restore_session_messages.rs patterns; use #[tokio::test(flavor = 'multi_thread')] because block_in_place panics on single-thread runtime.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compacting a session that has messages must run execute_compaction: clear the conversation to system-reminders, inject the compaction instruction, and reset the token tracker
  #   2. Compacting an empty session (no messages) must return an error 'Nothing to compact' and leave the session untouched
  #   3. After a successful compaction the handle must send 'Continue' to the agent loop to kick off DAG construction
  #   4. The returned CompactionResult must report real pre and post token counts and a real compression ratio, never the hard-coded 1.0 placeholder
  #   5. Compacting an unknown session id must return an error 'Session not found'
  #   6. On execute_compaction failure the session status must revert to Idle, compaction progress cleared, and the error propagated to the caller
  #
  # EXAMPLES:
  #   1. A session with several user and assistant messages is compacted: the messages are cleared to system-reminders, the compaction instruction is present, and 'Continue' is sent so the agent starts building the DAG
  #   2. A brand-new empty session is compacted: the handle returns an error 'Nothing to compact' and the message count stays at zero
  #   3. Compacting a session that has messages returns a CompactionResult whose original_tokens is greater than zero and whose compacted_tokens is less than original_tokens
  #   4. Compacting a session id that does not exist returns an error whose message begins with 'Session not found'
  #
  # ========================================
  Background: User Story
    As a fspec user running the Rust ratatui TUI
    I want to run the /compact command and have it perform real in-view DAG compaction
    So that my context window is actually reduced instead of the command silently doing nothing

  Scenario: Compacting a populated session runs the compactor agent and replaces the conversation with its summary
    Given a session with several user and assistant messages
    When I compact the session through the handle
    Then the compactor agent builds the summary once, as automatic compaction does
    And the conversation is replaced by the committed summary
    And no ordinary inference turn runs and no "Continue" message is sent
    And the session returns to Idle with ordinary inference allowed

  Scenario: A failed manual compaction leaves the conversation unchanged
    Given a session with several user and assistant messages
    And the compactor agent fails with "summary service unavailable; retry later"
    When I compact the session through the handle
    Then the session shows "Compaction failed:" with the compactor agent's reason
    And no ordinary inference turn runs
    And the conversation is unchanged

  Scenario: Compacting an empty session returns an error and leaves it untouched
    Given a brand-new session with no messages
    When I compact the session through the handle
    Then the handle returns an error containing "Nothing to compact"
    And the session message count stays at zero

  Scenario: Compacting a populated session reports real token counts
    Given a session with several user and assistant messages
    When I compact the session through the handle
    Then the returned CompactionResult original_tokens is greater than zero
    And the returned CompactionResult compacted_tokens is the acknowledgement sentinel 0
    And the returned CompactionResult compression_ratio is the acknowledgement sentinel 0.0
    And the returned CompactionResult turns_summarized and turns_kept are 0

  Scenario: Compacting an unknown session id returns a not-found error
    Given a session id that does not exist
    When I compact the session through the handle
    Then the handle returns an error beginning with "Session not found"
