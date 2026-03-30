# Epic Review: NET-001 — SSE Disconnection Retry

**Date:** 2026-03-30
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 2 issues across 1 work unit
- 🟡 Warnings: 3 issues across 1 work unit
- 🟢 Observations: 4

## Work Unit Results

### NET-001: SSE Disconnection Retry — WARN

## 🔴 Critical Issues (Must Fix)

1. **DRY violation: `deep_search_handler.rs` duplicates retry constants and delay calculation**
   - **File:** `codelet/napi/src/deep_search_handler.rs:324,343`
   - Line 324 hardcodes `const MAX_RETRIES: u32 = 3` instead of importing `MAX_NETWORK_RETRIES` from `codelet_cli::interactive`.
   - Line 343 duplicates the delay formula (`1000u64 * 2u64.pow(network_retry_count.saturating_sub(1))`) instead of calling `network_retry_delay()`.
   - `compaction_retry.rs` correctly imports both from `recovery_network`. The NAPI crate already imports `is_transient_network_error` — it should also import the constant and delay function.
   - **Risk:** If the backoff strategy or retry budget changes in `recovery_network.rs`, `deep_search_handler.rs` will silently diverge.

2. **FinalResponse doesn't reset `network_retry_count` in `deep_search_handler.rs`**
   - **File:** `codelet/napi/src/deep_search_handler.rs:330-335`
   - The `FinalResponse` arm (line 330) is matched *before* the `Ok(_)` catch-all (line 333). Since Rust match arms are exclusive, `FinalResponse` events will NOT execute the `network_retry_count = 0` reset on line 335.
   - In contrast, `stream_loop.rs` explicitly resets the counter on `FinalResponse` (line 721), `Text` (609), `ToolCall` (621), and `Usage` (689).
   - Rule [2] requires "Retry counter resets on successful data receipt (Text, ToolCall, Usage, FinalResponse)".

## 🟡 Warnings (Should Fix)

1. **Missing `When` step in "Transient network error patterns are correctly detected" scenario**
   - **File:** `spec/features/sse-disconnection-retry.feature:111-117`
   - Scenario goes `Given → Then → And → And → And → And` with no `When` step.
   - Gherkin best practice requires Given/When/Then ordering.

2. **Coverage impl line range for DeepSearch scenario points to wrong lines**
   - Coverage for "Network retry works in DeepSearch sub-agent streams" points to `codelet/napi/src/deep_search_handler.rs:1-50` (file header/drop-guards), not the actual retry logic on lines 315-366.

3. **Test `test_detects_wrapped_agent_error` (line 44-53) has no corresponding scenario or @step comments**
   - Tests a valid edge case but is untracked in the feature file.

## 🟢 Observations (Nice to Have)

1. **`error_classifiers.rs:94`**: Pattern `lower.contains("sse error") && lower.contains("instance")` is oddly generic — "instance" could match unrelated errors. Worth a comment explaining what real error this targets.

2. **`recovery_network.rs:30`**: `2u64.pow(attempt.saturating_sub(1))` could overflow if MAX_NETWORK_RETRIES increases significantly. Current value of 3 is safe.

3. **Several test scenarios rely on comments rather than assertions for key steps** — e.g., "partial text preserved" test has zero assertions for its core steps. These are unit tests of classifier/delay, not integration tests of the full retry behavior. This is acceptable given the testing constraints but worth noting.

4. **Compaction retry cannot actually restart the stream** — it sleeps and calls `stream.next()` on an already-errored stream. The scenario says "recovers" but recovery here is fundamentally different from stream_loop's approach (which creates a fresh API call).

## Coverage Verification
- Feature file: `spec/features/sse-disconnection-retry.feature` — OK (10 scenarios, @NET-001 tag present)
- Test file: `codelet/cli/tests/network_retry_test.rs` — WARN (extra test without scenario tracing)
- Impl files: `stream_loop.rs`, `recovery_network.rs`, `error_classifiers.rs`, `compaction_retry.rs`, `deep_search_handler.rs` — ISSUE: deep_search_handler.rs coverage points to wrong lines
- Scenario coverage: 10/10 scenarios linked

## Files Reviewed
- spec/features/sse-disconnection-retry.feature
- codelet/cli/tests/network_retry_test.rs
- codelet/cli/src/interactive/recovery_network.rs
- codelet/cli/src/interactive/error_classifiers.rs (lines 60-120)
- codelet/cli/src/interactive/stream_loop.rs (lines 85-91, 600-730, 1185-1320)
- codelet/cli/src/interactive/compaction_retry.rs (lines 1-20, 300-350)
- codelet/cli/src/interactive/mod.rs (lines 1-50)
- codelet/napi/src/deep_search_handler.rs (lines 1-60, 310-366)
