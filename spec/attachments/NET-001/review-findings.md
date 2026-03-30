# Epic Review: NET-001 — SSE Disconnection Retry

**Date:** 2026-03-30
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 4 issues across 1 work unit (all fixed)
- 🟢 Observations: 6

## Work Unit Results

### NET-001: SSE Disconnection Retry — PASS (after fixes)

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Fixed)

1. **No "reconnected" UX feedback after successful retry recovery**
   - **Files:** `codelet/cli/src/interactive/stream_loop.rs`, `compaction_retry.rs`, `src/tui/components/AgentView.tsx`
   - **Problem:** After emitting "Network error (attempt 1/3). Retrying in 1.0s..." the user saw nothing when recovery succeeded — model just silently started streaming again. Multiple retries accumulated permanent messages in conversation.
   - **Fix:** Redesigned UX to emit single "⟳ Reconnecting..." on first retry, "✓ Reconnected" on recovery, "✗ Reconnection failed" on exhaustion. TUI uses replace-in-place semantics so only one message transitions.

2. **ReasoningDelta arm did not reset `network_retry_count`**
   - **File:** `codelet/cli/src/interactive/stream_loop.rs:648`, `compaction_retry.rs:246`
   - **Problem:** Rule [2] lists "Text, ToolCall, Usage, FinalResponse" but ReasoningDelta IS valid successful data receipt. A network error after receiving reasoning but before text would not have reset the counter.
   - **Fix:** Added `network_retry_count = 0` and reconnection feedback to ReasoningDelta arm in both stream_loop.rs and compaction_retry.rs.

3. **Per-retry status messages cluttered conversation**
   - **Files:** `stream_loop.rs:1203`, `compaction_retry.rs:320`
   - **Problem:** Each retry attempt emitted a separate "Network error (attempt X/Y). Retrying in Z.Xs..." message, accumulating in the conversation permanently.
   - **Fix:** Changed to emit "⟳ Reconnecting..." only on first attempt. Details preserved in tracing logs.

4. **Exhaustion message was verbose and redundant**
   - **File:** `stream_loop.rs:1278`
   - **Problem:** "Network error persists after 3 retries — giving up" was followed by the actual error. Redundant.
   - **Fix:** Changed to concise "✗ Reconnection failed" which replaces the "⟳ Reconnecting..." in conversation.

## 🟢 Observations

1. **Compaction retry has weaker recovery** — can't restart the stream (no access to agent), just continues polling. Acknowledged in code comment. Asymmetry with stream_loop is a design constraint.
2. **DeepSearch retry has no `is_interrupted` check** — `is_interrupted` flag not available in that scope. Sub-agents are independently managed.
3. **Tests are unit-level** — verify classifier/delay functions, not full retry loop integration. Acceptable for 5-point estimate.
4. **Shared constants properly centralized** — `MAX_NETWORK_RETRIES` and `network_retry_delay()` imported by all 3 sites.
5. **No `unwrap()`, `todo!()`, or `unimplemented!()` in production code** — verified.
6. **Feature file has excellent structure** — architecture docstring, background user story, example mapping context, proper tags.

## Coverage Verification
- Feature file: `spec/features/sse-disconnection-retry.feature` — OK
- Test file: `codelet/cli/tests/network_retry_test.rs` — OK (11 tests, all pass)
- Impl files: stream_loop.rs, recovery_network.rs, error_classifiers.rs, compaction_retry.rs, deep_search_handler.rs — OK
- Scenario coverage: 10/10 scenarios covered

## Fix Results

### NET-001: SSE Disconnection Retry
- 🟡 Issue 1 (No reconnection UX feedback) → ✅ Fixed: Added `network_retry_in_progress` flag, "⟳ Reconnecting..."/"✓ Reconnected"/"✗ Reconnection failed" messages with TUI replace semantics
- 🟡 Issue 2 (ReasoningDelta missing reset) → ✅ Fixed: Added counter reset and reconnection feedback to ReasoningDelta arms
- 🟡 Issue 3 (Per-retry message clutter) → ✅ Fixed: Single "⟳ Reconnecting..." on first attempt only
- 🟡 Issue 4 (Verbose exhaustion message) → ✅ Fixed: Changed to "✗ Reconnection failed"

## Final Verification
- All Rust tests pass: ✅ (11/11)
- All TUI tests pass: ✅ (AgentView: 19/19, NAPI-010: 11/11, Resume: 29/29)
- Cargo check clean (no warnings): ✅
- Full build succeeds: ✅
- Feature file valid: ✅

## Files Modified
1. `codelet/cli/src/interactive/stream_loop.rs` — Added `network_retry_in_progress` flag, reconnection feedback, ReasoningDelta counter reset
2. `codelet/cli/src/interactive/compaction_retry.rs` — Same pattern as stream_loop
3. `src/tui/components/AgentView.tsx` — Replace semantics for reconnection status messages (3 handlers)
