# BUG-270: Streaming durability diagnosis and focused repair

## Outcome

Healthy transcript admission now commits directly to the durable journal when
the session has no older recovery work. It skips staging, resyncing and deleting
a redundant recovery file. Journal record, directory and root durability barriers
still precede publication. Failures use the existing bounded recovery path.

The same real-filesystem fixtures measured approximately **21 to 98–102 deltas/s**
on this machine, about **4.7×** throughput. This is a local persistence result,
not a live-provider or full-UI benchmark. Synchronous journal I/O still blocks
the consuming task; this repair does not implement batching or async persistence.

## Original diagnosis

Commit `2699693b74124cf12b323997fd68dd491adf7e8f` added synchronous journal
admission before text/thinking publication. Stream consumption cannot continue
until that work finishes. This explains local display/consumption stalls, not
slower remote model generation.

The original commit also reread transcript history per append and held a
process-global recovery mutex across disk work. BUG-255 (`f0eb0759`) already
repaired those amplification mechanisms. They were not still broken at the
investigated HEAD, `56cf5cc019e174e24f152916aa4116c980322c4c`.

The remaining slowdown was independently reproducible without a network,
historical replay, loop detection or a large session: 256 small deltas took
12.05 seconds (21.2 deltas/s), median sink time 46.987 ms. Increasing payload
size barely changed the result.

## Environment and reproduction

- Date: 2026-10-26; macOS 26.6.2 (25G83), aarch64; rustc 1.95.0.
- Real stores and temporary filesystems composed from `codelet_test_helpers`;
  production `BackgroundOutput`, persistence spool and journal; no LLM endpoint.
- Alternating text/thinking streams, 256 events per payload size. Labels 16
  and 1024 describe repeated UTF-8 bytes, plus ten sequence/separator bytes.
- Direct control calls the same session's `handle_output` without admission.
  This is a fanout-only lower bound, not the complete pre-commit agent loop.
- Run serially to reduce filesystem contention. There are no portable timing
  thresholds; assertions cover durability operations, recovered identity,
  content/order and cooperative executor progress.

```sh
cd rust
cargo test --profile release-slim -p codelet-agent-loop \
  --test bug270_streaming_speed -j 2 -- --nocapture --test-threads=1
cargo test --profile release-slim -p codelet-agent-loop \
  --test bug270_admission_safety --test bug270_failure_ownership \
  -j 2 -- --test-threads=1
```

## Measured results

Original serial run (`bug270-speed-isolated.log`):

| Sink / payload | Deltas | Total ms | Deltas/s | p50 ms | p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Direct / 16 | 256 | 0.190 | 1,345,011.1 | 0.001 | 0.001 |
| Durable / 16 | 256 | 12,051.466 | 21.2 | 46.987 | 52.233 |
| Direct / 1024 | 256 | 0.431 | 593,909.6 | 0.001 | 0.003 |
| Durable / 1024 | 256 | 12,122.173 | 21.1 | 47.036 | 53.015 |

Repair run (`bug270-green.log`):

| Sink / payload | Deltas | Total ms | Deltas/s | p50 ms | p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Direct / 16 | 256 | 0.171 | 1,498,899.2 | 0.001 | 0.001 |
| Durable / 16 | 256 | 2,611.403 | 98.0 | 9.984 | 13.120 |
| Direct / 1024 | 256 | 0.362 | 707,020.2 | 0.001 | 0.002 |
| Durable / 1024 | 256 | 2,508.606 | 102.0 | 9.928 | 11.979 |

Both durable runs recover exactly 256 distinct identities in original order
per stream, observe 768 labeled journal boundaries and zero warm history reads.
The initial overlapping baseline (`bug270-speed.log`) measured 12.9 and 14.4
deltas/s, illustrating contention sensitivity. These measurements are not a
controlled two-checkout benchmark or a universal speedup promise.

## Work budget and call path

`BackgroundOutput.emit` calls `StreamJournal.admit` before publishing a chunk.
`StreamJournal.append` creates one immutable journal identity and recovery
envelope, then calls `persistence_retry::submit`. This is the actual production
path, not an unwired helper.

