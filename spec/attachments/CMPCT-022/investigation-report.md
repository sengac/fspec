# CMPCT-022 — PromptCancelled → Compaction Recovery Bug Cluster

## Investigation Date
2026-04-16

## Reported Symptom

> Sometimes when an LLM hits its maximum context window length while the agent is still taking turns, we hit a PromptCancelled error. The compaction system is supposed to handle this and remove the last message that crashed the prompt, perform the compaction and continue from the last message, but it doesn't.

## How the System Is Supposed to Work

### The Cancellation Pipeline

1. **`CompactionHook::on_completion_call`** (`codelet/core/src/compaction_hook.rs:161-224`) runs BEFORE each API call inside rig's streaming loop. It estimates payload tokens using `MAX(last_known_tokens, estimated_payload)`. If the effective total exceeds the threshold, it *atomically*:
   - Sets `state.compaction_needed = true`
   - Calls `cancel_sig.cancel()`

2. **Rig yields `PromptError::PromptCancelled { chat_history }`** at one of six cancel-check sites in `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs`:
   - Line 412 — after `on_completion_call` (before API request)
   - Line 460 — after `on_text_delta` (mid text stream)
   - Line 486 — after `on_tool_call` (before tool execution)
   - Line 509 — after `on_tool_result` (after tool execution)
   - Line 542 — after `on_tool_call_delta` (during tool-arg streaming)
   - Line 586 — after `on_stream_completion_response_finish` (end of turn)

3. **Stream loop catches the error** at `codelet/cli/src/interactive/stream_loop.rs:1131-1202`:
   ```rust
   let is_compaction_cancel = is_compaction_cancelled(&e);
   let compaction_triggered = token_state
       .lock()
       .map(|state| state.compaction_needed)
       .unwrap_or(false);

   if is_compaction_cancel && compaction_triggered {
       break;   // ← hook-triggered path
   }

   if is_prompt_too_long && has_compactable_turns {
       session.messages.pop();
       signal_compaction_needed(&token_state);
       break;   // ← API-returned-error path
   }
   ```

4. **Post-loop handler** at `stream_loop.rs:1500-1527` routes to `compaction_retry.rs::handle_compaction_retry` which runs `execute_compaction` (in-view DAG reconstruction, embeds original prompt), resets the token tracker, creates a fresh `CompactionHook`, and starts a new stream with the hardcoded `"Continue"` prompt at `compaction_retry.rs:128-133`.

5. **`run_retry_stream`** at `compaction_retry.rs:172-377` processes the retry stream with its own (incomplete) error cascade.

## Eight Bugs Identified

### 🔴 BUG 1 — Asymmetric Message Handling
**Covered by:** CMPCT-023

The pre-prompt, hook-cancel, API-error, and Gemini-continuation compaction paths all have subtly different `session.messages` transformations. The code is a DRY violation; bugs 2, 5, and 6 are direct consequences.

### 🔴 BUG 2 — Partial `assistant_text` Silently Discarded
**Covered by:** CMPCT-024

The hook-cancel `break` at `stream_loop.rs:1160` does NOT call `handle_final_response` to save accumulated text. Compare with Gemini continuation (`gemini_continuation.rs:334-337`), which does. Rig can cancel at 5 of 6 yield sites AFTER tokens have been emitted — so this is not hypothetical.

### 🔴 BUG 3 — Naive Substring on `Display`
**Covered by:** CMPCT-025 (complements PROV-045)

`is_compaction_cancelled` at `error_classifiers.rs:115-117` uses `error.to_string().contains("PromptCancelled")`. No `source()` chain walk, no structural downcast. Silently fails if the error is wrapped with any `.context(...)`.

### 🔴 BUG 4 — Fragile `&& compaction_triggered` Guard
**Covered by:** CMPCT-026

The conjunction at `stream_loop.rs:1156` requires two independent pieces of evidence to agree. If `token_state` has been replaced (lines 1062, 1254, 1359 create fresh retry states) or if rig were to cancel from a non-`on_completion_call` site, the flag is false and PromptCancelled cascades all the way to line 1447 — terminating the session.

### 🔴 BUG 5 — Incomplete Error Cascade in `run_retry_stream`
**Covered by:** CMPCT-027

`run_retry_stream` only handles `is_transient_network_error`. It does NOT handle prompt-too-long, truncation, image-content, compaction-cancel, or stall-timeout — all of which ARE handled in the primary stream. If the compacted context is still too large, the retry dies hard.

### 🟡 BUG 6 — Token Tracker Not Updated Before Cancel Break
**Covered by:** CMPCT-024 (bundled with BUG 2)

