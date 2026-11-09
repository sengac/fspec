@BUG-582
@done
@RPC-419
@tui
@rpc
@store
@agent-view
@header
@RPC-101
@BUG-459
Feature: Context Fill Percentage Realtime Recompute
  """
  BUG-459: CMPCT-067 (coherent request context fill) supersedes the local
  recompute: the backend and TUI consume the context-fill snapshot and never
  recompute fill from cumulative display counters, so the local-recompute,
  cached-threshold and ContextFillUpdate-override scenarios are retired (see
  coherent-request-context-fill.feature).
  RPC-101 ROOT CAUSE (recompute cadence — fixed, retained):
  • TS AgentView.tsx updateTokenStateFromChunk only called setContextFillPercentage on ContextFillUpdate, ignoring the high-cadence TokenUpdate stream.
  • Rust token_state.rs apply_chunk routed TokenUpdate to apply_token_tracker which never touched context_fill_pct.
  • Backend emits TokenUpdate frequently during streaming but ContextFillUpdate only at turn start/usage/final; ESC interrupt skips the terminal ContextFillUpdate, freezing the badge.
  RPC-419 ROOT CAUSE (recompute formula — corrected here):
  • RPC-101 lifted the WRONG formula: the compaction cost proxy TokenTracker::effective_tokens (rust/core/src/compaction/model.rs:85-96), effective = input_tokens - floor(cache_read * 0.9). That is a cache-discounted billing/compaction-trigger metric, NOT context fill. The authoritative backend ContextFillUpdate uses physical occupancy with no discount.
  • Wire TokenTracker.input_tokens is ALREADY total_input = raw + cache_read + cache_creation (PROV-001, rust/cli/src/interactive/output.rs:44,:61), so subtracting 90% of cache_read subtracted from a value that already includes 100% of it.
  • Rounding mismatch: local recompute used round() while the backend truncates (as u32), causing ±1% flicker even with a correct formula.
  • Result: badge sawtoothed within a turn (e.g. 110% <-> 24%) because bare TokenUpdates (stream_loop.rs:882,:940 text/reasoning deltas) overwrote the correct authoritative value with the cache-discounted, output-and-reasoning-excluding number.
  RPC-419 FIX (parallel TS + Rust, formula change only):
  • pct = trunc((input_tokens + output_tokens + reasoning_tokens) / threshold * 100) — no cache discount; wire input_tokens already includes cache tokens; missing optional fields treated as 0; truncation matches the backend's `as u32` cast.
  • Implementation sites: rust/fspec-tui/src/store/agent_view/token_state.rs::apply_token_tracker and src/tui/components/AgentView.tsx::updateTokenStateFromChunk. Misleading comments claiming the old formula mirrored emit_context_fill_from_usage are corrected in both.
  • FORMULA AUTHORITY: emit_context_fill_from_usage (rust/cli/src/interactive/stream_loop.rs:119-137) + ApiTokenUsage::total_context (rust/core/src/token_usage.rs:64-72).
  • Convergence property: a bare TokenUpdate carrying the same usage as the preceding ContextFillUpdate recomputes to the identical percentage — no sawtooth, no ±1 flicker.
  RETAINED RPC-101/RPC-100/RPC-099 INVARIANTS:
  1. Cache the threshold (tokens) from every ContextFillUpdate: TS cachedContextThresholdRef, Rust TokenState.context_threshold_tokens. Non-positive / non-finite thresholds MUST NOT wipe a previously-cached good value.
  2. Skip recompute when threshold == 0 (no divide-by-zero; badge holds last value).
  3. Authoritative ContextFillUpdate from the backend MUST overwrite any locally-recomputed value.
  4. Overshoot >100% MUST be preserved (RPC-100 pre-compaction signal). NEVER clamp to 100.
  5. Per-session TokenState isolation (RPC-099).
  6. Restore path (session switch, resume): seed cache from extractTokenStateFromChunks().contextThreshold so the badge keeps updating after a session swap.
  INTEGRATION POINTS:
  • Wire (rust/napi/index.d.ts): ContextFillInfo carries { fillPercentage, effectiveTokens, threshold, contextWindow }.
  • Rust store: TokenState.context_threshold_tokens; apply_token_tracker recomputes; apply_context_fill caches threshold.
  • TS UI: AgentView.tsx cachedContextThresholdRef; recompute inside updateTokenStateFromChunk on TokenUpdate; restore-path seeding via ExtractedTokenState.contextThreshold.
  """

  Background: User Story
    As a developer using the Codelet TUI (both Rust rust/fspec-tui and TypeScript src/tui/)
    I want to see the SessionHeader [X%] context-fill badge update in real-time on every TokenUpdate (same cadence as the `tokens: X↓ Y↑` counters), AND have the last known percentage survive an ESC interrupt that skips the final ContextFillUpdate
    So that I can monitor approaching compaction live during streaming and after interrupts — instead of staring at a frozen percentage that only refreshes when the backend chooses to emit a ContextFillUpdate (end-of-turn or not at all on ESC)

  Scenario: TokenUpdate without prior ContextFillUpdate leaves the badge unchanged
    Given a fresh session with no ContextFillUpdate received yet (threshold cache is 0)
    When a TokenUpdate with input_tokens=50000 arrives
    Then the SessionHeader badge MUST remain at [0%] (no threshold means no recompute, never divide by zero)

  Scenario: Cache-heavy TokenUpdate no longer collapses the badge (oscillation regression)
    Given a session with cached threshold=168000 tokens and an authoritative ContextFillUpdate showing 110%
    When a bare TokenUpdate arrives with input_tokens=175000 (including cache_read_input_tokens=150000 and cache_creation_input_tokens=5000), output_tokens=3000 and reasoning_tokens=8000
    Then the SessionHeader badge MUST remain [110%] computed as trunc(186000/168000*100) instead of collapsing to [24%]
