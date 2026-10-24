# BUG-232: Malformed compaction DAG submissions

Research date: 2026-10-22. Context and outcome expectations approved through HITL
log triage; this document is not evidence of an implemented fix or passing tests.
Source references describe the triage snapshot and may move as development continues.

## Context and impact

Three summary submissions were rejected for missing or unclosed DAG nodes. Rejecting
invalid memory is correct, but repeated invalid submissions prevent the session from
returning to ordinary inference. The work is to understand and improve generation,
submission, and bounded correction, not to make invalid summaries acceptable.

## Incident evidence

- Log: `~/.fspec/logs/fspec-combined.log.2026-10-21:654`, `:661`, and `:669`.
- Session: `e284c6cd-1886-492e-b7ec-0430e3eeb893`.
- UTC times: 23:49:42, 23:50:07, and 23:50:50.
- Error: `summary submission blocked ordinary inference: invalid DAG candidate: missing or unclosed dag-node structure`.
- Two builder-correction warnings occur in the following day's log at `:25` and
  `:165`. They lack operation/session correlation and must not be counted as
  confirmed additional failures of this session.

Local log paths are investigative pointers, not portable test fixtures. Preserve only
minimal, sanitized candidate evidence in repository fixtures; do not copy full prompts
or unrelated conversation content into this attachment.

## Code and existing contracts

- `rust/core/src/compaction/dag_syntax.rs:12` matches complete DAG nodes;
  `rust/core/src/compaction/dag_syntax.rs:75` rejects missing/unmatched structure.
- `rust/agent-loop/src/turn_compaction.rs:60` validates before queueing a submission.
- `rust/cli/src/interactive/compaction_handoff.rs:36` requires a successful queued
  handoff, rather than treating tool completion as successful compaction.
- `rust/cli/src/interactive/stream_loop.rs:1076` blocks inference after rejection.
- `rust/cli/src/compaction_operation.rs:159` implements bounded candidate correction.
- `spec/features/whole-dag-candidate-validation.feature:25` explicitly forbids
  salvaging valid-looking subsets from malformed candidates.

Related completed work: CMPCT-070, CMPCT-052, BUG-211, and BUG-229. Confirm the
incident-time executable revision before describing this as a regression in current code.
BUG-233 owns same-depth overlap; BUG-234 owns operation-budget exhaustion.

## Confirmed versus unconfirmed

Confirmed: validation rejected the submissions, and the submission barrier prevented
ordinary inference. This protects the committed conversation boundary.

Unconfirmed: whether the model returned prose without DAG nodes, omitted a closing
tag, produced malformed syntax, or was truncated. The error covers multiple causes;
it is not proof of output-token exhaustion. Repeated log entries do not establish
whether the same candidate was replayed or distinct candidates were submitted.

## Expected outcomes

1. A sanitized reproduction identifies the failing generation/submission route and
   distinguishes absent structure, malformed structure, and truncated output where
   the provider supplies sufficient evidence.
2. Candidate generation and bounded correction follow the established DAG contract;
   recoverable invalid output can produce a newly validated complete candidate.
3. Exhausted or unavailable correction leaves the previous committed generation intact,
   keeps the inference barrier effective, and exposes an actionable failure.
4. Diagnostics identify the operation, session, attempt, and validation reason without
   requiring full prompt or candidate logging by default.
5. Valid candidates continue to use authoritative persisted coordinates and coverage.

## Regression evidence to produce

- Cover plain prose/no nodes, unmatched tags, and malformed material after an otherwise
  valid node; none may be installed or partially salvaged.
- Exercise the actual submission/handoff path, not only parser helpers.
- Verify invalid-then-valid bounded correction and exhausted-correction failure.
- Verify the durable boundary and ordinary-inference admission remain unchanged on failure.
- Review `rust/core/tests/cmpct070_whole_dag.rs:1`,
  `rust/agent-loop/tests/cmpct052_durable_submission.rs:1`, and
  `rust/cli/tests/cmpct052_inference_handoff.rs:1` before adding coverage.

## Scope limits and completion

Do not weaken syntax validation, clamp coordinates, fabricate a substitute DAG, or
resume ordinary inference simply because a tool returned. Do not expand this into an
unrelated compactor rewrite. Completion requires scenario-linked failing-then-passing
tests, scoped quality checks, and evidence that the initiating problem is addressed
while existing rejection guarantees remain intact.

## Implementation investigation (2026-10-22)

The current checkout is `fd32e995`; installed `fspec --version` reports 0.10.9.
Neither proves the revision running at the incident timestamps. The local log
contains the reported rejection messages, but no incident candidate body or
correlated provider finish reason was recovered. Truncation, replay, and exact
producer identity therefore remain unconfirmed; synthetic candidates below are
sanitized reproductions, not reconstructed incident content.

