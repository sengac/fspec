# CMPCT-032 — Research Findings: Compaction Triggering Broken

**Date:** 2026-04-17
**Reporter:** Claude Code (via session da7dbdc0-5e24-4a84-b66c-c429718c8f42)
**Scope:** commit `dc3d5934 "fix: compaction and mouse fixes"` (implementing CMPCT-023..031 + CTX-007)

## Summary

The compaction refactor in CMPCT-023..031 **deleted the post-loop compaction block** and moved recovery into an in-loop macro (`in_loop_compaction_restart!()`) that only fires from the ERROR arm of `stream.next()`. As a result, any stream-loop exit path that finishes via `Ok(FinalResponse)` — or any other non-error exit — can leave `token_state.compaction_needed = true` unhandled. In release builds there is no production-mode safety net; only a `#[cfg(debug_assertions)]` log remains.

This is a **silent** regression: users will simply see the next API call fail with `prompt too long` instead of the previous "compaction-and-retry" behaviour.

## What Changed (Per Card)

| Card | Change |
|------|--------|
| CMPCT-023 | All entry paths (B=prompt-too-long, C=PromptCancelled, D=Gemini cont.) funnel through `begin_compaction_recovery()` in `codelet/cli/src/interactive/recovery_compaction.rs` |
| CMPCT-024 | `flush_partial_state_before_compaction()` saves partial Assistant text + token tracker before compacting |
| CMPCT-025 | `extract_prompt_cancelled` walks the `anyhow::Error` chain for typed `PromptError::PromptCancelled` (+ `Box<PromptError>`) instead of substring matching `"PromptCancelled"` |
| CMPCT-026 | `classify_compaction_branch()` in `codelet/cli/src/interactive/error_classifiers.rs` returns `Recover` if typed `PromptCancelled` is in chain OR `compaction_needed=true`. The `&&` guard is gone. |
| CMPCT-027 | **`codelet/cli/src/interactive/compaction_retry.rs` was DELETED.** The post-loop `handle_compaction_retry` call was replaced by an in-loop `in_loop_compaction_restart!()` macro. |
| CMPCT-028 | Retry prompt changed from hardcoded `"Continue"` to `CompactionRecoveryPolicy` → either `"Continue"` (EmbedInInstruction) or `"Please continue from where you left off before the context limit was reached."` (ResumeFromPartial) |
| CMPCT-029 | Rig patch at `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:608` flushes pending tool_call+tool_result into `chat_history` BEFORE `PromptCancelled` is yielded. stream_loop then calls `reconcile_session_messages` + `inject_synthetic_tool_results_for_orphans` before `begin_compaction_recovery` |
| CMPCT-031 | Rig patch bounds tool_result text at 64 KiB (`bound_tool_result_text`) |
| CTX-007 | Per-model threshold via `resolve_compaction_threshold()` in `codelet/cli/src/compaction_threshold.rs` (priority: user override → builtin family default → base formula). Built-in defaults: Claude = base formula, Gemini/OpenAI = 80% of context window |

## The Critical Regression

### Old system (pre-commit dc3d5934)

1. `CompactionHook::on_completion_call` fires before each API call. If tokens > threshold, it sets `TokenState.compaction_needed = true` and calls `cancel_sig.cancel()`.
2. Rig yields `PromptError::PromptCancelled` from one of 6 cancel sites in `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs`.
3. The error arm in `stream_loop.rs` used a stringly-typed substring check for `"PromptCancelled"`, AND required `state.compaction_needed == true`.
4. On `break`, the **post-loop block** in `stream_loop.rs` executed `handle_compaction_retry()` (from the now-deleted `compaction_retry.rs`), which ran `execute_compaction`, built a fresh hook + stream, and processed a retry stream via `run_retry_stream()`. This **covered every break path**, not just the error arm.

### New system (post-commit dc3d5934)

Recovery is now entirely in-loop, via `in_loop_compaction_restart!()`. The macro is invoked from:
- Path B (prompt-too-long error)
- Path C (PromptCancelled error)
- Path D (Gemini continuation exhaustion)

All three invocation sites are **inside the `Some(Err(e)) => {}` branches** of the `stream.next()` match. The `Some(Ok(FinalResponse))` branch at `stream_loop.rs:968-1295` does NOT check `token_state.compaction_needed`.

The post-loop block (`stream_loop.rs:1777-1798`) was gutted:

```rust
// CMPCT-027: The post-loop `handle_compaction_retry` call that used to
// live here has been removed. [...] Reaching this point with
// `compaction_needed == true` would mean a loop exited without calling
// the macro — that is a bug.
#[cfg(debug_assertions)]
{
    if let Ok(state) = token_state.lock() {
        if state.compaction_needed && !is_interrupted.load(Acquire) {
            debug!("[stream_loop] POST-LOOP: compaction_needed=true but loop exited …");
        }
    }
}
```

**There is no production handling. The comment itself admits: "that is a bug."**

### Concrete missed paths