CMPCT-002 Rule [2] mandates: *"Token tracker MUST be updated with cumulative billing before signaling compaction."* Gemini continuation does this. Main stream loop does not.

### 🟡 BUG 7 — Hardcoded `"Continue"` Prompt Semantically Wrong
**Covered by:** CMPCT-028

When the hook cancels on `on_completion_call` (most common), the user's prompt was never sent. The retry sends `"Continue"` to the LLM — meaningful only because `execute_compaction` embeds the prompt in the instruction text. Any variation in that embedding path silently drops the user's request.

### 🟡 BUG 8 — `PromptCancelled.chat_history` Always Dropped
**Covered by:** CMPCT-029 (complements PROV-050)

At cancel sites 486/509, rig has accumulated `tool_calls`/`tool_results` in local vecs that are flushed to `chat_history` only if the while-loop completes normally. If PromptCancelled is yielded mid-loop, those are neither in `session.messages` nor in rig's `chat_history`. The `chat_history` field in the error variant is then dropped unread.

## Test Coverage Gap
**Covered by:** CMPCT-030

The existing tests are almost entirely pure-function decision tests. The full cycle `PromptCancelled → compaction → retry` has **no test**. `gemini_continuation_compaction_test.rs` is literally:
```rust
let error_str = "PromptCancelled";
let is_compaction_cancel = error_str.contains("PromptCancelled");
assert!(is_compaction_cancel);
```
— tautological and not invoking any production code.

Missing scenarios:
1. Full end-to-end stream → PromptCancelled → compaction → retry
2. PromptCancelled during a tool call
3. Last-user-message pop verification
4. `compaction_needed=false` but PromptCancelled fires (BUG 4)
5. Nested / wrapped PromptCancelled errors (BUG 3)
6. Partial-text preservation assertion (BUG 2)
7. Cascading compaction in retry stream (BUG 5)
8. Equivalence between pre-prompt and post-cancel compaction paths
9. Circuit breaker for repeated compactions

## Top-Level Code Smells

1. **Two-signal guard pattern** — `is_compaction_cancel && compaction_triggered` uses two sources of evidence when one should suffice.
2. **String-typed error classification** — every classifier matches substrings against `Display`; `PromptError` is a typed enum.
3. **Asymmetric dual-path control flow** — pre-prompt, post-cancel, and Gemini-continuation compaction paths should be unified.
4. **Tests document behavior instead of verifying it** — placeholder tautologies provide false confidence.
5. **Retry stream is a weak copy of primary stream** — handler logic duplicated minus most error cascade branches.
6. **Hidden coupling via shared `Arc<Mutex<TokenState>>`** — multiple recovery paths replace the mutex mid-flight.

## Related Existing Work
- **PROV-042** — Parent card for broader stream-loop resilience (VTCode-inspired patterns)
- **PROV-045** — Replace string-matching classifiers with typed `StreamErrorKind` enum (broader than CMPCT-025)
- **PROV-050** — Split-safe compaction ensuring tool call/result pairs are never broken (complements CMPCT-029)
- **CMPCT-002** — Handle Gemini continuation + compaction gracefully (marked done, but tests are tautological — CMPCT-030 fixes this)

## Files Referenced
- `codelet/cli/src/interactive/stream_loop.rs` — main stream loop
- `codelet/cli/src/interactive/compaction_retry.rs` — post-cancel retry
- `codelet/cli/src/interactive/error_classifiers.rs` — error classification helpers
- `codelet/cli/src/interactive/gemini_continuation.rs` — Gemini continuation loop (reference implementation)
- `codelet/cli/src/interactive_helpers.rs` — `execute_compaction`
- `codelet/core/src/compaction_hook.rs` — `CompactionHook::on_completion_call`
- `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` — rig's cancel-check sites
- `codelet/patches/rig-core/src/completion/request.rs` — `PromptError::PromptCancelled` definition
- `codelet/cli/tests/gemini_continuation_compaction_test.rs` — tautological placeholder tests
- `codelet/cli/tests/prompt_too_long_recovery_test.rs` — existing (partial) coverage

## Recommended Execution Order

1. **CMPCT-025** (structural classification) — foundation; unblocks CMPCT-026
2. **CMPCT-026** (single source of truth) — depends on CMPCT-025
3. **CMPCT-024** (preserve partial text + token tracker) — independent
4. **CMPCT-023** (unify entry paths) — can happen after 024, 025, 026
5. **CMPCT-027** (retry stream error cascade) — independent
6. **CMPCT-028** (retry prompt semantics) — needs CMPCT-023
7. **CMPCT-029** (tool-call cancel state) — complements PROV-050
8. **CMPCT-030** (test coverage) — should land alongside every other card to prevent regression
