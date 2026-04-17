# CMPCT-027 AST Research — In-Loop Compaction Restart Sites

## Scope

Identify every site in the CLI interactive streaming loop where the old
post-loop `handle_compaction_retry` / `run_retry_stream` flow must be replaced
with an in-loop restart via `begin_compaction_recovery` +
`execute_compaction_and_capture_events` + `stream = agent.prompt_streaming...`
+ `continue`.

## Existing call sites of `begin_compaction_recovery`

Gathered via AstGrep (`pattern: "begin_compaction_recovery"`, `language: rust`):

| File | Line | Role |
|------|------|------|
| `codelet/cli/src/interactive/stream_loop.rs` | 1170 | Path C — PromptCancelled (hook-cancel) |
| `codelet/cli/src/interactive/stream_loop.rs` | 1224 | Path B — API `prompt is too long` |
| `codelet/cli/src/interactive/gemini_continuation.rs` | 352 | Path D — Gemini continuation cancel |
| `codelet/cli/src/interactive/recovery_compaction.rs` | 140 | Helper definition |
| `codelet/cli/src/interactive/mod.rs` | 48 | Public re-export |

All three call sites are immediately followed today by:

- Paths B & C: `break;` → falls to post-loop `handle_compaction_retry` (at
  `stream_loop.rs:1544-1559`).
- Path D: `return Ok(GeminiContinuationResult::CompactionNeeded);` → bubbles
  back to `stream_loop.rs` which also `break`s to the same post-loop handler.

## Existing handle_compaction_retry / run_retry_stream

Neither `fn handle_compaction_retry` nor `fn run_retry_stream` match on an
AstGrep function-name pattern (they are `async fn` with generic bounds that
the naive pattern does not capture), but grep confirms they live in
`codelet/cli/src/interactive/compaction_retry.rs` lines 36 and 173 respectively.

`handle_compaction_retry` performs three responsibilities:

1. `execute_compaction(session, compaction_in_progress, Some(prompt))` + debug
   capture events (`compaction.triggered`, `context.update`,
   `compaction.failed`).
2. Build a fresh `Arc<Mutex<TokenState>>` + `CompactionHook` for the retry.
3. Call `run_retry_stream` with the new stream.

`run_retry_stream` is the module with the stunted error cascade that is the
subject of this card.

## Call graph after CMPCT-027

After refactoring, the single unified flow is (all paths):

```text
begin_compaction_recovery(...)            // Paths B/C/D (flush state)
  └─> execute_compaction_and_capture_events(...)   // NEW helper
        └─> execute_compaction(...)                  // unchanged
  └─> reset token_state in place
  └─> CompactionHook::new(Arc::clone(&token_state), threshold)
  └─> stream = agent.prompt_streaming_with_history_and_hook("Continue", ...)
  └─> reset per-turn locals
  └─> continue;   // stay in primary loop → full error cascade governs
```

`handle_compaction_retry` and `run_retry_stream` are removed. The post-loop
`if compaction_needed && !is_interrupted { ... handle_compaction_retry(...) }`
block at `stream_loop.rs:1544-1559` becomes dead code and is deleted.

## Circuit breaker placement

`compaction_retry_count` is declared alongside `truncation_retry_count`,
`thinking_exhaustion_retry_count`, and `network_retry_count` near the top of
`run_agent_stream_internal` (around `stream_loop.rs:498-505`). It is
incremented *before* each cascaded compaction. Budget is `MAX_COMPACTION_RETRIES = 3`.

## Risk: token_state aliasing inside existing retries

`stream_loop.rs` already uses a pattern where truncation/network retries
create a LOCAL `retry_token_state` and hand it to a new `CompactionHook`,
leaving the outer `token_state` untouched. That means the outer
`classify_compaction_branch(&e, &token_state)` call would observe stale
state if the retry hook triggers compaction.

For CMPCT-027's compaction restart, we keep the outer `token_state` Arc alive
and RESET its fields (`compaction_needed = false`, `input_tokens = session.token_tracker.input_tokens`, zero caches, zero output) then build a
new hook from the SAME Arc. This avoids the aliasing risk specifically for
the compaction-restart path and keeps the branch classifier correct for
subsequent errors. The pre-existing network/truncation retry aliasing is
out of scope for CMPCT-027 (filed as follow-up).
