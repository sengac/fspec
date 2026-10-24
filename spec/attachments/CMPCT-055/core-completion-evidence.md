# Core/tools completion — 2026-10-18

Exclusive ownership remains `rust/core/**`, `rust/tools/**` and corresponding
055/064/070/072 specifications, coverage and attachments. Existing index and
worktree edits are preserved. No board changes, staging or commits.

## Completion-pass file inventory

Production edits: `rust/core/src/compaction/dag_syntax.rs`,
`rust/core/src/compaction_hook.rs`, and `rust/core/src/persistence/` files
`compaction_boundary.rs`, `compaction_commit.rs`, `manifest.rs`,
`manifest_commit.rs`, `manifest_updates.rs`, `session_replay.rs`.
All focused production modules remain below 300 lines; the inherited oversized
hook and manifest receive surgical changes, not broad decomposition.

Tests edited: `rust/core/tests/cmpct055_generation_commit.rs`,
`cmpct064_manifest_transactions.rs`, `cmpct070_whole_dag.rs`,
`cmpct072_durable_preservation.rs`. Tests added:
`rust/core/tests/cmpct055_publication_ownership.rs`,
`rust/core/tests/cmpct064/lost_ack.rs`,
`rust/core/tests/cmpct070_cold_boundary.rs`,
`rust/core/tests/cmpct082_fail_closed_hook.rs`, and
`rust/tools/tests/compaction_generation_dispatch.rs`.

Specification and coverage edits: both `.feature` and `.feature.coverage` for
`versioned-dag-generation-commit`, `durable-session-manifest-transactions`,
`whole-dag-candidate-validation`, `durable-dag-preservation` under
`spec/features/`. Added completion attachments are this file and
`spec/attachments/CMPCT-070/2026-10-18-core-audit.md`.
Other inherited staged/unstaged work is not attributed to this completion pass.

## Latest verified checkpoint

Final post-hook regression `/tmp/cmpct-core-complete-final-green.log` passes
34 top-level tests across nine explicitly named core integration targets.
Its additional successful child-process test summaries are not counted twice.
Exact invocation from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct055_generation_commit --test cmpct055_publication_ownership --test cmpct059_durable_boundary --test cmpct064_manifest_transactions --test cmpct070_cold_boundary --test cmpct070_promotion_validation --test cmpct070_whole_dag --test cmpct072_durable_preservation --test cmpct082_fail_closed_hook -j2 --no-fail-fast
```

Final successful quality commands and complete logs:

```
cargo check --profile ci-test -p codelet-core -p codelet-tools -j2
# /tmp/cmpct-core-complete-final-check.log
cargo clippy --profile ci-test -p codelet-core -p codelet-tools --lib -j2
# /tmp/cmpct-core-complete-final-clippy.log
```

The explicitly scoped test Clippy attempt remains blocked by the sessions
dependency, not passed: `cargo clippy --profile ci-test -p codelet-core --test
cmpct082_fail_closed_hook --test cmpct055_publication_ownership -j2` fails on
`sessions/src/committed_context.rs:23` (`clippy::expect_used`) in
`/tmp/cmpct-core-complete-final-test-clippy.log`. The sessions owner received
this blocker and the unchecked provider-reset warnings; core does not suppress
the lint or modify the dependency. Shared dependency warnings are retained.

Final four-feature validation passes in
`/tmp/cmpct-core-complete-final-specs.log`. Read-only scenario-name set, mapping
range/statistics and required registered tag checks pass in
`/tmp/cmpct-core-complete-final-mappings.log` for all 6/9/8/6 owned scenarios.
This confirms traceability, not measured line coverage or live route acceptance.

The assigned 082 core-hook blocker is now implemented and verified. Existing
082 acceptance preceded the supporting core behavioral test; the provider-owned
feature and sidecar were not edited. `/tmp/cmpct-core-hook-phases-red.log`
executes the test and fails with `before=false, finish=[false, false]`.
`/tmp/cmpct-core-hook-phases-green2.log` passes that same test after the fix.
The first green attempt `/tmp/cmpct-core-hook-phases-green.log` failed to compile
because an inserted lock guard landed in the test helper instead of the finish
hook; this was corrected before the successful run, not counted as green.

Exact core RED and GREEN invocation from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct082_fail_closed_hook -j2
```

The physical provider reproduction now passes both tests in
`/tmp/cmpct-core-hook-provider-green.log`, using:

```
cargo test --profile ci-test -p codelet-providers --features test-support --test cmpct082_physical_admission -j2
```