Before repair, `submit` always performed `enqueue` then `retry`. Source audit
counts ten `sync_all()` calls per healthy Unix text/thinking delta:

| Work | Calls |
| --- | ---: |
| Recovery staged file, recovery directory, parent | 3 |
| Retry resync of already-promoted recovery record | 1 |
| Transcript record plus two two-directory parent syncs | 5 |
| Recovery-file deletion acknowledgement | 1 |

After repair, healthy admission performs the same five transcript sync calls
without the five recovery-file sync calls. The observer exposes three logical
journal boundary labels, not every underlying syscall. Counts are from source
audit, not an OS syscall trace; setup, retries and failures add work.

Healthy admission also removes the separate enqueue/retry manifest checks.
It retains one recovery-lease manifest read and the journal ownership manifest
read. Sequence-zero request-anchor loading adds another read. Large-manifest
latency was not independently measured and remains outside this focused repair.

## Safety and ownership

- Direct admission requires a version-1 transcript within `MAX_PENDING_BYTES`,
  empty admitted/retained ownership, and an empty on-disk recovery lane.
- Session mutex and recovery lease span direct commit. Existing pending work
  follows the original ordered spool path; unreadable recovery fails closed.
- Direct failure retains the exact envelope in `owner.admitted` and attempts
  durable staging **before releasing the same lease**. Existing retry and
  caller-retention paths remain, including their capacity limits.
- Directory/root failures leave a provisional journal record. Tests compare
  that complete record, including ID, with the spool's original envelope,
  then repair and retry twice to prove idempotent recovery.
- A combined record-barrier/staging-failure fixture drops the output owner,
  reopens recovery ownership and verifies authentic exactly-once recovery.
  On macOS it uses an add-file-denying ACL: ordinary mode bits are insufficient
  because recovery intentionally restores private directory permissions.
- A warmed writer is deleted before its next delta; no publication or journal
  resurrection is allowed. Invocation admission and tool-start barriers still
  gate dispatch; existing executor/result-capacity tests retain side-effect safety.
- When storage cannot durably stage bytes, retained ownership is process-local
  and **shutdown-unsafe** until repaired. No unconditional cross-process ordering
  or process-crash survival is claimed for this total-storage-failure case.

## ACDD and validation evidence

The initial characterization tests passed before any repair and are not
presented as regression RED. Their original workflow-ordering mistake remains
recorded in BUG-270 architecture notes; no timestamps or gates were bypassed.

Repair criteria preceded repair tests. The initial read-only-lane test passed
because the baseline reset mode 0500 to 0700. The strengthened permission-
preservation scenario genuinely failed on baseline, then passed after direct
admission (`bug270-recovery-lane-red.log`, `bug270-safety-red.log`,
`bug270-green.log`). Three later safety scenarios extend validation, not a
claim of a second production repair.

Traceability covers three features and twelve scenarios:

- `streaming-persistence-speed-simulations.feature`: three diagnostic scenarios.
- `streaming-durable-admission-fast-path.feature`: six repair/safety scenarios.
- `streaming-admission-failure-ownership.feature`: three storage/deletion scenarios.

Raw logs retain initial compile/test experiments separately from genuine RED.
`bug270-green.log` includes 17 passing tests; `bug270-safety-broader.log` includes
23 passing parent tests plus cold-recovery subprocesses (one subprocess helper
is intentionally ignored in its parent run). Check/clippy logs are scoped to
the changed package/targets. No full-workspace test pass is claimed.

## Remaining performance limits

The real-lock single-thread diagnostic still shows ready-task starvation while
`emit` waits synchronously (216.912 ms in the repair run, 224.401 ms baseline).
This is preserved characterization, not an unresolved failure of the direct-
admission criteria. Further work may move blocking I/O off executor workers or
use explicitly bounded group commit, with separate acceptance criteria for
latency, ordering, durability, cancellation and tool-side-effect admission.
