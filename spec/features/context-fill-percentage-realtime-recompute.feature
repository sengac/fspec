@done
@tui
@rpc
@store
@agent-view
@header
@RPC-101
Feature: Context Fill Percentage Realtime Recompute

  """
  ROOT CAUSE:
  • TS AgentView.tsx:1112-1125 updateTokenStateFromChunk only called setContextFillPercentage on ContextFillUpdate, ignoring the high-cadence TokenUpdate stream.
  • Rust token_state.rs:42-50 apply_chunk routed TokenUpdate to apply_token_tracker which wrote input/output/cache/reasoning/tps but never touched context_fill_pct.
  • Backend emits TokenUpdate frequently during streaming but ContextFillUpdate only at end-of-turn; ESC interrupt skips the terminal ContextFillUpdate entirely, leaving the badge frozen on a stale value.
  FIX (parallel TS + Rust):
  1. Cache the threshold (tokens) from every ContextFillUpdate: TS `cachedContextThresholdRef`, Rust `TokenState.context_threshold_tokens`. Non-positive / non-finite thresholds MUST NOT wipe a previously-cached good value (older fixtures emit ContextFillUpdate with threshold: 0.0).
  2. Recompute on every TokenUpdate using the same formula as `emit_context_fill_from_usage` (codelet/cli/src/interactive/stream_loop.rs:108-126) and TokenTracker::effective_tokens (codelet/core/src/compaction/model.rs:90-96): effective = input_tokens - floor(cache_read * 0.9); pct = round(effective / threshold * 100). Skip recompute when threshold == 0.
  3. Authoritative ContextFillUpdate from the backend MUST overwrite any locally-recomputed value (backend knows reasoning_tokens and other adjustments the local recompute doesn't model).
  4. Overshoot >100% MUST be preserved (RPC-100 invariant — used as pre-compaction signal). NEVER clamp to 100.
  5. Restore path (session switch, resume): seed cache from extractTokenStateFromChunks().contextThreshold so the badge keeps updating after a session swap.
  INTEGRATION POINTS:
  • Wire (codelet/napi/index.d.ts:2952-2957): ContextFillInfo carries { fillPercentage, effectiveTokens, threshold, contextWindow } — TS interface widened to expose all four.
  • Rust store: TokenState gains `context_threshold_tokens: u64`; `apply_token_tracker` recomputes; `apply_context_fill` caches threshold.
  • TS UI: AgentView.tsx adds `cachedContextThresholdRef`, recomputes inside `updateTokenStateFromChunk` on TokenUpdate; seeds cache at both session-restore sites (~3664, ~4290).
  • ExtractedTokenState gains `contextThreshold: number | null` so restore-path priming is structural, not magic.
  """

  Background: User Story
    As a developer using the Codelet TUI (both Rust codelet/fspec-tui and TypeScript src/tui/)
    I want to see the SessionHeader [X%] context-fill badge update in real-time on every TokenUpdate (same cadence as the `tokens: X↓ Y↑` counters), AND have the last known percentage survive an ESC interrupt that skips the final ContextFillUpdate
    So that I can monitor approaching compaction live during streaming and after interrupts — instead of staring at a frozen percentage that only refreshes when the backend chooses to emit a ContextFillUpdate (end-of-turn or not at all on ESC)


  Scenario: TokenUpdate after cached threshold recomputes the badge locally without a new ContextFillUpdate
    Given a session has received ContextFillUpdate with fill_percentage=10 and threshold=100000 tokens
    When a TokenUpdate with input_tokens=45000 arrives without an accompanying ContextFillUpdate
    Then the SessionHeader badge MUST display [45%] (recomputed locally from 45000/100000)
    When a further TokenUpdate with input_tokens=90000 arrives later in the same turn
    Then the SessionHeader badge MUST display [90%] at TokenUpdate cadence


  Scenario: TokenUpdate without prior ContextFillUpdate leaves the badge unchanged
    Given a fresh session with no ContextFillUpdate received yet (threshold cache is 0)
    When a TokenUpdate with input_tokens=50000 arrives
    Then the SessionHeader badge MUST remain at [0%] (no threshold means no recompute, never divide by zero)


  Scenario: Local recompute applies the 90% cache discount to cache_read_input_tokens
    Given a session with cached threshold=100000 tokens (from a prior ContextFillUpdate)
    When a TokenUpdate with input_tokens=50000 and cache_read_input_tokens=20000 arrives
    Then the SessionHeader badge MUST display [32%] computed as effective=50000-(20000*0.9)=32000 and pct=round(32000/100000*100)=32 (matches TokenTracker.effective_tokens)


  Scenario: Authoritative ContextFillUpdate overrides any locally-recomputed value
    Given a session with cached threshold=100000 tokens after a ContextFillUpdate{fill_percentage=5}
    When the backend emits an authoritative ContextFillUpdate{fill_percentage=62} (it knows about reasoning_tokens the local recompute does not model)
    Then the SessionHeader badge MUST display [62%] (backend value wins)
    Given a TokenUpdate with input_tokens=50000 has locally recomputed the badge to [50%]
    Then the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates


  Scenario: Local recompute preserves overshoot above 100% (RPC-100 invariant)
    Given a session with cached threshold=100000 tokens
    When a TokenUpdate with input_tokens=110000 arrives
    Then the SessionHeader badge MUST display [110%] (NOT clamped to 100 — the pre-compaction overshoot signal is preserved)


  Scenario: ContextFillUpdate with non-positive threshold does not wipe a previously-cached good threshold
    Given a session has received ContextFillUpdate{fill_percentage=50, threshold=100000}
    When a subsequent fixture-style ContextFillUpdate{fill_percentage=60, threshold=0.0} arrives (older fixtures only set fill_percentage)
    Then the badge MUST display [60%] from the new fill_percentage
    Then the cached threshold MUST remain at 100000 tokens (non-positive threshold MUST NOT erase cached value)
    When a TokenUpdate with input_tokens=75000 arrives
    Then the badge MUST recompute against the cached threshold and display [75%]