The production change is limited to `rust/core/src/compaction_hook.rs`: poisoned
state cancels before sending and at response finish, including when provider
usage normalization returns no usable usage. No new public API or module is
introduced for this fix. Provider traceability handoff: supporting core test
`rust/core/tests/cmpct082_fail_closed_hook.rs:12-55`, implementation
`rust/core/src/compaction_hook.rs:196-200` and `:254-279`. The provider owner owns
any 082 sidecar changes. Hook/test formatting passes in
`/tmp/cmpct-core-hook-fmt-green.log`; scoped whitespace validation passes in
`/tmp/cmpct-core-hook-diff-check.log`.

The ownership follow-up now passes both tests in
`/tmp/cmpct-core-ownership-green2.log`. The first green attempt was compilation
blocked by an incorrect inserted guard reference; it is not successful evidence.
`/tmp/cmpct-core-ownership-regression.log` then passes all 33 tests across the
eight core targets (the seven listed below plus publication ownership).

Exact regression command from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct055_generation_commit --test cmpct055_publication_ownership --test cmpct059_durable_boundary --test cmpct064_manifest_transactions --test cmpct070_cold_boundary --test cmpct070_promotion_validation --test cmpct070_whole_dag --test cmpct072_durable_preservation -j2 --no-fail-fast
```

Both `cargo check --profile ci-test -p codelet-core -p codelet-tools -j2` and
`cargo clippy --profile ci-test -p codelet-core -p codelet-tools --lib -j2`
finish successfully in `/tmp/cmpct-core-ownership-check.log` and
`/tmp/cmpct-core-ownership-clippy.log`. Dependency warnings remain outside this
ownership. Test-target Clippy for `cmpct055_publication_ownership` is blocked in
`/tmp/cmpct-core-ownership-test-clippy.log` by sessions' production
`committed_context.rs` using `expect` under `clippy::expect_used`; this is not
a successful quality result and belongs to the sessions owner.

Latest read-only traceability verification is
`/tmp/cmpct-core-final-owned-mappings.log`: exact scenario names, all mapped
file/range bounds, registered feature tags and unique test-line totals pass for
055/064/070/072 (6/9/8/6 scenarios; 301/278/342/172 mapped test lines).
`/tmp/cmpct-core-final-owned-specs.log` validates all four owned feature files.
These figures are scenario traceability, not measured execution coverage.

The 08:43 coordinator handoff additionally assigns the existing core
`CompactionHook` poisoned-lock blocker. Its acceptance already exists in
`explicit-request-admission-without-fallback.feature`; no provider-owned feature
or mapping edits are authorized here. Reproduction
`/tmp/cmpct-core-hook-poison-red.log` actually executes the provider test and
fails `unavailable guard emitted completion`. A core-owned additional test
`cmpct082_fail_closed_hook` exercises pre-request and response-finish checks
with both valid and rejected usage normalization. This is a cross-owner core
implementation handoff for 082, not evidence that the four owned cards prove
every runtime route or the final gate.

## Coordinator review follow-up — ownership and irreversible promotion

Two additional feature scenarios preceded `cmpct055_publication_ownership`.
Actual behavioral RED `/tmp/cmpct-core-ownership-red.log` has both failures:
the callback child blocks on nested `load_session` until its bounded parent
terminates it, and a real post-rename directory-open failure is exposed only as
ordinary `CompactionCommitError::Persistence`, without the operation identity.

Exact RED and GREEN invocation from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct055_publication_ownership -j2 --no-fail-fast
```

After RED, a scoped writer guard now rejects same-thread manifest facade
reads/writes and direct `SessionStore` writer reentry before acquiring ownership.
Publication retains the transaction for the authoritative pending check,
notification and acknowledgement, so there is no detached stale-notification
window. The two legacy `manifest.rs` changes are access preflight calls only;
no broad refactor or formatting of that oversized inherited module was done.
Callbacks must not spawn a separate thread and synchronously wait for another
session operation; cross-thread waiting is outside this synchronous API contract.

API handoff for CLI/loop: `CompactionCommitError::Promoted { identity, source }`
now distinguishes irreversible rename followed by failed directory durability
acknowledgement from a pre-promotion persistence error. `promoted_identity()`
returns the exact identity for both `Promoted` and `Publication` errors.
Commit, publication and reconciliation preserve this identity. Runtime callers
must retain the inference barrier and pending ownership on either error and
reconcile exactly that identity; never build a replacement generation or treat
this result as rejection before promotion. This requires runtime-owner evidence.

The failure is real filesystem behavior, not a fault injector: on Unix an
existing writable/searchable but unreadable sessions directory permits staging
and atomic rename, then denies opening the directory for its durability sync.
Permissions are restored before inspecting the error and cold committed bytes.
The reentry test runs in a bounded real child process to prevent a regression
from hanging the test runner. GREEN/quality results follow after completion.

