# BUG-275: Cancellation acknowledgement and persistence fix

## Outcome

Implement a fix, not another research-only pass. Escape must acknowledge the
request promptly without permitting a new inference turn to overtake unfinished
durable settlement. Preserve genuine outcomes and crash-safe cancellation records.

## Evidence

Isolated debug daemon, current source `e486b5bf`, 2026-10-27:

```text
ToolCall observed:           5.357469 s
interrupt RPC returned:      5.866707 s (RPC duration 0.005207 s)
Idle observed:               9.662631 s
Acknowledgement-to-Idle:     3.795924 s (about 200 ms polling granularity)
```

An eight-second sample contains 2,333 cancellation-path samples, including:

```text
record_tool_cancellations                         2333
  emit_tool_cancellation
    persist_tool_result_identified               1167
      PersistenceSpool::retry -> archive::commit
        append_message_idempotent -> store_with_id -> append_message
          save_index -> NamedTempFile::write_all -> write
    persist_assistant_message                    1159
      PersistenceSpool::retry -> archive::commit
        append_message_idempotent -> store_with_id -> append_message
          save_index -> NamedTempFile::write_all -> write
```

Some samples wait on the global message-store mutex. The fixture also contained
previously cancelled search work, so these are not contention-free measurements.
Debug timings are not a release SLA. Raw local evidence:
`/tmp/fspec-esc-evidence/exact-interrupt.sample.txt` and `warm-cancel.log`.
The attached investigation report is self-contained if those temporary files expire.

`2699693b` added `record_tool_cancellations` before `emit_interrupted` in
`rust/cli/src/interactive/stream_loop.rs:784`. Index rewriting already existed
in `38a95522`; the newly inserted cancellation work is the regression.

## Acceptance criteria to specify

1. While cancellation persistence is deliberately paused, the user sees that
   interruption was accepted; unrelated async work remains responsive.
2. No next turn or successful completion is admitted before required settlement.
3. A running tool receives one truthful cancelled/unknown-outcome record; a tool
   that already completed retains its genuine outcome, including genuine errors.
4. Crash/reopen and persistence failures preserve recoverability, original identities,
   ordering and exactly-once publication without fabricating success.
5. Publishing an index uses bounded buffered writes rather than three OS writes
   per entry, while maintaining the existing index format and integrity guarantees.

## ACDD implementation game plan

1. Review interruption/outcome features and existing tests listed below. Specify
   acknowledgement versus settlement explicitly; do not redefine Idle as "still
   persisting". Register any new tags, format and validate the Gherkin scenarios.
2. Write one integration test per scenario using shared temp-directory helpers,
   real message/journal stores and controlled persistence barriers. Assert event
   ordering and dispatch exclusion rather than fragile millisecond deadlines.
   Record RED before touching production code.
3. Separate visible cancellation acknowledgement from settlement ownership. Use
   existing lifecycle events if adequate; otherwise document the minimal UI/RPC
   contract change. Keep durable settlement ordered and off async worker threads.
4. Buffer `messages/index/write.rs` output, explicitly flush before syncing and
   atomically promoting the file. Preserve fault-injection boundaries and directory
   synchronization. Evaluate a single publication for related appends only if
   needed after measurement; request approval before a major storage refactor.
5. Run focused regressions, then repeat the isolated warm profile with the same
   fixture and query. Compare acknowledgement, settlement, lock wait, index writes
   and CPU; do not claim faster safe settlement solely from an earlier UI event.

## Validation targets

From `rust/`, use package-and-target scoped commands:

```sh
cargo test -p codelet-agent-loop --test bug249_interrupted_tool_state
cargo test -p codelet-cli --test codex_interruption_outcomes
cargo test -p codelet-core --test concurrent_message_index_persistence
cargo test -p codelet-core --test bug273_shared_index
```

Add a dedicated `bug275_cancellation_settlement` target in the relevant crate(s).
Run scoped check/clippy, formatting, feature/tag validation and coverage checks
before closure. Never run unscoped workspace tests or use the live store.

## Coordination

BUG-277 stops abandoned search work; do not make this fix depend on every search
being fast. BUG-276 fixes cancellation ownership during startup. BUG-272 preserves
Codex completed outcomes. Coordinate shared stream-loop edits; retain separate
scenario/test ownership. Suggested order: BUG-272, BUG-276, BUG-277, then final
cross-card performance validation for BUG-275. No hard dependency is required.
