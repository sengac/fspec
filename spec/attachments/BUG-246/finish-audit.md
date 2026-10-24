# BUG-246 caller completion audit

## Current completion handoff

The canonical Cargo completion audit in `validation-evidence.md` supersedes
pending/direct-run statements in the historical sections below. Actual Cargo
logs now cover all 17 BUG-246 scenarios in eight targets. The later integrated
run also covers seven related BUG-249 caller cases and the shared recovery
retirement lifecycle cases. No shared caller/core source or BUG-249 test edits
were made in this completion pass.

Owned-source inspection found no additional required production repair. Two
owned integration files needed formatting only. fspec coverage commands refresh
pre-refactor coordinates and statistics, include the user inference gate and
archive helper, and retain one scenario-to-test mapping per scenario. Scoped
syntax/tag/file-existence validation and rustfmt succeed. Fresh release-slim
Cargo check/Clippy complete in `/tmp/bug246-completion-cargo-quality.log`, with
`CHECK_EXIT=0` and `CLIPPY_EXIT=0`. The required later current-source runtime
rerun passes in `/tmp/bug246-completion-user-admission.log`, 1 passed and
`CARGO_EXIT=0`. The broad 18:06 Clippy log fails on an unrelated TUI lint and
is not used as the scoped quality gate. The interrupted duplicate invocation
(`USER_ADMISSION_EXIT=130`) is not used as a passing result.

All three required canonical gates are GREEN. Final fspec validation and the
normal done transition succeeded without bypassing workflow checks. All eight
linked features have registered `@done` tags and pass syntax/tag validation.
The final status is recorded in `/tmp/bug246-completion-done.log`; no BUG-246
completion gate remains pending.

No commits, manual work-unit edits, Cargo profile changes, direct-rustc runs,
fallback ownership paths, or new inline comments are introduced by this pass.

## Historical audit

Owner: current scoped agent-loop worker. Parent coordination messages to
`bb177085-4cc6-46e3-9c61-f3f68b435749` were rejected as busy; this attachment
records coordination without claiming delivery.

## Baseline findings

- The reserved three bounded in-process ownership slots specified in the feature
  and coordination contract are not implemented in the current spool.
- BackgroundOutput returns on journal admission failure before capturing genuine
  tool results or settling assistant originals. Existing BUG-246 recovery and
  capacity scenarios must execute before fixing those paths.
- Spool admission silently treats a failed disk read/lease as empty disk state
  before accepting another memory record. A new feature-first corruption test
  checks that direct failed admission leaves the original with its caller.
- Existing capacity/owned-boundary logs are compiler failures, not behavioral RED.
- Parent/core owner must coordinate BUG-250 session deletion. Current
  SessionStore::delete_owned only removes the manifest. Recovery must never
  recreate a deleted session's transcript; transcript retry currently does not
  verify the session still exists. No compaction-settlement files are owned here.

## Active validation

- `/tmp/bug246-finish-baseline.log`: Cargo release-slim, -j2, six existing
  BUG-246 test targets. Full dependency rebuild underway; no passing claim.
- `/tmp/bug246-corrupt-admission-red.log`: new scoped target queued behind it.
- Existing direct-test scripts cannot run yet: prior agent-loop fingerprint and
  matching artifacts are absent. No build configuration or artifact cleanup is
  performed by this worker.

## Executed repair sequence

- `/tmp/bug246-finish-baseline.log`: real Cargo baseline reached runtime after
  dependency compilation. Six journal/persistence tests passed; unavailable
  recovery journal lost the assistant original (0 instead of 1). Cargo stopped
  there; remaining compiled targets were not claimed as executed.
- `/tmp/bug246-finish-capacity-red.log`: that baseline's actual compiled capacity
  target failed authentic result count (0 instead of 1), after executing its
  real sentinel tool once.
- `/tmp/bug246-finish-corrupt-direct-red.log`: current-source direct integration
  test failed retained count (2 instead of 1). Failed durable reads had accepted
  an extra record into memory rather than leaving it with its caller.
- Implemented explicit admitted/retained ownership: 128/16 MiB admitted capacity,
  three additional retained records with a combined 16 MiB bound, and ordered
  drainage before promoting retained records. Direct enqueue propagates lock/read
  errors. Internal submission can retain the original but returns shutdown-unsafe
  failure, never durable success. No fallback to an empty durable state remains.
- Background recovery now captures authentic assistant and tool-result originals
  even when publication is refused. Terminal failure still settles originals.
  Result identity is stable and dispatch remains gated. Archive commit moved to
  a focused helper without changing its behavior; no compaction settlement edits.
- `/tmp/bug246-finish-current-direct-green.log`: optimized current-source runs
  now pass corruption, unavailable journal, capacity, ambiguous commit boundaries,
  temporary archive failures, authentic results and stable journal identities.
  Final user-admission result must be read before claiming the whole suite.
- `/tmp/bug246-finish-direct-clippy.log`: current-source library Clippy succeeds
  with warnings denied. The earlier own-variable compiler typo was corrected;
  it is not behavioral evidence.

The direct runner compiles the actual current agent-loop library and integration
targets against matching Cargo release-slim dependency identities, opt-level 3,
16 codegen units and thin-LTO test executables. It changes no Cargo profile or
configuration. The first supplemental linking attempt used plugin-only bitcode
and failed Apple LLVM compatibility; the successful invocation embeds Rust
bitcode with native objects. Only executed runtime output is GREEN evidence.

Final Cargo runs remain queued on the shared artifact lock:
`/tmp/bug246-finish-green.log`, `/tmp/bug249-caller-finish-baseline.log`, and
`/tmp/bug246-finish-check.log`. Do not mistake these pending logs for passes.

## BUG-249 acceptance alignment

The older caller-capacity test expected an empty conversation after recovery,
contradicting BUG-246's explicitly approved original-ownership requirement.
`/tmp/bug249-caller-ownership-audit.log` demonstrates that disagreement after
the ownership fix. The caller scenario now explicitly retains the authentic
prefix as an original once, without claiming that publication succeeded. No
production change was made merely to satisfy the former empty-original assertion.
Added actual subprocess cases cover aborted/replaced attempts and termination
before dispatch, in addition to acknowledged prefix and unknown external action.
BUG-249/250 status remains parent-owned.

## Final integrated audit

`validation-evidence.md` supersedes the intermediate pending statements above.
Current-source optimized tests pass 15 BUG-246 scenarios and 10 caller scenarios,
including four real process-kill parents and sequential deleted-session recovery.
The archival paging API now intentionally includes transcript display records;
the shared test reader therefore loads the exact manifest-referenced originals
through get_message, failing explicitly on a missing reference. It never filters
away a transcript incorrectly inserted into the originals lane. The intermediate
projection-related failure is `/tmp/bug250-deleted-recovery-green.log`; final
integrated GREEN logs are named explicitly in validation-evidence.md.

The public recovery open operation now propagates directory failures. Internal
submission derives its ownership key without claiming a directory was opened,
so it can retain bounded original ownership and report shutdown unsafe when
durable admission is impossible. Invalid durable filenames cannot reset sequence
numbering or admit another record.

No commits. BUG-246 advanced to validating after real integrated runtime evidence
and the one-feature/one-test-file workflow gate succeeded. Final Cargo quality
and full scoped Cargo test commands remain pending on the shared lock.
BUG-249/250 status remains parent-owned.
