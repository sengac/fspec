# BUG-246 validation evidence

Status: done through the normal fspec workflow transition. All three final
canonical Cargo gates executed successfully. No commits.

## Canonical Cargo completion audit

This section supersedes the older pending statements below. Only actual Cargo
execution is used for completion; historical direct-source runs are supplemental.

- `/tmp/bug246-250-final-integrated-cargo.log` (18:06): release-slim Cargo
  execution passes 16 BUG-246 cases in seven integration targets, including the
  two session-ownership cases. No failed or ignored BUG-246 cases.
- `/tmp/bug246-finish-green.log` (17:15): release-slim Cargo execution passes
  `bug246_user_admission` as well as the earlier six BUG-246 targets. Together
  these logs execute all 17 current scenarios across eight targets. They do not
  establish that user admission was rerun in the later integrated invocation.
- `/tmp/bug246-completion-user-admission.log`: the required later current-source
  user-admission Cargo rerun passes, 1 passed, 0 failed, `CARGO_EXIT=0`. This
  includes the supervisor's nonsemantic expression/module-order formatting and
  supersedes the intentionally interrupted duplicate invocation.
- The 18:06 integrated log also passes seven BUG-249 caller cases across
  `bug249_admission_ownership`, `bug249_executor_gate`,
  `bug249_interrupted_tool_state`, and `bug249_stream_crash`. The ignored
  subprocess fixture is explicitly launched by its four process-kill parents.
  BUG-249 completion remains with its assigned owner.
- That same log passes the BUG-250 deleted-session case and seven retirement
  parents, including cold recovery and active session ownership. The earlier
  sequential-only lifecycle limitation is superseded by the shared
  `RecoveryLease` implementation and these executed tests.
- `/tmp/bug242-252-final-clippy.log` is NOT a global pass: it fails on the
  unrelated `fspec-tui/src/store/agent_view/token_state.rs:100`
  `collapsible_match` lint. `/tmp/bug246-finish-clippy.log` also fails, on an
  intermediate missing `RequestStarted` core variant. Neither log is recorded
  as a successful scoped BUG-246 Clippy gate.
- `/tmp/bug246-completion-cargo-quality.log`: both fresh scoped canonical
  commands finish successfully, `CHECK_EXIT=0` and `CLIPPY_EXIT=0`:
  `cargo check --profile release-slim -j2 -p codelet-agent-loop --lib` and
  `cargo clippy --profile release-slim -j2 -p codelet-agent-loop --lib -- -D warnings`.
  A redundant queued user-admission rerun was interrupted after its existing
  Cargo GREEN was located (`USER_ADMISSION_EXIT=130`); that interruption is not
  a test failure or a passing execution. The pipeline's aggregate status also
  includes that interruption; the individual quality markers and the separate
  completed current-source runtime invocation establish the final gates.

## Current traceability and source audit

- `/tmp/bug246-completion-acdd.log`: all eight feature files pass scoped syntax,
  registered-tag and referenced-file audits; all 17 scenarios are fully mapped.
  This does not claim global tag compliance or measured code coverage.
- `/tmp/bug246-completion-acdd-final.log`: the same eight checks pass after
  refreshing coordinates; each scenario has exactly one test mapping and at
  least one implementation mapping. Working-tree and staged scoped whitespace
  checks also pass.
- `/tmp/bug246-completion-traceability.log` and
  `/tmp/bug246-completion-journal-traceability.log`: fspec commands replace stale
  pre-refactor coordinates, recompute coverage statistics, link the archive
  commit helper and user inference gate, and preserve one test per scenario.
  No temporal or step-validation bypass is used.
- `/tmp/bug246-completion-format.log`: scoped rustfmt passes with exit 0 after
  whitespace/import-order corrections in two owned integration tests. The
  existing Cargo behavioral results precede these nonsemantic corrections.
- Owned production files are below 300 lines and contain no inline comments,
  deprecated compatibility branches, empty-state recovery fallbacks, or newly
  duplicated ownership/locking implementations. No production changes were
  necessary in this completion audit. Shared core/background-output sources
  were not edited.
- Coordination messages to the supervisor and BUG-249/250 owner were rejected
  because both were busy; they were not queued. This attachment records the
  audit without claiming message delivery. BUG-249 caller tests were read-only.

The three final gates are completed, not queued. No work-unit JSON was edited
manually. The normal `update-work-unit-status` done transition succeeded with
no workflow bypass. All eight linked features carry registered `@done` tags;
their syntax/tag checks pass in `/tmp/bug246-completion-done.log`. Final scoped
ACDD, rustfmt and whitespace checks pass in
`/tmp/bug246-completion-final-gates.log`. No remaining BUG-246 completion gate
is pending, and no direct compiler substitutions were used in this pass.

## Historical completion audit evidence

