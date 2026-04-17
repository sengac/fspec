# CMPCT-024 — AST Research: State-flush Sites Before Compaction

## Purpose

Verify (a) the number of sites that currently save `assistant_text` before
terminating/breaking, and (b) the token-tracker flush pattern we need to
mirror.

## Save-partial-text call sites

Pattern: `handle_final_response($$$ARGS)`

```
codelet/cli/src/interactive/gemini_continuation.rs:92    handle_final_response(assistant_text, ...)
codelet/cli/src/interactive/gemini_continuation.rs:200   handle_final_response(text, ...)
codelet/cli/src/interactive/gemini_continuation.rs:270   handle_final_response(text, ...)
codelet/cli/src/interactive/gemini_continuation.rs:316   handle_final_response(text, ...)
codelet/cli/src/interactive/gemini_continuation.rs:335   handle_final_response(text, ...)   ← the BUG-free reference path
codelet/cli/src/interactive/gemini_continuation.rs:354   handle_final_response(text, ...)
codelet/cli/src/interactive/stream_loop.rs:572           handle_final_response(&assistant_text, ...)
codelet/cli/src/interactive/stream_loop.rs:672           handle_final_response(&assistant_text, ...)
codelet/cli/src/interactive/stream_loop.rs:965           handle_final_response(&assistant_text, ...)   ← normal MultiTurnStreamItem::FinalResponse
codelet/cli/src/interactive/stream_loop.rs:1246          handle_final_response(&assistant_text, ...)   ← truncation recovery reference
codelet/cli/src/interactive/stream_loop.rs:1348          handle_final_response(&assistant_text, ...)   ← network retry reference
codelet/cli/src/interactive/stream_loop.rs:1467          handle_final_response(&assistant_text, ...)
codelet/cli/src/interactive/compaction_retry.rs:209/319/363   handle_final_response(&retry_text, ...)
```

Currently **ZERO** calls between line 1131 (error arm) and line 1161 (break for
compaction cancel). That's the missing site BUG 2 identifies.

## Token-tracker flush pattern

The canonical flush uses `session.token_tracker.update_from_usage(&ApiTokenUsage, cumulative_output)`:

- `codelet/cli/src/interactive/stream_loop.rs:1544` — post-loop final flush
- `codelet/cli/src/interactive/gemini_continuation.rs:378-387` — `update_token_tracker(session, display)` helper
- `codelet/cli/src/interactive/gemini_continuation.rs:323` — pre-return flush in continuation path

For the BUG 6 fix we need the same shape as gemini_continuation.rs:378-387.

## Helper-function extraction target

Plan step 6 asks for a test-friendly helper. Good signature (matches both the
call pattern in stream_loop and gemini_continuation):

```rust
pub(super) fn flush_partial_state_before_compaction(
    session: &mut Session,
    assistant_text: &mut String,
    display: &StreamingTokenDisplay,
) -> Result<()>
```

This lets us unit-test the combined side effects (message append + tracker
update + buffer clear) against a real `Session` without needing a rig stream.

## Caller sites requiring modification

1. `codelet/cli/src/interactive/stream_loop.rs:1156-1161` — the target fix.

## Event-emission decision

`compaction_retry.rs:59-61` emits `compaction_started` and
`compaction_progress` unconditionally at the top of `handle_compaction_retry`.
That function runs immediately after our break returns (see
`stream_loop.rs:1512-1527`). Emitting the events here too would cause double
emission. The plan explicitly asked us to call this out — decision: emit ONLY
in `compaction_retry.rs`, add a `debug!` log in the break arm noting the
events will fire downstream.

(Contrast with gemini_continuation.rs:341-343, which emits the events itself
because its parent stream_loop path does NOT go through
`handle_compaction_retry`'s emit block — it instead `break`s out of the
inner match and re-enters post-loop compaction via the
`compaction_needed` flag, which DOES call `handle_compaction_retry`. Wait —
actually it does re-enter handle_compaction_retry. Re-checking…)

Tracing the flow one more time:
- `GeminiContinuationResult::CompactionNeeded` ⇒ `break` in stream_loop:958-960
- After `break`, stream_loop:1512 checks `compaction_needed` and calls
  `handle_compaction_retry`, which at line 59-61 emits the events.

So gemini_continuation.rs:341-343 IS in fact double-emitting today (a
pre-existing defect, not introduced by CMPCT-024). But patching that is out
of scope for this card — the plan only says "If yes, DO NOT emit here".
We'll match that guidance. A follow-up card can reconcile
gemini_continuation's emission.
