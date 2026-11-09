# BUG-329 root cause: response attempt rollback tests fail on an unpersisted session

## Reproduction

`cargo test --profile ci-test -p codelet-agent-loop --test response_attempt_rollback`

```
test aborted_output_does_not_poison_loop_detection ... FAILED
panicked at agent-loop/tests/response_attempt_rollback.rs:144:5:
assertion failed: !background.is_interrupted.load(Ordering::Acquire)
```

The previous run also reported `background_persistence_excludes_abandoned_text_and_reasoning`
failing at its live-chunk assertion (`response_attempt_rollback.rs:90-92`); in this
run it passed, so that failure is order-dependent.

## Evidence: the product is right, the test is stale

1. `aborted_output_does_not_poison_loop_detection` builds its `BackgroundSession`
   with `uuid::Uuid::new_v4()` (`response_attempt_rollback.rs:139`) and never
   creates a session manifest.
2. Every `BackgroundOutput::emit` first admits the event to the transcript journal
   (`rust/agent-loop/src/background_output.rs:430-444`). The journal's first append
   loads the session manifest to anchor the request
   (`rust/agent-loop/src/stream_journal.rs:69-77`,
   `codelet_core::persistence::load_session`). For a session that does not exist the
   load fails, so admission returns `Err`.
3. On an admission error `emit` calls `report_persistence_failure`
   (`background_output.rs:357-368`), which publishes an error chunk and stores
   `is_interrupted = true`. The test's assertion at line 144 is meant to prove the
   loop detector did not fire, but the interrupt it observes comes from the
   persistence failure: the first `Text` delta is never reached by the detector.
   Refusing to publish output that cannot be journaled is the intended
   (no-fallback) behaviour of the transcript journal, so the product must not change.
4. The test is also not `#[serial_test::serial]`, while it calls
   `codelet_common::set_data_directory` (`response_attempt_rollback.rs:137`), a
   process-global `Mutex<Option<PathBuf>>` (`rust/common/src/data_dir.rs:27-31`).
   `serial_test::serial` only excludes other `serial` tests, so it runs concurrently
   with the serial persistence test and can swap the data directory between that
   test's `create_session_with_provider` and its first journal append. The persistence
   test then hits the same admission failure, publishes no deltas and fails its
   live-chunk assertion (lines 90-92) — the "fails before replay is called" symptom.
5. The persistence test carries `#[serial_test::serial]` twice (lines 53-54).

## Fix

- The loop-detection test creates a real persisted session through
  `create_session_with_provider` in a fresh data directory (as the persistence test
  does), resets the stores, and is `#[serial_test::serial]`, so it exercises the
  loop detector on output that is actually admitted.
- Both persistence-touching tests share one setup helper; the duplicated `serial`
  attribute is removed.
- The scenario's Given step names the persisted session so the precondition is
  part of the acceptance criteria.
- The three agent-loop scenarios (persistence, loop detection, boundary
  serialization) move from `background-response-attempt-rollback.feature` into
  their own `background-response-attempt-persistence.feature`, so each feature maps
  to one test file (the rollback feature keeps its TUI scenarios and test files).

## Alternatives rejected

- Letting the journal admit events for a session with no manifest (skip the request
  anchor): a silent fallback that would publish output with no durable transcript.
- Asserting only on the loop detector's internal state: hides that the test was
  passing through a persistence failure path; the observable `is_interrupted`
  contract is the right thing to assert once the session is real.
- Marking only the loop test `serial` without creating a session: the test would
  stop racing but still fail, since its output is never admitted.