- `/tmp/bug246-finish-integrated-direct-green.log`: 15 passed across all seven
  BUG-246 integration targets, compiled from the current agent-loop source with
  matching current Cargo dependencies. This includes the final shared archive
  projection changes, original-storage reader correction, ordering validation and
  deleted-session guard. No failed or ignored BUG-246 tests.
- `/tmp/bug249-250-finish-integrated-direct-green.log`: 10 caller tests passed;
  one child-only subprocess helper is intentionally ignored by the outer runner
  and explicitly launched by its four process-kill parent tests.
- `/tmp/bug246-finish-integrated-direct-clippy.log`: current-source library Clippy
  completes successfully with warnings denied. `/tmp/bug246-finish-format.log`
  records successful scoped rustfmt checks. Scoped git diff whitespace check passes.
- `/tmp/bug246-finish-global-validate.log`: all 2102 feature files are valid.
  `/tmp/bug246-finish-spec-validation.log`: all seven BUG-246 feature files pass
  syntax and registered-tag checks. Global tag compliance is not claimed.
- Coverage is linked to implementation and each scenario's exact test. The
  features were split by test target to satisfy the workflow's mandatory
  one-feature/one-test-file gate; the validating transition succeeds without
  bypassing temporal or coverage validation.

Direct runs use Rust's actual integration tests, actual current production
source, real stores, sentinel tools and subprocess termination. They preserve
release-slim optimization level 3, 16 codegen units and thin-LTO test linking,
and change no Cargo profile/configuration. They are not mocks or source scans.

## Behavioral RED and repair

- `/tmp/bug246-finish-baseline.log`: unavailable recovery journal lost the
  authentic assistant original, 0 instead of 1. Six other baseline tests passed.
- `/tmp/bug246-finish-capacity-red.log`: the actual Cargo-built capacity target
  lost the captured authentic result, 0 instead of 1, after the real sentinel ran.
- `/tmp/bug246-finish-corrupt-direct-red.log`: failed disk reads admitted another
  record into memory, retained count 2 instead of 1.
- `/tmp/bug246-ordering-red.log`: a corrupted durable ordering filename silently
  allowed another admission instead of returning an explicit repair error.
- `/tmp/bug250-deleted-recovery-red.log`: retained ownership recreated a removed
  transcript after its session was deleted.

Repairs retain authentic originals after journal refusal, implement the specified
three bounded retained slots, drain admitted records before promoting retained
ones, propagate direct admission failures, validate physical ordering/identity,
and refuse transcript recovery into a missing session. No compaction-settlement
files changed in this caller audit.

## Remaining integration gates

- `/tmp/bug246-corrupt-admission-red.log` is now an executed Cargo GREEN:
  2 passed. Its filename describes enqueue intent, not its eventual result.
- `/tmp/bug246-finish-green.log`: complete final Cargo BUG-246 invocation queued.
- `/tmp/bug249-caller-finish-baseline.log`: final Cargo caller invocation queued.
- `/tmp/bug250-deleted-recovery-cargo.log`: deleted-session Cargo target queued.
- `/tmp/bug246-finish-check.log` and `/tmp/bug246-finish-clippy.log`: final scoped
  Cargo quality checks queued. Queued commands are not passes.
- BUG-249/250 status remains parent-owned. Core lifecycle coordination still
  needs recovery-spool cleanup and consistent delete/append ownership. The
  sequential deleted-session guard does not establish concurrent serialization.
  Details: `spec/attachments/BUG-246/coordination-contract.md`.

## Historical evidence

## Executed evidence

- `/tmp/bug246-red-stable.log`: three real-filesystem recovery scenarios failed
  before the bounded recovery owner was implemented.
- `/tmp/bug246-identity-compiler-red.log`: supplemental missing stable UUID import
  evidence; artifact-version errors in this log are not behavioral evidence.
- `/tmp/bug246-new-red.log`: Cargo release-slim, `-j2`, journal promotion target:
  the stable physical assistant/result identity test passed. Two empty-original
  assertions revealed that the test reader used model replay, which emits a
  replay-coordinate envelope even when the original archive is empty. The helper
  now uses bounded archival pages. Those failures do not demonstrate a routing bug.

## Earlier incomplete scoped runs

- `/tmp/bug246-capacity-runtime-red.log`: actual sentinel tool, dispatch admission,
  128 retained envelopes, authentic result capture, output-owner drop and explicit
  recovery. Capacity retention production changes wait for this executable RED.
- `/tmp/bug246-owned-boundary-red.log`: three scoped recovery/admission/commit targets.
- `/tmp/bug246-current-red.log`: canceled before the full agent-loop target finished
  thin-LTO linking, to prioritize capacity RED. Compilation is not a passing run.

## Specifications

`/tmp/bug246-fspec-validate.log` reports all 2091 feature files valid. Scoped
registered-tag checks pass for both modified BUG-246 features. Global tag
validation reports violations outside these features; scoped results
must not be presented as a global tag pass.
