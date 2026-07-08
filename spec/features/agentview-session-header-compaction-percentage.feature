@RPC-420
@done
@store
@rpc
@header
@agent-view
@tui
@RPC-100
Feature: Session header compaction percentage not calculating/updating properly — clamp >100, missing COMPACTED suffix, no reset on session_state cleared
  """
  TS REFERENCE (DeepSearch confirmed):
  • Renderer: src/tui/components/SessionHeader.tsx:129-132 — `percentText = compactionReduction !== null ? `[${fp}%: COMPACTED ${Math.abs(reduction)}%]` : `[${fp}%]``
  • fillPercentage origin: live `StreamChunk::ContextFillUpdate{contextFill.fillPercentage}` → setContextFillPercentage; fallback `calculateContextFillPercentage(currentContextTokens, contextWindow, maxOutput)` = round((inputTokens / (contextWindow - min(maxOutput, 32_000))) * 100) on restore
  • compactionReduction origin: AgentView.tsx:959-979 — handleCompactionComplete → setCompactionReductionRef.current?.(Math.round(result.compressionRatio)) (CORRECT: result.compressionRatio is already the percent of tokens removed [0,100]; Rust displays it directly — unit contract fixed by RPC-420)
  • Reset on cleared: AgentView.tsx:992-1006 — SessionStateChange→Cleared resets contextFillPercentage and tokenUsage to 0
  • Color: src/tui/utils/sessionHeaderUtils.ts:37-42 — <50 green, <70 yellow, <85 magenta, >=85 red
  RUST CURRENT GAPS (verified by AST read):
  • codelet/fspec-tui/src/store/agent_view/token_state.rs:63 — `self.context_fill_pct = info.fill_percentage.min(100) as u8;` ← CLAMP BUG, also drops u32→u8 range
  • codelet/fspec-tui/src/views/agent/chrome_paint.rs:65 — `compaction_reduction: None,` ← HARDCODED, never read from store
  • codelet/fspec-tui/src/app/dispatch_stream_chunks.rs:120-133 — CompactionComplete branch only formats scrollback notice, never persists per-session reduction
  • codelet/fspec-tui/src/app/dispatch_stream_chunks.rs:57-75 — SessionStateChange branch handles state→pause/resume but does NOT reset TokenState or compaction_reduction on `Cleared`
  • codelet/fspec-tui/src/store/agent_view/work_unit_state.rs:48 — `reset_token_state(session)` already exists; reuse it
  • codelet/fspec-tui/src/views/agent/header.rs:71 + header_build.rs:134 + header_build.rs:158-164 — render path already supports compaction_reduction Option<i32>; just needs to receive non-None
  IMPLEMENTATION PLAN:
  1. token_state.rs: change `context_fill_pct: u8` → `context_fill_pct: u16`; change apply_context_fill to `self.context_fill_pct = info.fill_percentage.min(u16::MAX as u32) as u16;`. Update header_build.rs build_right_line + context_fill_color signature to take u16 (color thresholds 50/70/85 still work).
  2. AgentViewStore (store/agent_view.rs): add field `compaction_reduction_by_session: HashMap<SessionId, i32>`. Add accessors `compaction_reduction_for(&SessionId) -> Option<i32>`, `set_compaction_reduction(SessionId, i32)`, `clear_compaction_reduction(&SessionId)`.
  3. dispatch_stream_chunks.rs CompactionComplete branch: compute `reduction = compaction_result.compression_ratio.round() as i32;` (wire value is already percent removed — RPC-420) and call `self.agent_view_store.set_compaction_reduction(session_id.clone(), reduction);` BEFORE the existing notice emit.
  4. dispatch_stream_chunks.rs SessionStateChange branch: on `SessionState::Cleared`, call `self.agent_view_store.reset_token_state(session_id);` and `self.agent_view_store.clear_compaction_reduction(session_id);` BEFORE the existing set_session_status call.
  5. chrome_paint.rs: replace hardcoded `compaction_reduction: None` with `compaction_reduction: sid.and_then(|s| store.compaction_reduction_for(s))`.
  6. Type changes propagate to header.rs SessionHeader.compaction_reduction field type stays Option<i32>; build_right_line ALREADY uses `r.abs()` so any i32 sign is safe.
  INTEGRATION TEST FIXTURE PLAN: spec file → spec/features/agentview-session-header-compaction-percentage.feature, test → codelet/fspec-tui/tests/agentview_session_header_compaction_percentage_rpc100.rs.
  Use the shared helpers in tests/common/mod.rs (sid, build_app, render_into 80x24 ratatui::TestBackend) — same pattern as agentview_session_header_per_session_tokens_rpc099.rs.
  Four scenarios:
  • (a) >100% fill renders raw value: dispatch ContextFillUpdate{105} on s-1, render, scrape header line, assert contains `[105%]` not `[100%]`, assert span style fg=Red.
  • (b) COMPACTED suffix renders after CompactionComplete: dispatch ContextFillUpdate{80} then CompactionComplete{ratio:0.4}, assert header contains `[80%: COMPACTED 60%]`.
  • (c) compaction_reduction does not leak across sessions: dispatch CompactionComplete on s-1, dispatch nothing on s-2, render s-1 (`COMPACTED N%` present), Action::SessionNext → render s-2 (no `COMPACTED`), Action::SessionPrev → render s-1 (`COMPACTED N%` present again).
  • (d) SessionStateChange→Cleared resets both: seed s-1 with tokens+fill+compaction, dispatch SessionStateChange{state:Cleared}, render, assert `[0%]`, no COMPACTED suffix, `tokens: 0↓ 0↑`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TokenState.context_fill_pct MUST preserve the raw u32 fill_percentage from ContextFillInfo (widened to u16) — values >100 are valid and MUST render as `[105%]` etc. so the user sees pre-compaction overshoot. Currently `.min(100) as u8` discards this signal.
  #   2. AgentViewStore MUST hold a per-session compaction_reduction value (HashMap<SessionId, i32> keyed by session) populated when StreamChunk::CompactionComplete arrives — mirrors TS AgentView.tsx:959-979 `setCompactionReductionRef.current?.(Math.round(result.compressionRatio))` (compressionRatio is already the percent of tokens removed [0,100]; round it directly — RPC-420).
  #   3. chrome_paint::paint_header_and_role MUST read the per-session compaction_reduction from the store and pass it as `Some(value)` into SessionHeader so build_right_line renders `[X%: COMPACTED Y%]` form — not hardcoded `None` as it is today at chrome_paint.rs:65.
  #   4. The compaction_reduction value MUST be computed as `compaction_result.compression_ratio.round() as i32` (wire value is percent removed — RPC-420) — same convention as format_compaction_notice in dispatch_slash_commands.rs, keeping notice text and badge suffix coherent.
  #   5. When SessionStateChange { state: Cleared } arrives via dispatch_stream_chunks.rs, the per-session TokenState AND compaction_reduction entry MUST be reset (TokenState back to Default, compaction_reduction removed) so the header shows `[0%]` for a freshly-cleared session — mirrors TS AgentView.tsx:992-1006.
  #   6. compaction_reduction is PER-SESSION: setting it on s-1 MUST NOT leak into s-2's header when the user cycles with Shift+Right (same per-session-storage invariant RPC-099 established for tokens).
  #   7. Color band selection (context_fill_color) MUST continue to honour the 50/70/85 thresholds for the now-u16 percentage value — values >=85 (including 100+, e.g. 105%) MUST land in the red bucket; no separate band for >100%.
  #
  # EXAMPLES:
  #   1. ContextFillUpdate with fill_percentage=105 arrives for session s-1. Render frame: header right-side text contains `tokens: ...↑ [105%]` rendered in red (>=85 bucket). Currently it shows `[100%]` due to the .min(100) clamp.
  #   2. Session s-1: ContextFillUpdate{fill_percentage=80} then CompactionComplete{original_tokens=10000, compacted_tokens=4000, compression_ratio=60.0, turns_summarized=12}. Header right-side text contains `[80%: COMPACTED 60%]` — the 60 comes from round(60.0), the wire value being percent removed (RPC-420).
  #   3. Two sessions: s-1 (CompactionComplete with ratio 70.0 → reduction 70%) and s-2 (no compaction yet). Header for s-1 shows `[X%: COMPACTED 70%]`. Shift+Right to focus s-2 → header for s-2 shows `[X%]` only (no COMPACTED suffix — s-1's value did NOT leak).
  #   4. Session s-1 has accumulated TokenUpdate{input:5000, output:1200}, ContextFillUpdate{fill_percentage=45}, and CompactionComplete{ratio:70.0}. Then SessionStateChange{state: Cleared} arrives. Next render: header shows `tokens: 0↓ 0↑` and `[0%]` (no COMPACTED suffix) — both TokenState and compaction_reduction were reset for that session.
  #   5. Color verification: ContextFillUpdate{fill_percentage=105} for session s-1. The rendered `[105%]` span MUST have foreground style Color::Red (matches context_fill_color band `pct >= 85`).
  #   6. Formula coherence: CompactionComplete{original_tokens=10000, compacted_tokens=4000, compression_ratio=60.0} → scrollback notice contains `60.0% reduction` AND header badge contains `COMPACTED 60%` — both derive directly from the percent-unit wire value 60.0 (RPC-420).
  #
  # ========================================
  Background: User Story
    As a developer using the Rust TUI
    I want to see the SessionHeader top-right [X%] badge accurately reflect the live context-fill percentage AND show the [X%: COMPACTED Y%] suffix immediately after a CompactionComplete event
    So that I know when my session is approaching compaction (>=85% red) AND I get the same visual COMPACTED feedback the TypeScript original provides — without losing values >100% that signal pre-compaction state

  Scenario: ContextFillUpdate above 100% renders the raw value (not clamped to 100)
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 105, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[105%]"
    And the SessionHeader text does NOT contain "[100%]"
    And the percentage span foreground style is Color::Red

  Scenario: CompactionComplete adds the COMPACTED suffix with reduction taken directly from the percent-unit compression_ratio
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 80, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[80%: COMPACTED 60%]"

  Scenario: compaction_reduction is per-session and does not leak across Shift+Right
    Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 50, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    And Action::ChunkReceived("s-2", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 50, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 3000, compression_ratio: 70.0, turns_summarized: 8 } }) has been dispatched
    When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    Then the SessionHeader text contains "COMPACTED 70%"
    When the App dispatches Action::SessionNext to focus "s-2" and re-renders
    Then the SessionHeader text contains "[50%]"
    And the SessionHeader text does NOT contain "COMPACTED"
    When the App dispatches Action::SessionPrev to focus "s-1" and re-renders
    Then the SessionHeader text contains "COMPACTED 70%"

  Scenario: SessionStateChange Cleared resets context_fill_pct, tokens, and compaction_reduction
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 5000, output_tokens: 1200, reasoning_tokens: None, tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 45, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 3000, compression_ratio: 70.0, turns_summarized: 8 } }) has been dispatched
    When Action::ChunkReceived("s-1", StreamChunk::SessionStateChange { state: SessionState::Cleared }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[0%]"
    And the SessionHeader text does NOT contain "COMPACTED"
    And the SessionHeader text contains "tokens: 0↓ 0↑"

  Scenario: Compaction notice line and header badge agree on the reduction percentage
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED 60%"
    And the scrollback contains a notice line containing "60.0% reduction"
