# BUG-255: Streaming journal ingestion rescans all prior history

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P1 — increasingly long stalls in streaming and shared persistence.
- Evidence: static complexity and locking analysis; no timing benchmark was executed during review.

## Problem and impact

Each transcript append enumerates and sorts every prior journal file, reads and
deserializes every record, and rebuilds the lifecycle projection. Background
output performs this synchronously for individual text and thinking deltas.
Completed turns remain in the session journal, so the cost grows with the entire
session, not just the active response.

Processing N records therefore performs at least quadratic cumulative record
reads, in addition to directory scans and sorting. At 10,000 deltas, the append
path reads about 50 million prior records. This is a derived operation count,
not a measured latency result. Full-history reads also repeat large old tool
results that have no relationship to the newly admitted delta.

The append occurs while the process-global recovery-owner mutex is held, so one
large session can delay persistence and admission in otherwise unrelated sessions.

## Source evidence

Locations refer to the reviewed commit:

- `rust/core/src/persistence/transcript/store.rs:139` enumerates all journal paths on every append.
- `rust/core/src/persistence/transcript/store.rs:140` reads and decodes their contents.
- `rust/core/src/persistence/transcript/store.rs:174` rebuilds the full projection for admission.
- `rust/agent-loop/src/background_output.rs:403` routes streaming events through journal admission synchronously.
- `rust/agent-loop/src/stream_journal.rs:82` submits each journal record through persistence recovery.
- `rust/agent-loop/src/persistence_retry.rs:181` acquires the shared owner mutex for retry and keeps it across archive commit.
- `rust/core/tests/bug249_transcript_journal.rs:143` checks linear stored bytes for 128 deltas, but does not check reads, CPU work, or cross-session blocking.

## Reproduction to automate

1. Append fixed-size text deltas to a real temporary journal in growing batches.
2. Measure or instrument filesystem reads and admission work for each batch.
3. Repeat after a completed turn and after adding a large genuine tool result.
4. Submit an event to another session while the large session admits a delta.

Expected: steady-state admission does not reread and reproject the complete
session, and unrelated sessions do not wait behind another session's history scan.

## Suggested repair direction

Maintain validated incremental admission state, sequence coordinates, and
identity indexes with a safe cold-open reconstruction path. Preserve detection
of corrupt or incomplete records, idempotent retries, writer ownership, and
durable-before-publication guarantees. Review global lock scope independently
of the storage layout; moving the same scan to a blocking task does not remove
its growing cost or shared critical section.

## Acceptance and validation guidance

- Test scaling beyond the current 128-record byte-size assertion.
- Prefer deterministic read/work counters over fragile wall-clock assertions.
- Include accumulated completed turns and large old result payloads.
- Verify concurrent sessions retain independent admission progress.
- Retain integrity, promotion, process-crash, and session-deletion regressions.

Related specification: `spec/features/incremental-transcript-journal.feature`.