## New acceptance work

Feature updates precede new behavioral tests for:

- 055: obsolete/already acknowledged snapshots must never publish a completion
  before discovering that their identity is no longer pending and authoritative.
- 070: invalid promoted DAGs must not be reconciled into an inference-ready
  boundary; cold replay rejects absent revision/model/output-reservation metadata.
- 072: missing immutable originals fail source capture and durable promotion
  before admission rather than allowing a summary to conceal unavailable evidence.

First new RED attempt `/tmp/cmpct-core-completion-red.log` is compilation-only:
CLI Session initializers had duplicate/missing selected-preparer/operation fields.
The CLI owner repaired those fields. `/tmp/cmpct-core-completion-red2.log`
then executed 12 tests: 8 passed and 4 failed across all three targets. The
failures are obsolete publication accepted, invalid DAG reconciliation accepted,
empty committed model accepted by cold replay, and missing-original capture
accepted. This is behavioral RED, unlike the first compilation-only attempt.

Exact RED invocation from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct055_generation_commit --test cmpct070_cold_boundary --test cmpct072_durable_preservation -j2 --no-fail-fast
```

Only after that execution, minimal production fixes were applied to
`core/src/persistence/compaction_boundary.rs` and `compaction_commit.rs`:

- Source capture/promotion validate immutable original availability, including
  current suffix references, before invoking admission.
- Cold boundaries require a nonzero committed revision, nonempty selected model
  and nonzero admitted output reservation as well as valid whole DAG lineage.
- Reconciliation revalidates the authoritative boundary and originals before
  clearing publication pending; invalid memory leaves manifest bytes unchanged.
- Publication checks the current pending identity under the writer transaction
  before notifying, and acknowledges that same boundary in the transaction.
  Obsolete or already acknowledged snapshots cannot emit another completion.

GREEN `/tmp/cmpct-core-completion-green.log`: all 31 tests pass across seven
targets: 055 (4), 059 (4), 064 (9), 070 cold (2), 070 promotion (1), 070 whole (5),
072 (6). The 059 target also executes its actual fresh-process cold reader;
064 lost-ack executes the actual child writer exiting 74 after commit.

Exact GREEN invocation from `rust/`:

```
cargo test --profile ci-test -p codelet-core --test cmpct055_generation_commit --test cmpct059_durable_boundary --test cmpct064_manifest_transactions --test cmpct070_cold_boundary --test cmpct070_promotion_validation --test cmpct070_whole_dag --test cmpct072_durable_preservation -j2 --no-fail-fast
```

Tools existing baseline completed: `/tmp/cmpct-core-tools-post-hook-green.log`
has four facade integration tests passing; `/tmp/cmpct-core-tools-post-hook-inject.log`
has seven selected inject-summary unit tests passing. These do not prove the
agent-loop/CLI durable routes. Earlier seven-target core green remains recorded
in `CMPCT-070/2026-10-18-core-audit.md` (28 tests before this new extension).

## API and dependency coordination

No public API/schema was added. Existing capture, commit, publication,
reconciliation and cold-boundary checks were strengthened. Publication callbacks
must not recursively mutate the manifest store, matching the existing admission
callback contract; authority checks/publication/reconciliation should share the
writer transaction rather than check a detached snapshot then notify.

CLI/loop owners retain actual direct/deferred/self/cross route obligations for
072. Core validator tests that loop over route names are not runtime proof.
Use `DagValidationContext` captured authoritatively before builder invocation
and revalidate at the writer-locked admission seam before physical preparation.

## Traceability and quality

Owned mappings now cover all 4/9/8/6 scenarios with valid test and implementation
ranges. `/tmp/cmpct-core-completion-mappings.log` records a read-only check of
exact scenario names, registered/required feature tags, range bounds and mapped
test-line totals. These are traceability counts, not measured line coverage.
`/tmp/cmpct-core-completion-specs.log`: all four owned features format and
validate. Global `fspec validate-tags` still fails for existing unrelated files;
full output is `/tmp/cmpct-core-completion-tags-all.log`. Owned tags pass.

Check/clippy logs are `/tmp/cmpct-core-completion-check.log` and
`/tmp/cmpct-core-completion-clippy.log`; results are recorded after completion.

Scope limitations: 064's partial stage is planted, not an actual writer killed
inside fsync/rename; the real lost-ack process exits after `SessionStore::commit`
returns. No Windows crash-durability equivalence is claimed (promotion explicitly
rejects unsupported platforms). Core contract tests do not replace production
route evidence or the full seven-provider/eight-negative-row gate owned elsewhere.

Attempts to notify parent/current CLI owner returned `session_busy` and were
not queued. This attachment is the shared mailbox until final handoff.
