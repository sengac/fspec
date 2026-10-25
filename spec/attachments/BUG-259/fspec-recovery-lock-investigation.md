# BUG-259: Recovery-lock investigation — 2026-10-25

## Finding

The screenshot is an application-generated interruption following transient
recovery-file-lock contention, not a Codex API failure. macOS process spawning
can reproduce the contention without another application thread acquiring that
session's lock. The recovery lease relies on file close rather than explicit
unlock; a spawned child's temporarily inherited file reference can prolong the
lock beyond the parent's close. A nonblocking acquisition then fails instantly.

## Incident evidence

- Log: `/Users/rquast/.fspec/logs/private/fspec-combined.log.2026-10-25:1149`.
- At `2026-10-25T08:19:48.180463Z` (18:19:48 local), session
  `308a34d0-20a1-4209-b979-9e6d4331faa4` reports exactly:
  `recovery owner unavailable: lock acquisition failed because the operation would block`.
- Its last text record, sequence 2570, was created at `08:19:48.153067Z`.
  It contains the backtick following `Initial attachment is `, matching the
  screenshot's truncated response. Sequence 2571 is a terminal `interrupted`
  event at `08:19:48.257280Z`.
- Parent session `9342962b-5a6e-45fb-90ba-38b6bcbcecb8` requested a
  `shell_command` at `08:19:48.000258Z`, and its tool-start event is timestamped
  `08:19:48.079274Z`. This is consistent with the reproduced spawning race;
  the historical lock-holding PID itself was not logged and cannot be proven.
- The recovery directory is now empty and `lsof` showed no current holder of
  this session's recovery lock. The final text delta exists in the transcript,
  consistent with successful subsequent recovery rather than a permanent lock.
- The running process started at 17:59:07. Its executable is
  `/Users/rquast/.local/bin/fspec`, byte-identical to the local release-slim
  binary (SHA-256 `72557bb4d1629532175dc0b8ec0f1f0804e83cb6474f9208f49e0b44971d4197`).

## Code path and why the last commit missed it

- `rust/core/src/persistence/session_lifecycle.rs:36` calls `File::try_lock`
  once and wraps the failure in `io::Error::other`. `RecoveryLease` has no
  explicit-unlock Drop implementation; release relies on its `File` field.
- `rust/agent-loop/src/persistence_retry_ownership.rs:26` converts any such
  failure directly to the error visible in the screenshot.
- `rust/agent-loop/src/persistence_retry.rs:227` attempts retention after
  enqueue failure, but retention acquires the same recovery lease. It can fail
  before reaching the three retry calls; those calls have no backoff anyway.
- `rust/agent-loop/src/background_output.rs:350` publishes this as an API-style
  error and sets `is_interrupted`, terminating an otherwise working stream.
- `f0eb0759` changed recovery ownership from a global mutex to per-session
  mutexes and cached journal writers. It did not change this nonblocking
  lock acquisition or lease release. Blame attributes the acquisition to
  `2699693b`.
- `rust/core/tests/bug255_append_scaling.rs:69` tests a held transcript
  `writer.lock` plus independent sessions, not process-spawn inheritance of a
  recovery lease. Its coverage cannot detect this race.

## Reproduction and controls

The standalone Rust test harness is attached as
[`fspec-recovery-lock-investigation.rs`](fspec-recovery-lock-investigation.rs).
It links the existing compiled production core and agent-loop libraries rather
than replacing their locking implementation. Every filesystem mutation is in a
temporary directory created with `codelet_test_helpers::setup_temp_directory`.

Run: `/tmp/fspec-recovery-lock-investigation --test-threads=1`.

Two complete four-test runs both returned 2 passed, 2 failed:

1. A lease held for 40 ms causes the actual `PersistenceSpool::open` to fail
   after only 58.25 microseconds / 55.333 microseconds with the exact screenshot
   error. Reopening after release succeeds.
2. One thread repeatedly acquires and drops the actual `RecoveryLease`; another
   launches 500 `/bin/sh -c true` children using `process_group(0)`, like the
   shell tool. No other thread acquires the recovery lock. This produces
   39 failures in 50,574 acquisitions, then 29 failures in 51,911 acquisitions.
3. Control: 50,000 actual recovery-lease acquisitions without process spawning
   complete without any failure.
4. Control: the equivalent standard-file locking loop with explicit `unlock()`
   before close has zero contention failures during 500 child spawns. This is
   a mechanism control, not an implemented or fully validated production fix.

Evidence is attached as
[`fspec-recovery-lock-investigation-tests.log`](fspec-recovery-lock-investigation-tests.log)
and
[`fspec-recovery-lock-investigation-repeat.log`](fspec-recovery-lock-investigation-repeat.log).

Build against the existing artifacts:

```sh
rustc --edition=2021 --test -C lto=thin -C opt-level=1 \
  spec/attachments/BUG-259/fspec-recovery-lock-investigation.rs \
  -L dependency=rust/target/release-slim/deps \
  --extern codelet_agent_loop=rust/target/release-slim/deps/libcodelet_agent_loop-cc99ff2121b7c389.rlib \
  --extern codelet_core=rust/target/release-slim/deps/libcodelet_core-70577d03ebc1c23b.rlib \
  --extern codelet_common=rust/target/release-slim/deps/libcodelet_common-136537460226b866.rlib \
  --extern codelet_test_helpers=rust/target/release-slim/deps/libcodelet_test_helpers-7a259000ffd830d9.rlib \
  -o /tmp/fspec-recovery-lock-investigation
```

## Recommended repair scope

First add acceptance scenarios and regression tests for spawn overlap and
short-lived real contention. Explicitly release the recovery lock before
closing the parent's file, and handle temporary contention with bounded retry
while preserving session-deletion lock order, authentic pending records, and
independent-session progress. Preserve typed contention errors rather than
flattening them into generic strings/Other errors. Do not replace this with an
unbounded blocking wait without checking lock ordering and cancellation.

The historical holder is inferred, not logged. The failure mechanism and
incorrect transient-contention handling are both independently reproduced.
The original investigation changed no repository or production-session files;
the follow-up creates BUG-259 and preserves these research attachments only.
No production fix or full test-suite validation is claimed.
Do not delete an active lock file as a
workaround: replacing its inode can break mutual exclusion.
