# BUG-258: Outer agent failures bypass durable terminal journaling

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P2 — cold transcript replay loses some failed request outcomes.
- Evidence: static inspection of stream error returns and outer fallback publication; no new failure-injection test was executed during review.

## Problem and impact

The new durable terminal journal is driven by `BackgroundOutput::emit`. Some
stream failures propagate directly to the outer agent loop without emitting a
`StreamEvent::Error`. When recovery fails or its budget is exhausted, the outer
fallback publishes `Error` and `Done` directly through
`BackgroundSession::handle_output`, bypassing durable journal admission.

The current live session can display that failure, but a fresh process cannot
recover its terminal outcome. This violates the new separation contract: safe
failure metadata belongs in the durable transcript, not model originals.

## Source evidence

Locations refer to the reviewed commit:

- `rust/agent-loop/src/background_output.rs:403` admits stream events into the durable journal before publication.
- `rust/agent-loop/src/stream_journal.rs:162` maps emitted errors to the safe provider-error terminal outcome.
- `rust/cli/src/interactive/stream_loop.rs:277` can propagate request-preparer failures before emitting an error event.
- `rust/cli/src/interactive/stream_loop.rs:1822` returns context overflow to an outer recovery owner without publishing a terminal event there.
- `rust/agent-loop/src/agent_loop.rs:1474` publishes the final fallback error directly to the session.
- `rust/agent-loop/src/agent_loop.rs:1478` likewise publishes fallback completion without journal admission.
- `rust/agent-loop/src/agent_loop.rs:1481` has another direct-publication path for exhausted automatic recovery after compaction.

Ordinary exhausted network failures that already emit an error are not the
missing case; the fix must not produce duplicate terminal records for them.

## Reproduction to automate

1. Drive a real background turn whose request preparer rejects invalid limits before a stream error event is emitted.
2. Allow the outer error fallback to publish its live failure.
3. Read the durable journal and restart with a fresh session manager.
4. Compare cold transcript replay with the live terminal outcome.

Also exercise a context overflow delegated to the outer owner where recovery
ultimately fails or exhausts its budget.

Expected: a terminal failure that is published after recovery is abandoned has
one durable, safe outcome and appears once in cold transcript replay.

## Suggested repair direction

Route final fallback termination through the same request-owned journal and
deduplication rules as ordinary stream termination. Keep intermediate recoverable
failures distinct from the final abandoned outcome. Preserve status updates and
completion notifications, avoid leaking raw provider errors into durable safe
metadata, and never put terminal prose into model history.

## Acceptance and validation guidance

- Invalid request preparation has one recoverable terminal failure.
- Failed or exhausted outer overflow recovery retains the original request outcome.
- Errors already journaled by the inner stream are not duplicated.
- Successful automatic recovery does not acquire a spurious final failure.
- Repeated cold restores preserve outcome identity and ordering without model-history pollution.

Related specifications: `spec/features/durable-transcript-outcomes.feature`,
`spec/features/cold-transcript-terminal-replay.feature`, and
`spec/features/cold-empty-failure-display.feature`.
