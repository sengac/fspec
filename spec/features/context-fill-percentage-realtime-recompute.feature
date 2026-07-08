@done
@RPC-419
@tui
@rpc
@store
@agent-view
@header
@RPC-101
Feature: Context Fill Percentage Realtime Recompute
  """
  RPC-101 ROOT CAUSE (recompute cadence — fixed, retained):
  • TS AgentView.tsx updateTokenStateFromChunk only called setContextFillPercentage on ContextFillUpdate, ignoring the high-cadence TokenUpdate stream.
  • Rust token_state.rs apply_chunk routed TokenUpdate to apply_token_tracker which never touched context_fill_pct.
  • Backend emits TokenUpdate frequently during streaming but ContextFillUpdate only at turn start/usage/final; ESC interrupt skips the terminal ContextFillUpdate, freezing the badge.
  RPC-419 ROOT CAUSE (recompute formula — corrected here):
  • RPC-101 lifted the WRONG formula: the compaction cost proxy TokenTracker::effective_tokens (codelet/core/src/compaction/model.rs:85-96), effective = input_tokens - floor(cache_read * 0.9). That is a cache-discounted billing/compaction-trigger metric, NOT context fill. The authoritative backend ContextFillUpdate uses physical occupancy with no discount.
  • Wire TokenTracker.input_tokens is ALREADY total_input = raw + cache_read + cache_creation (PROV-001, codelet/cli/src/interactive/output.rs:44,:61), so subtracting 90% of cache_read subtracted from a value that already includes 100% of it.
  • Rounding mismatch: local recompute used round() while the backend truncates (as u32), causing ±1% flicker even with a correct formula.
  • Result: badge sawtoothed within a turn (e.g. 110% <-> 24%) because bare TokenUpdates (stream_loop.rs:882,:940 text/reasoning deltas) overwrote the correct authoritative value with the cache-discounted, output-and-reasoning-excluding number.
  RPC-419 FIX (parallel TS + Rust, formula change only):
  • pct = trunc((input_tokens + output_tokens + reasoning_tokens) / threshold * 100) — no cache discount; wire input_tokens already includes cache tokens; missing optional fields treated as 0; truncation matches the backend's `as u32` cast.
  • Implementation sites: codelet/fspec-tui/src/store/agent_view/token_state.rs::apply_token_tracker and src/tui/components/AgentView.tsx::updateTokenStateFromChunk. Misleading comments claiming the old formula mirrored emit_context_fill_from_usage are corrected in both.
  • FORMULA AUTHORITY: emit_context_fill_from_usage (codelet/cli/src/interactive/stream_loop.rs:119-137) + ApiTokenUsage::total_context (codelet/core/src/token_usage.rs:64-72).
  • Convergence property: a bare TokenUpdate carrying the same usage as the preceding ContextFillUpdate recomputes to the identical percentage — no sawtooth, no ±1 flicker.
  RETAINED RPC-101/RPC-100/RPC-099 INVARIANTS:
  1. Cache the threshold (tokens) from every ContextFillUpdate: TS cachedContextThresholdRef, Rust TokenState.context_threshold_tokens. Non-positive / non-finite thresholds MUST NOT wipe a previously-cached good value.
  2. Skip recompute when threshold == 0 (no divide-by-zero; badge holds last value).
  3. Authoritative ContextFillUpdate from the backend MUST overwrite any locally-recomputed value.
  4. Overshoot >100% MUST be preserved (RPC-100 pre-compaction signal). NEVER clamp to 100.
  5. Per-session TokenState isolation (RPC-099).
  6. Restore path (session switch, resume): seed cache from extractTokenStateFromChunks().contextThreshold so the badge keeps updating after a session swap.
  INTEGRATION POINTS:
  • Wire (codelet/napi/index.d.ts): ContextFillInfo carries { fillPercentage, effectiveTokens, threshold, contextWindow }.
  • Rust store: TokenState.context_threshold_tokens; apply_token_tracker recomputes; apply_context_fill caches threshold.
  • TS UI: AgentView.tsx cachedContextThresholdRef; recompute inside updateTokenStateFromChunk on TokenUpdate; restore-path seeding via ExtractedTokenState.contextThreshold.
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

  Scenario: Authoritative ContextFillUpdate overrides any locally-recomputed value
    Given a session with cached threshold=100000 tokens after a ContextFillUpdate{fill_percentage=5}
    And a TokenUpdate with input_tokens=50000 has locally recomputed the badge to [50%]
    When the backend emits an authoritative ContextFillUpdate{fill_percentage=62} (the backend remains authoritative whenever it speaks)
    Then the SessionHeader badge MUST display [62%] (backend value wins)
    And the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates

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

  Scenario: Local recompute uses the backend physical-occupancy formula including output and reasoning tokens
    Given a session with cached threshold=100000 tokens (from a prior ContextFillUpdate)
    When a TokenUpdate with input_tokens=50000, output_tokens=3000 and reasoning_tokens=2000 arrives
    Then the SessionHeader badge MUST display [55%] computed as trunc((50000+3000+2000)/100000*100) with no cache discount applied

  Scenario: Cache-heavy TokenUpdate no longer collapses the badge (oscillation regression)
    Given a session with cached threshold=168000 tokens and an authoritative ContextFillUpdate showing 110%
    When a bare TokenUpdate arrives with input_tokens=175000 (including cache_read_input_tokens=150000 and cache_creation_input_tokens=5000), output_tokens=3000 and reasoning_tokens=8000
    Then the SessionHeader badge MUST remain [110%] computed as trunc(186000/168000*100) instead of collapsing to [24%]

  Scenario: Local recompute truncates like the backend instead of rounding
    Given a session with cached threshold=100000 tokens
    When a TokenUpdate with input_tokens=45900 and no output or reasoning tokens arrives
    Then the SessionHeader badge MUST display [45%] (truncation matching the backend's `as u32` cast, not [46%] from rounding)

  Scenario: Missing optional token fields are treated as zero
    Given a session with cached threshold=100000 tokens
    When a TokenUpdate with input_tokens=40000, output_tokens=1000 and absent reasoning and cache fields arrives
    Then the SessionHeader badge MUST display [41%] without error