Current code already bounds builder corrections to three complete validations.
However, incremental instructions omit the complete node syntax demonstrated by
the fresh prompt, and generic correction repeats only the validation error and
candidate. It provides neither operation/attempt correlation nor an explicit
complete-node remedy. Runtime regressions exercise a real provider HTTP boundary,
GenerateCompaction tool, production registration, strict validator, queued handoff,
and durable application using persisted originals. They include prose, unclosed
nodes, and an otherwise valid candidate followed by malformed structure.

Features were written and validated before test creation; BUG-232 entered testing
before tests were created. Scenario-to-test links cover all three scenarios.
Implementation and final validation results will be recorded after observed RED.

## ACDD and validation evidence

- Initial runtime RED: `cargo test --offline --profile ci-test -p codelet-agent-loop
  --test bug232_malformed_generation --test bug233_peer_range_generation
  --no-fail-fast -j 4` failed all six HTTP/handoff scenarios on missing producer
  guidance, operation/target diagnostics, and conflict details. Output:
  `/tmp/bug232-233-red-final.log`. The range property test separately failed on
  the missing conflicting-depth diagnostic before being moved into BUG-233's
  single scenario-mapped test target, preserving fspec's one-feature/one-test-file rule.
- Initial GREEN: the same two targets passed all seven scenarios (3 + 4), including
  malformed-then-valid correction, exactly three rejected attempts, cold durable
  manifest equality on failure, and no ordinary request after summary handoff.
  Output: `/tmp/bug232-233-green-final.log`.
- A second test-first iteration captures actual tracing output across spawned tool
  tasks. `/tmp/bug232-233-log-target-red.log` proves successful correction warnings
  lacked the target session even though they had operation/attempt fields. A target
  and caller span is added around the production generator; the final rerun passes.
- `cargo check --offline --profile ci-test -p codelet-core -p codelet-cli
  -p codelet-agent-loop --lib -j 4` passed (`/tmp/bug232-233-check.log`).
- The equivalent `cargo clippy` invocation reached an unrelated concurrent change
  in `common/src/logging/privacy.rs:109,111` and failed its denied `expect_used`
  lint. This file is not owned or changed by BUG-232/233. That worker subsequently
  corrected it, and the final scoped Clippy run passes as recorded below.
- Whole-candidate/promotion controls: six tests passed in
  `cmpct070_whole_dag` and `cmpct070_promotion_validation`.
  The three `compaction_builder_validation` tests passed unchanged.
- Legacy positive handoff controls fail before candidate generation because their
  synthetic tool arguments omit now-required root `work_unit_id` (two CLI cases,
  one agent-loop durable-submission case). Rejection controls still pass. The new
  tests explicitly supply calling-session `work_unit_id: NONE` and an invoking
  project admission scope; they do not disable attribution or weaken any gate.
- `bug211_compaction_budget_recovery` has four successes and four failures inside
  the unchanged fixture's request-token assertion (11371 versus 90336), before its
  builder behaviour. These unrelated fixture failures are not repaired here.
- Both new feature files validate; coverage audit finds every linked file and
  reports 100% scenario/test/implementation coverage. Global `fspec validate-tags`
  and `fspec check` remain red on hundreds of pre-existing repository tag violations;
  neither new feature is listed among those violations. `git diff --check` passes.

No changes are made to `compaction_builder.rs`, operation deadlines, runway,
cancellation policy, whole-candidate acceptance, or original coordinate boundaries.
Shared builder ownership remains with BUG-234/235's worker. Initial compile/fixture
setup errors and transient concurrent builder syntax errors were not counted as
valid RED evidence or as successful validation.

### Final results

- `/tmp/bug232-233-complete-green.log`: **7 passed, 0 failed**, including actual
  warning records containing target session, shared operation identity, attempt,
  and validation reason. Raw rejected candidate payloads are absent from those
  warnings. The property includes genuine partial intersections in addition to
  duplicates, containment, unsorted peers, inclusive endpoint conflicts, disjoint
  peers and valid cross-depth hierarchy.
- `/tmp/bug232-233-final-check.log` and
  `/tmp/bug232-233-final-clippy.log`: both commands exit **0** for core, CLI and
  agent-loop libraries using `--offline --profile ci-test --lib -j 4`.
- Targeted `rustfmt --check`, `git diff --check`, both feature syntax validations,
  and both coverage audits pass. `fspec validate` passes all **2062** feature files
  (`/tmp/bug232-233-all-features-validate.log`). Coverage is **100%**, three scenarios,
  one test each, with implementation mappings.
- Existing baseline/control failures described above remain explicitly unresolved;
  no tool-attribution bypass, token-fixture change, global tag cleanup, or unrelated
  production edit is included in this work.

BUG-234's worker integrated its shared timing identity into the existing correction
loop while retaining this change's guidance and rejection handling. BUG-232/233
never edited `compaction_builder.rs`. Final runtime tests run against that combined
working tree. No commit or branch was created.

Both cards completed the fspec lifecycle through `done`; both features carry
registered `@cli` and `@done` tags. Final global tag validation still reports 526
unrelated violating files and does not list either new feature.
