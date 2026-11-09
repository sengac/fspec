@done
@RPC-421
@agent-core
@context-management
@rpc
@compaction
Feature: Honest compaction acknowledgement from compact_session
  """
  RPC-421 engine-side twin of single-sourced-compaction-notice.feature. compact_session in BOTH twins (rust/sessions/src/handle_impl.rs and rust/napi/src/session_bindings.rs session_compact) measures compacted_tokens immediately after execute_compaction clears the context to reminders + instruction — BEFORE the agent builds the DAG summary. The returned CompactionResult therefore carried a fabricated ~90-99% reduction with turns_summarized 0.
  Fix: acknowledgement-shaped success on the UNCHANGED rpc_types::CompactionResult wire schema — original_tokens = real pre-compaction snapshot (honest), compacted_tokens = 0, compression_ratio = 0.0, turns_summarized = 0, turns_kept = 0. Final numbers are unknowable at RPC-return time (the DAG builds asynchronously after send_input("Continue")); the CompactionComplete chunk (CMPCT-038 apply-site emission) is the single source of truth for the numbers.
  Test file: rust/napi/tests/rpc421_honest_ack_test.rs — the napi test crate is the only crate that sees codelet-cli, codelet-sessions and codelet-napi at once (cmpct038/cmpct039 precedent), so both engine twins are validated there.

  Merged into rust-tui-compact-real-compaction (RPC-418) by COV-075 (product owner, 2026-11-05: duplicates merge into the owning feature): RPC-418's 'Compacting a populated session reports real token counts' already asserted the acknowledgement sentinels on the same test and now also carries the turns_summarized/turns_kept step, so this scenario was removed here.
  """

  Background: User Story
    As a fspec TUI or CLI user compacting a session
    I want the compact_session RPC result to be an honest started-successfully acknowledgement
    So that no frontend or remote client can render a fabricated reduction measured before the DAG summary exists
