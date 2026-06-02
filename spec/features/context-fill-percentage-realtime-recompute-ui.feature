@done
@tui
@rpc
@agent-view
@header
@RPC-101
Feature: Context Fill Percentage Realtime Recompute UI

  """
  TS UI side of the RPC-101 parallel fix.
  AgentView.tsx:1114-1175 — cachedContextThresholdRef (useRef<number>(0)) caches threshold from every ContextFillUpdate; updateTokenStateFromChunk on TokenUpdate recomputes pct = round((inputTokens - floor(cacheRead * 0.9)) / threshold * 100) and calls setContextFillPercentage when threshold>0.
  A backend ContextFillUpdate authoritatively overwrites the locally-recomputed value (it knows reasoning_tokens etc. the local recompute does not model).
  Non-positive incoming threshold does NOT wipe the cached good value.
  Tests: src/tui/__tests__/context-window-fill-percentage.test.tsx:536-678 (ink-testing-library + GlobalSessionStreamManager injectTestChunk).
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

