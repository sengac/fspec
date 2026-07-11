@done
@RPC-421
@tui
@agent-view
@slash-command
@rpc
@compaction
@rust
Feature: Single-sourced compaction success notice
  """
  DESIGN DECISION (dossier §4 choice): acknowledgement-shaped success on the UNCHANGED rpc_types::CompactionResult wire schema. Real producers return original_tokens = real snapshot, compacted_tokens = 0, compression_ratio = 0.0, turns 0. Rationale: (a) final numbers are unknowable at RPC-return time (DAG builds asynchronously after send_input("Continue")) — blocking would hang /compact; (b) keeping the struct byte-identical preserves the RPC schema for remote websocket clients and index.d.ts; (c) rpc037_cross_transport_parity.rs and compaction_reduction_display_contract_rpc420.rs exercise compact_session against a STUB service with canned results, so they are unaffected; (d) a zero sentinel makes any premature rendering visibly inert (0.0%) instead of plausible-but-false ~99%, unlike honestly-labelled trough fields which still invite fake displays on remote clients
  Notice single-sourcing: dispatch_slash_commands.rs Compact Ok branch goes silent (Err branch + no-session no-op kept); format_compaction_notice MOVED to dispatch_stream_chunks.rs (:242-251) — its sole caller is the StreamChunk::CompactionComplete handler in that same file (:166), and dispatch_slash_commands.rs no longer references the helper. The CompactionComplete arm's RPC-047 'acceptable parity' comment is rewritten. Feature/test 1:1 pairs: single-sourced-compaction-notice.feature → fspec-tui/tests/single_sourced_compaction_notice_rpc421.rs (TUI exactly-one-notice); honest-compaction-acknowledgement.feature → napi/tests/rpc421_honest_ack_test.rs (napi crate is the only crate seeing cli+sessions+napi — covers both engine twins + repl_loop source shape). Amended siblings keep their existing 1:1 pairs: slash-command-compact.feature↔slash_compact_rpc047.rs, rust-tui-compact-real-compaction.feature↔rpc418_compact_session.rs, compression-ratio-clamping.feature↔cmpct039_ratio_clamp_test.rs. CMPCT-039's clamp and CMPCT-041's store_pre_compaction_tokens snapshot routing are preserved untouched
  """

  Background: User Story
    As a fspec TUI or CLI user compacting a session
    I want to see exactly one truthful compaction notice per compaction, sourced from post-DAG-injection measurements
    So that I am never misled by fabricated reduction numbers measured before the DAG summary exists

  Scenario: /compact Ok emits no immediate success notice
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.0, original_tokens 8000, compacted_tokens 0, turns_summarized 0, turns_kept 0
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    Then backend.compact_session is called exactly once with session_id s-1
    And s-1's scrollback contains no line starting with "[compaction]"

  Scenario: Exactly one compaction notice lands when the CompactionComplete chunk arrives after /compact
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.0, original_tokens 8000, compacted_tokens 0, turns_summarized 0, turns_kept 0
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    And ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 75.0, original_tokens 8000, compacted_tokens 2000, turns_summarized 6, turns_kept 2 is dispatched
    Then s-1's scrollback contains exactly one line starting with "[compaction]"
    And that line equals "[compaction] 75.0% reduction (8000 → 2000 tokens, 6 turns summarised)"

  Scenario: /compact failure still emits the error notice and no compaction notice
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Err("out of memory")
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    Then s-1's scrollback contains a line equal to "[error] /compact failed: out of memory"
    And s-1's scrollback contains no line starting with "[compaction]"

  Scenario: Auto-compaction emits exactly one notice without a preceding /compact
    Given an App with an open session s-1 and no /compact command issued
    When ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 40.0, original_tokens 5000, compacted_tokens 3000, turns_summarized 4, turns_kept 2 is dispatched
    Then s-1's scrollback contains exactly one line starting with "[compaction]"
