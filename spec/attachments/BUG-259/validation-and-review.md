# BUG-259 validation and direct review

Baseline: `b2475dcd` (card-registration-only commit after `f0eb0759`).
All commands below use local fixtures; no production sessions or credentials.

## Causal red / green

From `rust/`:

```sh
cargo test --offline --profile ci-test -p codelet-core --test bug259_recovery_contention -j2
```

- **Before production changes: 1 passed, 3 failed.** The short-held lease made
  `PersistenceSpool::open` return `recovery owner unavailable: lock acquisition
  failed because the operation would block`. Persistent contention returned
  `Other`, not `WouldBlock`. Real overlapping shell spawning produced failed
  acquisitions. The invalid-path control passed.
- **After production changes: 4 passed, 0 failed.** Same test file and command;
  production change is limited to `RecoveryLease`. Subsequent test formatting
  only wraps imports/expressions, without changing assertions or behavior.
- Full outputs: `red-ci-test.log`, `green-ci-test.log` in this directory.
  These are actual captured logs, not reconstructed test runs.

## Affected regression matrix

Every command uses `--offline --profile ci-test -j2` and explicit package/targets.

| Package | Target | Result |
| --- | --- | --- |
| codelet-core | bug259_recovery_contention | 4 passed |
| codelet-core | bug255_append_scaling | 4 passed |
| codelet-core | cmpct064_delete_ownership | 1 passed |
| codelet-agent-loop | bug246_session_ownership | 2 passed |
| codelet-agent-loop | bug250_deleted_recovery | 1 passed |
| codelet-agent-loop | bug250_recovery_retirement | 7 passed, 1 ignored subprocess fixture |
| codelet-agent-loop | bug258_outer_terminal | 7 passed, 1 ignored subprocess fixture |

Total: **26 parent-suite tests passed**, two subprocess entry points ignored by
their normal parent runners and invoked explicitly by those runners. Child
process test counts are not double-counted. Actual outputs are retained in
`core-regressions.log` and `recovery-regressions.log`. The latter reports warnings
about unused existing fixture fields/methods; no lint suppression was added.

## Direct review

- Explicit unlock releases the parent's shared lock description before close;
  shell children cannot prolong the successfully released lock through inheritance.
- Only `TryLockError::WouldBlock` retries, using monotonic time and 5 ms sleep
  intervals capped by a 250 ms deadline. Persistent contention remains typed
  `io::ErrorKind::WouldBlock`; other filesystem errors propagate immediately.
- Existing inode and mutual exclusion are retained: no unlink/replacement,
  lock stealing, unsafe code, global lock or unbounded blocking acquisition.
- The synchronous API remains synchronous. This may occupy its calling thread
  for at most the bounded contention interval, plus filesystem/scheduler time;
  it is not advertised as cancellable during that interval.
- Deletion still takes recovery ownership before session/transcript ownership;
  no call sites or lock order changed. The registry mutex remains lookup-only.
- The new spool regression checks the original transcript record and identity,
  pending count, duplicate retry and exact one-record durable reconstruction.
- Both changed Rust files remain below 300 lines. No outside card was modified;
  BUG-254 remains specifying and BUG-253/255/256/257/258 remain done.

## Quality / traceability checkpoint

- `rustfmt --edition 2021 --check` on both changed Rust files: passed.
- `git diff --check`: passed.
- Native single-feature `validate`, `validate-tags`, `audit-coverage`: passed.
- Four scenarios have native coverage mappings to their actual tests and
  `rust/core/src/persistence/session_lifecycle.rs`.
- `cargo check --offline --profile ci-test -p codelet-core -j2`: passed.
- `cargo clippy --offline --profile ci-test -p codelet-core --test bug259_recovery_contention -j2`: passed, no reported warnings.
- Final post-format run of the new target: 4 passed, 0 failed; `final-ci-test.log`.
- Last disk checkpoint: 13 GiB free, above the strict 6 GiB floor.
- Canonically sorted outside-card records hash identically to HEAD:
  `6adda4a757eef7bdcf26c75249cafb228673d50c122e54beaf2514c764667bce`.
- Native `show-work-unit --format json` confirms Done. Final feature tags have
  @done and no @wip; native tag validation passes. Multi-tag layout was expanded
  before native removal because that command removes whole tag lines only.
