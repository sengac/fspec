# BUG-272: API-error cleanup destroys Codex replay continuity

## Summary

A terminal provider error can delete a **previously executed tool's genuine
result**, even though the Codex checkpoint still contains its committed call.
The next user message then fails local replay validation before reaching the
server. This is a history/checkpoint consistency bug, not demonstrated remote
prompt-cache corruption.

The strongest fix direction is to preserve accepted calls and real outcomes on
ordinary provider failures, while retaining explicit, validated recovery for
actual context overflow and genuinely unresolved executions.

## Evidence and confidence

- Investigated on 2026-10-27, repository HEAD `b9b47002`, with existing local
  changes present. No production code or existing feature files were changed.
- Evidence: user report and direct inspection of current source,
  specifications, and tests. No live API reproduction or test execution was
  performed. The reported session's durable state was not inspected.
- High confidence in the code-level failure mechanism below. The exact affected
  tool and its persisted result cannot be established from the report alone.

## Observed incident

The report describes this sequence in the BUG-271 agent:

1. A Codex rejection with `code="server_is_overloaded"`,
   `type="service_unavailable_error"`, and message "Our servers are currently
   overloaded. Please try again later."
2. The client prints:
   `[Removed the failed tool call from context; send your next message to continue]`.
3. The user sends `...`.
4. The next attempt reports:
   `API Error: RequestError: Codex replay has no recorded outcome for call call_f4188cdf891546a4a81c9ad7c65a01c2`.

The earlier reported fspec workflow/coverage error is a tool
result, not the provider overload. A failed inference request does not establish
that the preceding tool execution failed or that its result should be discarded.

## Root-cause trace

### 1. Generic terminal recovery deletes messages based on shape

`rust/cli/src/interactive/stream_loop.rs:2333` documents BUG-170 / BUG-185 recovery.
At `:2347`, every non-interrupted terminal API error reaching this branch invokes
`strip_failed_tool_call_tail(&mut session.messages)`. It is not restricted to
context-length errors and does not establish that the last tool failed.

`rust/cli/src/interactive/recovery_unrecoverable.rs:55` implements the surgery:

- For a trailing tool-result-only user message, remove the entire message and
  remove corresponding calls from the nearest preceding assistant message.
- For a tool-call-only assistant tail, remove that assistant message.
- Preserve non-tool assistant content when filtering a mixed assistant message.

At `stream_loop.rs:2352`, a removed tool pair/call causes cache-token accounting
to be cleared, token estimates to be recalculated, and the error annotation to
be emitted. The loop then breaks instead of returning an error. There is no
Codex checkpoint transition or genuine-outcome settlement in that branch.

The module's opening documentation still describes prompt-too-long-only
invocation; BUG-185 broadened the actual caller to all terminal API errors.

### 2. Codex commits a tool call before local execution

`rust/providers/src/codex/streaming.rs:215` rejects failed terminal responses.
At `:221`, successful response input/output is committed, and only then are tool
calls yielded at `:222` for execution. This ordering is intentional.

`rust/providers/src/codex/history.rs:221` constructs a checkpoint from prepared
request input plus successful response output. Preparing the next request does
not itself commit that request's newly supplied results (`history.rs:197`).

Therefore a valid intermediate state is:

```text
Checkpoint:     original user message -> committed tool call C
Live messages:  original user message -> tool call C -> genuine result O
Next inference request, carrying O: rejected by server overload
```

The rejected response does not advance the checkpoint. Generic cleanup then
deletes C and O from live messages without removing C from committed ancestry.

### 3. Replay restores the call but cannot restore the deleted result

`rust/providers/src/codex/history_lineage.rs:63` reconciles incoming history with
committed projections. An absent projected tool call does not itself cause a
lineage failure (`:83`); replay retains all saved non-reminder items (`:99`).

The result is:

```text
Live messages after cleanup: original user message -> next user message
Assembled Codex replay:      original user message -> committed C -> next user message
Missing:                    genuine result O for C
```

`rust/providers/src/codex/history_replay.rs:18` requires a recorded output for
every function/custom tool call. It returns `CheckpointError::MissingOutcome`
at `:20`. The exact reported text is defined in
`rust/providers/src/codex/checkpoint_error.rs:13`.

This occurs during request preparation
(`rust/providers/src/codex/wire_request.rs:56`), before transport sends the next
request. Removing both visible halves can satisfy a generic no-orphan check
while still violating provider-owned checkpoint continuity.

If the deleted result was already in the committed checkpoint, the same cleanup
can instead produce an earlier `assistant, tool result or DAG ancestry` lineage
failure (`history_lineage.rs:83`). The reported missing-outcome diagnostic
fits the intermediate checkpoint state above.

## Why changing the cache key will not fix this

There are three separate states:

| State | Relevant implementation | Meaning |
| --- | --- | --- |
| Cache-token accounting | `rust/cli/src/interactive/stream_loop.rs:2353` | Usage/display fields cleared by cleanup; not tool history. |
| Codex affinity | `rust/providers/src/codex/routing.rs:135` | UUID-based `prompt_cache_key`, also sent as `session_id` at `:141`; not an outcome store. |
| Committed replay history | `rust/providers/src/codex/history_replay.rs:18` | Local call/outcome integrity; source of the second failure. |

