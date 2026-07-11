@done
@RPC-421
@agent-core
@context-management
@rpc
@compaction
Feature: Honest compaction acknowledgement from compact_session
  """
  RPC-421 engine-side twin of single-sourced-compaction-notice.feature. compact_session in BOTH twins (codelet/sessions/src/handle_impl.rs and codelet/napi/src/session_bindings.rs session_compact) measures compacted_tokens immediately after execute_compaction clears the context to reminders + instruction — BEFORE the agent builds the DAG summary. The returned CompactionResult therefore carried a fabricated ~90-99% reduction with turns_summarized 0.
  Fix: acknowledgement-shaped success on the UNCHANGED rpc_types::CompactionResult wire schema — original_tokens = real pre-compaction snapshot (honest), compacted_tokens = 0, compression_ratio = 0.0, turns_summarized = 0, turns_kept = 0. Final numbers are unknowable at RPC-return time (the DAG builds asynchronously after send_input("Continue")); the CompactionComplete chunk (CMPCT-038 apply-site emission) is the single source of truth for the numbers.
  The plain-CLI REPL (codelet/cli/src/interactive/repl_loop.rs /compact arm) is the third instance: it printed "[Context compacted: X→Y tokens, Z% compression]" from the same trough measurement. It now prints a compaction-started message with no fabricated numbers. Debug-capture events (BUG-134) keep recording the real trough measurements — diagnostics, not display.
  Test file: codelet/napi/tests/rpc421_honest_ack_test.rs — the napi test crate is the only crate that sees codelet-cli, codelet-sessions and codelet-napi at once (cmpct038/cmpct039 precedent), so both engine twins and the repl_loop source shape are validated there.
  """

  Background: User Story
    As a fspec TUI or CLI user compacting a session
    I want the compact_session RPC result to be an honest started-successfully acknowledgement
    So that no frontend or remote client can render a fabricated reduction measured before the DAG summary exists

  Scenario: compact_session returns an acknowledgement instead of fabricated reduction numbers
    Given a populated session with several user and assistant messages
    When the session is compacted through the session manager handle
    Then the returned CompactionResult original_tokens is greater than zero
    And the returned CompactionResult compacted_tokens is exactly 0
    And the returned CompactionResult compression_ratio is exactly 0.0
    And the returned CompactionResult turns_summarized and turns_kept are 0

  Scenario: NAPI session_compact returns the same acknowledgement shape
    Given a populated NAPI session with several user and assistant messages
    When the session is compacted through the NAPI session_compact binding
    Then the returned CompactionResult original_tokens is greater than zero
    And the returned CompactionResult compacted_tokens is exactly 0
    And the returned CompactionResult compression_ratio is exactly 0.0

  Scenario: Plain-CLI REPL /compact prints no fabricated numbers
    Given the plain-CLI REPL /compact success handler source
    When the /compact success print statements are inspected
    Then the source no longer prints the fabricated context-compacted percentage line
    And the source prints a compaction-started message referencing the in-view DAG flow
