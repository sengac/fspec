# BUG-122: persistenceGetHistory() Blocks UI for 3.8s

## Discovery Method

Used `@microsoft/tui-test` E2E framework to measure actual "Loading..." duration in a real PTY, then added `performance.now()` timers to `initSession()` and `initializeModels()`.

## Measured Timings (tui-test diagnosis)

**"Loading..." visible for 3,785ms on a brand new session via Enter key.**

```
[2026-03-18T02:42:34.635Z] "Loading..." APPEARED after 51ms
[2026-03-18T02:42:38.420Z] "Loading..." DISAPPEARED after 3836ms
VISIBLE DURATION: 3785ms
```

## Perf Breakdown (from fspec.log)

```
persistenceSetDataDirectory:      0.1ms
blocklistInit:                    4.8ms
initializeModels:                28.0ms  ← Models ready in 28ms
  loadCloudModels:               12.7ms
  buildCloudSections:             6.5ms
    checkCodexOAuthTokens:        0.6ms
    getProviderConfig(codex):     1.4ms
    checkClaudeOAuthTokens:       0.3ms
    Promise.all (102 providers):  3.4ms
    loadCodexAllowlist:           0.4ms
  loadProfileSections:            7.6ms
  loadPersistedModelString:       0.3ms
persistenceGetHistory:         3775.8ms  ← THE BOTTLENECK
initSession TOTAL:             3809.1ms
```

## Root Cause

In `AgentView.tsx` line ~1389, `initSession()` is a single async function that:

1. `persistenceSetDataDirectory()` — 0.1ms ✓
2. `blocklistInit()` — 4.8ms ✓
3. `initializeModels()` — 28ms ✓ (calls `setCurrentProvider()`)
4. `persistenceGetHistory()` — **3,776ms** ← blocks event loop
5. `setError(null)` — triggers final re-render

`setCurrentProvider()` is called at step 3, but React can't re-render because `persistenceGetHistory()` at step 4 is a **synchronous NAPI call** that blocks the JavaScript event loop for 3.8 seconds. React state updates are batched and only flush when the event loop is free.

So the model name is resolved at 28ms but doesn't appear in the header until 3,809ms — after the history call finishes and the function returns.

## Key Question: Why Does persistenceGetHistory() Even Run Here?

`persistenceGetHistory()` loads the last 100 session history entries for the current project. This populates the history dropdown in the agent input area.

But:
- This is not needed to display the model name
- This is not needed to create or resume a session
- This could load lazily (after the UI is interactive)
- This could run in a separate useEffect
- This could run in a web worker / off the main thread
- For a NEW session, there may be few/no history entries that matter

## Proposed Fixes

### Option A: Separate useEffect (simplest, no NAPI changes)
Move `persistenceGetHistory()` to its own `useEffect` that runs independently of model initialization. This lets React re-render with the model name immediately after `initializeModels()` completes.

### Option B: Defer to idle callback
Use `requestIdleCallback` or `setTimeout(fn, 0)` before the history call to yield to React's render cycle.

### Option C: Investigate why it takes 3.8 seconds
100 history entries should not take 3.8 seconds. The Rust implementation likely has a performance issue (reading too many files, O(N) scan, etc.).

### Option D: Combination
Fix the Rust performance issue AND separate the effects so model display is never blocked by history loading.

## Reproduction

```bash
npm run build
npx @microsoft/tui-test --trace e2e/loading-diagnosis.test.ts
cat /tmp/fspec-loading-diag.log
grep '\[PERF' ~/.fspec/fspec.log | tail -20
```
