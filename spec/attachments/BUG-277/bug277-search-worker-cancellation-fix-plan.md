# BUG-277: Stop abandoned search workers

## Outcome

Cancellation must reach the actual SessionSearch worker. A dropped tool future
must not leave an unbounded history scan consuming CPU or monopolizing storage.
Keep blocking I/O outside Tokio async workers and preserve uncancelled results.

## Evidence

`rust/tools/src/session_search/mod.rs:184` uses `spawn_blocking`. Dropping the
awaiting future does not abort a running blocking closure. There is no worker
cancellation token/check in the current full-history scan.

In the isolated 2026-10-27 run, the session became Idle at 09:47:34 UTC.
An eight-second sample starting 09:47:40 still showed:

```text
SessionSearchTool::call blocking closure         5009 samples
  session_search_handler::handle_search
    manifest::get_session_messages_full         4952 samples
      MessageStore::get -> get_recovered_locked -> get_locked
```

The worker therefore continued after cancellation and Idle. In the subsequent
warm cancellation sample, search also waited on the shared message-store mutex;
cancellation persistence competed for the same store.

`manifest_messages.rs:97` holds the global message-store mutex while retrieving
the entire session history. This predates the October changes. `e486b5bf` fixed
async-thread occupancy, not worker lifetime or lock granularity. Do not reverse
the offloading fix or claim this profile proves a new 1000x search algorithm cost.

Raw local evidence: `/tmp/fspec-esc-evidence/search-interrupt.sample.txt`,
`exact-interrupt.sample.txt`, `interrupt-search.log` and `daemon.log`.

## Acceptance criteria to specify

1. Cancel an active search after a bounded read batch: its worker stops reading
   additional history and releases its leases; unrelated storage work progresses.
2. Drop the tool future while its closure is already running: the worker still
   observes cancellation. Aborting a JoinHandle alone is not sufficient.
3. Cancel before start, between sessions, within a large session and around blob
   resolution; do not publish a stale successful result into a newer turn.
4. A cancellation waiting for storage ownership is noticed within a bounded
   retry/check interval, rather than waiting behind an entire large-session scan.
5. Normal search/show/recent/originals behavior, filtering, trimming, result limits,
   identity/corruption checks, hook denial and per-session routing remain intact.

## ACDD implementation game plan

1. Extend acceptance criteria alongside `responsive-session-search.feature`.
   Establish an invocation-scoped cancellation owner independent of handler-map
   cleanup. Map one test to each scenario and validate the specifications.
2. Write RED integration tests with real temporary stores and shared helpers.
   Use deterministic read barriers/counters to prove that work stops and leases
   are released, not merely that the outer future returns. Include a competing
   writer and a single-async-worker heartbeat regression.
3. Pass a cancellation signal into the blocking closure and underlying scan.
   A drop guard must cancel abandoned invocations. Check before expensive work,
   between bounded read batches and before serialization/publication.
4. Reduce whole-session global-lock occupancy with bounded reads while preserving
   a consistent manifest/reference boundary and peer-generation validation. Use
   cancellation-aware bounded lease acquisition where necessary; keep existing
   corruption and missing-original handling. Do not load the entire history and
   then check cancellation only at the end.
5. Return an explicit cancellation outcome through existing tool settlement;
   do not mislabel a partial scan as a successful empty search. Coordinate with
   BUG-275 so only one truthful cancellation record is persisted.
6. Repeat the isolated profile. After cancellation and settlement, no abandoned
   search frames should remain; verify another session can search/write promptly.

## Validation targets

Add a dedicated worker-lifetime integration target, then run:

```sh
cargo test -p codelet-tools --test bug273_search_responsiveness
cargo test -p codelet-core --test bug273_shared_index
cargo test -p codelet-core --test shared_message_store_index_coherence
```

Run the affected agent-loop search-handler tests and new bounded-read tests by
package and target. Run scoped check/clippy, formatting, feature/tag validation
and coverage checks. Use synthetic stores for regression tests and only an
isolated copied-store daemon for profiling, never the live user session.