Rotating affinity or zeroing token counters cannot supply the deleted genuine
result. A shorter prefix could affect remote cache reuse, but that is not the
observed replay error and was not measured here.

## Existing contracts and coverage gap

- **BUG-170:** `spec/features/strip-failed-tool-call-tail-on-unrecoverable-api-error.feature`.
  Originally scoped surgery to unrecoverable prompt-too-long errors.
- **BUG-185:** `spec/features/strip-failed-tool-call-tail-on-any-terminal-api-error.feature:8`.
  Explicitly requires deletion for all terminal API errors. This requirement
  conflicts with preserving Codex committed ancestry and must be reconciled,
  not merely worked around in the provider.
- `rust/cli/tests/bug185_strip_failed_tool_call_tail_on_any_terminal_error.rs:1`
  documents primitive tests plus source-shape assertions. They do not prove
  next-turn continuation through a real Codex checkpoint/transport.
- **CMPCT-080:** `spec/features/codex-checkpoint-replay-integrity.feature:49`
  requires explicit missing-outcome failure, unchanged checkpoint, and no
  fabricated outputs. Its coverage file links
  `rust/providers/tests/cmpct080_checkpoint_replay.rs`.
- `rust/providers/tests/codex_replay_regressions.rs:195` verifies missing-outcome
  refusal and recovery with a recorded outcome, live and restored. It does not
  exercise terminal CLI surgery after an executed tool.
- `rust/providers/tests/codex_recovery_boundaries.rs:149` verifies checkpoint
  commit before tool delivery; it does not test subsequent terminal cleanup.
- `rust/cli/tests/codex_accepted_completion_batch.rs:71` demonstrates real CLI
  consumption and next-turn replay with preserved results and cancellation of
  an unexecuted remainder. Reuse its composition approach, not source matching.
- `rust/providers/src/codex/settlement.rs:39` provides genuine-result settlement
  with identity/conflict validation. Its pending handoff records explicitly say
  outcome unknown; they are not evidence of success or cancellation. The
  terminal cleanup branch does not call this API.

## Recommended fix boundaries

1. Preserve accepted calls and all genuine results for overload, auth, exhausted
   transport retries, and other non-context terminal provider failures. Surface
   the failure while allowing a safe later user-driven continuation.
2. Distinguish inference failure from tool execution failure. Preserve actual
   tool-error results too; do not re-run completed side effects automatically.
3. Keep Codex checkpoint and conversation history compatible. If an exceptional
   recovery must change ancestry, use a validated recorded generation transition
   with durable consistency, not message-vector surgery alone.
4. Keep the missing-outcome guard. Never fabricate success/cancellation, silently
   reset a session, weaken lineage validation, or rotate affinity as a substitute
   for retaining the genuine outcome.
5. Reconcile BUG-185 feature/tests explicitly. Keep context-overflow recovery,
   interruption, accepted-completion, and other providers' safety contracts.
6. Replace the misleading "failed tool call removed" explanation when the actual
   failure was server-side inference. Review other history-trimming callers for
   the same checkpoint mismatch, without expanding this card into unrelated work.

## Proposed regression scenarios

Write acceptance criteria first, then failing integration tests, then code.
Use real session/temp-store helpers and scripted provider transport boundaries.

1. **Overload after a completed tool:** commit C, execute a filesystem-changing
   tool once, retain O, reject the follow-up with the reported overload
   payload, run the actual terminal error arm, then send a new user message.
   Assert unchanged genuine C/O identities, exactly one replay output for C,
   one tool execution, and successful next-turn request construction.
2. **Cold resume:** persist after the same failure, reload, and prove preserved
   C/O continuity and no duplicate side effects.
3. **Other terminal failures:** cover auth rejection and exhausted network
   recovery without dropping completed tool results or confusing usage counters
   with history correctness.
4. **Parallel/mixed-content batch:** preserve every completed result and original
   call identity, assistant text/reasoning, and honest unresolved-execution state.
   Never invent an outcome for a call whose execution status is unknown.
5. **Genuinely missing outcome:** still fail before send with typed missing-outcome
   error and unchanged checkpoint; preserve CMPCT-080 protection.
6. **Actual context overflow:** retain existing compaction/recovery precedence
   and ensure any eventual history transition remains checkpoint-consistent.

Suggested existing control targets, run from `rust/` (not run during research):

```sh
cargo test -p codelet-providers --test codex_replay_regressions
cargo test -p codelet-providers --test cmpct080_checkpoint_replay
cargo test -p codelet-providers --test codex_recovery_boundaries
cargo test -p codelet-cli --test codex_accepted_completion_batch
cargo test -p codelet-cli --test bug170_strip_failed_tool_call_tail
cargo test -p codelet-cli --test bug185_strip_failed_tool_call_tail_on_any_terminal_error
```

## Open verification items

- Reproduce the exact combined CLI/provider path with a scripted overload;
  establish when genuine tool outcomes become durable in this path.
- Verify cold-resume behavior rather than assuming it matches in-memory replay.
- Decide the exceptional oversized-result recovery policy before replacing the
  BUG-185 contract; ordinary overload must not silently delete accepted history.
- The report includes retry hints (`retry-after: 6`, `retry-after-ms: 6000`)
  while the diagnostic reports `retry_after=None`. Retry-hint parsing is a
  separate possible issue, not necessary to explain this replay failure.
