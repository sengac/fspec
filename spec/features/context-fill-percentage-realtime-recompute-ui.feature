@done
@RPC-419
@tui
@rpc
@agent-view
@header
@RPC-101
Feature: Context Fill Percentage Realtime Recompute UI
  """
  TS UI side of the RPC-101 parallel fix, formula corrected by RPC-419.
  AgentView.tsx — cachedContextThresholdRef (useRef<number>(0)) caches threshold from every ContextFillUpdate; updateTokenStateFromChunk on TokenUpdate recomputes pct = Math.trunc((inputTokens + outputTokens + reasoningTokens) / threshold * 100) and calls setContextFillPercentage when threshold>0.
  RPC-419: the original RPC-101 formula (inputTokens - floor(cacheRead * 0.9), rounded) was the compaction cost proxy, not physical occupancy — it caused the badge to sawtooth during streaming. Wire inputTokens already includes cache tokens (PROV-001); no cache discount is applied; truncation matches the backend's `as u32` cast; missing optional fields are treated as 0. See spec/features/context-fill-percentage-realtime-recompute.feature for the full root cause and formula authority (emit_context_fill_from_usage + ApiTokenUsage::total_context).
  A backend ContextFillUpdate authoritatively overwrites the locally-recomputed value. Non-positive incoming threshold does NOT wipe the cached good value. Overshoot >100% is never clamped.
  Tests: src/tui/__tests__/context-window-fill-percentage.test.tsx (ink-testing-library + GlobalSessionStreamManager injectTestChunk).
  """

  Background: User Story
    As a developer watching the SessionHeader badge in the TypeScript Ink AgentView
    I want the [X%] badge to recompute live on every TokenUpdate chunk (not just on ContextFillUpdate)
    So that the badge tracks tokens: X↓ Y↑ at the same cadence during streaming and survives ESC interrupt

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

  Scenario: Badge shows physical occupancy including output and reasoning during streaming
    Given a session with cached threshold=100000 tokens
    When a TokenUpdate with input_tokens=50000, output_tokens=3000 and reasoning_tokens=2000 arrives during streaming
    Then the SessionHeader renders [55%] with no cache discount applied