1. **`Some(Ok(FinalResponse))` branch at `codelet/cli/src/interactive/stream_loop.rs:968-1295`.**
   This branch runs:
   - Gemini continuation handling
   - Thinking-exhaustion retry
   - `stop_reason` checks
   - `handle_final_response`
   - `emit_done_with_stop_reason`
   - `break`

   None of it checks `token_state.compaction_needed`. If rig's `on_stream_completion_response_finish` runs the hook and the hook sets the flag — but the stream yields `FinalResponse` rather than `PromptCancelled` (e.g., because the API returned a complete response before cancellation propagated, or because the hook's cancel happened on the post-completion callback, not pre-call) — the loop `break`s at line 1295 with `compaction_needed=true` and falls out into the dead debug-only post-loop block.

2. **Any other non-error exit path**: interruption, tool cancellation, normal stream end where a late-firing post-call hook sets the flag.

### Why the diagnostic is useless

The debug-only `tracing::debug!` in the post-loop block:
- Is excluded from release builds (`#[cfg(debug_assertions)]`)
- Does not trigger compaction
- Does not emit a user-visible error
- Does not set an exit code

In production, users experience **silent context-window exhaustion** the next time they send a message.

## Evidence (Files)

| File | Lines of interest | Evidence |
|------|-------------------|----------|
| `codelet/cli/src/interactive/stream_loop.rs` | 336 (emit_compaction_started), 968-1295 (FinalResponse branch), 1777-1798 (gutted post-loop block) | Post-loop is debug-only |
| `codelet/cli/src/interactive/recovery_compaction.rs` | 222 (begin_compaction_recovery docs) | New unified entry point |
| `codelet/cli/src/interactive/error_classifiers.rs` | `classify_compaction_branch`, `extract_prompt_cancelled` | Classifier only invoked from error arm |
| `codelet/core/src/compaction_hook.rs` | 76-155 (CompactionHook struct + impl) | `on_completion_call` sets flag + cancels |
| `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` | 412, 460, 494-496, 512, 608 | 6 PromptCancelled yield sites |
| `codelet/cli/src/compaction_threshold.rs` | `resolve_compaction_threshold` | Per-model threshold |
| `codelet/core/src/compaction/model.rs` | TokenState, TokenTracker | `compaction_needed` field |

## Required Fix (Approach)

1. **Reinstate a production-mode post-loop safety net.** When the loop exits with `token_state.compaction_needed == true` and the session is not interrupted, funnel into `begin_compaction_recovery()` with the appropriate `CompactionRecoveryPolicy` (likely `EmbedInInstruction("Continue")` for clean exits).
2. **Check the flag in the `FinalResponse` branch BEFORE emitting done.** If compaction is needed, perform recovery and emit an in-loop restart rather than terminating the turn.
3. **Keep the in-loop macro for the error arms.** The error-arm handling from CMPCT-023..028 is correct — do not revert it.
4. **Add a production-mode warning log** (not debug-only) if the post-loop safety net is ever reached, indicating WHICH branch missed the check.

## Integration Test Requirements (Non-Negotiable)

Tests MUST exercise **every stream-loop exit path** where the flag can be set:

- **T1 — Error arm, PromptCancelled:** hook sets flag, stream yields PromptCancelled → `in_loop_compaction_restart!` fires, `execute_compaction` called, fresh stream built, retry sent with correct policy prompt. (Regression guard for CMPCT-023..028.)
- **T2 — Error arm, prompt-too-long:** upstream API returns 400 prompt-too-long → macro fires → recovery.
- **T3 — Error arm, Gemini continuation:** Gemini exhaustion error → macro fires → recovery.
- **T4 — FinalResponse branch with flag set (THE REGRESSION):** hook sets flag on last chunk before FinalResponse; stream yields Ok(FinalResponse). MUST trigger recovery and restart, NOT terminate.
- **T5 — Post-loop safety net:** simulate a break path that bypasses both the error arms and the FinalResponse check (e.g., thinking-exhaustion retry that breaks). MUST trigger recovery.
- **T6 — Interrupt takes priority:** if `is_interrupted=true`, no recovery is attempted regardless of flag state.
- **T7 — Flag clears after recovery:** after `execute_compaction` runs, `compaction_needed` returns to false and the restart stream does NOT re-trigger.
- **T8 — Per-model threshold honoured:** Gemini (80%), OpenAI (80%), Claude (base formula), user override — the hook fires at the correct boundary for each.

All tests must use the real `stream_loop` entry point with a mocked `Stream` (not a unit test of a helper). Integration tests go in `codelet/cli/tests/` (or equivalent). Use `cargo test` to verify they fail on the current codebase and pass after the fix.

## Related Work Units

- Parent epic: `hierarchical-compaction`
- Preceding cards: CMPCT-023, CMPCT-024, CMPCT-025, CMPCT-026, CMPCT-027, CMPCT-028, CMPCT-029, CMPCT-031, CTX-007

## Session Context Reference

Supervisor session: `da7dbdc0-5e24-4a84-b66c-c429718c8f42`
Turns 0-148 contain full investigation trace (git log, file reads, code analysis).
