# AgentModal.tsx Rendering Optimization Analysis

This document outlines rendering optimization opportunities identified in AgentModal.tsx (~3900 lines).

---

## Implemented Optimizations

### PERF-002: Incremental Line Computation Cache ✅

**Location:** `conversationLines` useMemo

**Problem:** The `conversationLines` useMemo recalculated line wrapping for ALL messages whenever the conversation changed. During streaming, this meant:
- Re-running expensive `getVisualWidth()` calculations on every character of every message
- Re-splitting and re-wrapping all lines
- Creating new arrays for every update

**Solution:** Line wrapping results are cached per message in `lineCacheRef`. On each update:
- Only **new or changed messages** are recomputed
- Unchanged messages reuse their cached wrapped lines
- Cache is keyed by `(content, isStreaming, terminalWidth)`

**Impact:** During streaming, only the last (streaming) message is recomputed. For a conversation with 50 messages, this reduces computation by ~50x since we skip recomputing 49 unchanged messages.

---

### PERF-003: Deferred Conversation Updates ✅

**Location:** `useDeferredValue(conversation)`

**Problem:** During fast streaming, conversation updates could block user input handling.

**Solution:** Used React's `useDeferredValue` hook to mark conversation updates as lower priority than user interactions. This tells React's concurrent renderer that:
- User input (typing, key presses) should be handled immediately
- Conversation line updates can be slightly delayed if needed

**Impact:** More responsive UI during streaming - keyboard input remains snappy even when many updates are queued.

---

## Removed Optimization (Not Effective)

### ~~PERF-001: Throttled Text Streaming Updates~~ ❌

**Original Claim:** Text chunks were accumulated and flushed every 50ms, reducing re-renders from "hundreds per second" to "~20 per second".

**Why It Was Removed:**
1. **Ink already throttles renders** - Ink uses `throttle` from `es-toolkit` with a default maxFps of 30. Terminal output is already limited to ~30 writes/second regardless of how many state updates occur.
2. **Rust already batches chunks** - The code comment says "Text chunks are batched in Rust for efficiency", so chunks arrive less frequently than assumed.
3. **Marginal benefit** - The 50ms debounce reduced React reconciliation work, but this is already fast and the perceived difference was minimal.

**Verdict:** The manual throttling added complexity without meaningful user-visible improvement. Removed in favor of relying on Ink's built-in throttling.

---

## Additional Fixes

### VirtualList.tsx and ipc.ts: `setImmediate` → `setTimeout(0)` ✅

**Problem:** ESLint reports `setImmediate` as undefined because it's not in the configured globals.

**Solution:** Replaced `setImmediate` with `setTimeout(..., 0)` which has equivalent behavior for scheduling microtasks and is properly recognized by ESLint.

---

## Summary

| Optimization | Status | Real Impact |
|--------------|--------|-------------|
| PERF-001 (50ms throttle) | ❌ Removed | Marginal - Ink already throttles |
| PERF-002 (line cache) | ✅ Implemented | **Significant** - ~50x less computation for large conversations |
| PERF-003 (useDeferredValue) | ✅ Implemented | **Moderate** - Better input responsiveness |
| setImmediate fix | ✅ Implemented | Fixes ESLint errors |

---

## Why PERF-002 Is the Real Win

The line caching is genuinely impactful because:

1. **Cache hits are O(1)** - Just a Map lookup + array spread
2. **Cache misses are expensive** - `getVisualWidth()` for every character, word splitting, line wrapping logic
3. **During streaming, only the last message changes** - All other messages hit the cache
4. **For a 50-message conversation**: 49 cache hits (cheap) + 1 computation (expensive) vs 50 computations

The other optimizations (throttling, deferredValue) provide minor improvements but the line cache is where the real performance gain comes from.
