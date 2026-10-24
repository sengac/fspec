# BUG-233: Overlapping same-depth compaction ranges

Research date: 2026-10-22. HITL-approved investigation and outcome expectations;
no production fix or test result is claimed here. Source references are triage snapshots.

## Context and impact

Two submitted summaries contained ranges that conflict at the same DAG depth.
Rejection is intentional: conflicting peers make hierarchical memory ambiguous.
The desired fix targets generation or bounded correction without relaxing the
distinction between invalid peer overlap and valid parent/child coverage.

## Incident evidence

- Log: `~/.fspec/logs/fspec-combined.log.2026-10-22:516` and `:562`.
- Session: `3248e143-7a8f-45f4-906d-64ff1aacae20`.
- UTC times: 01:33:28 and 01:49:47.
- Error: `summary submission blocked ordinary inference: invalid DAG candidate: overlapping same-depth dag-node ranges`.
- The rejected candidate bodies have not been inspected; duplicate candidate replay
  versus two independently generated conflicts remains unresolved.

## Code and range semantics

- `rust/core/src/compaction/dag_syntax.rs:79` sorts nodes and checks ranges per depth.
- Bounds are inclusive: a peer starting at the previous peer's end overlaps it.
  For example, same-depth ranges 10–20 and 20–30 conflict; 10–20 and 21–30 do not
  conflict on overlap alone, though all other provenance/coverage rules still apply.
- Overlap at different depths is intentional and must remain supported.
- `rust/core/src/compaction/dag_validation.rs:102` wraps syntax/range rejection.
- `rust/cli/src/interactive/compaction_handoff.rs:36` and
  `rust/cli/src/interactive/stream_loop.rs:1076` enforce the submission barrier.
- `spec/features/whole-dag-candidate-validation.feature:35` specifies acceptance of
  cross-depth overlap and rejection of same-depth overlap.

Related completed work: CMPCT-036, CMPCT-070, CMPCT-052, and BUG-211. Check the
incident-time runtime revision. BUG-232 handles malformed/missing node structure;
BUG-234 handles time-budget exhaustion. Neither sibling replaces this range contract.

## Confirmed versus unconfirmed

Confirmed: the validator detected overlapping ranges at the same depth and prevented
the submitted candidate from becoming a successful ordinary-inference handoff.

Likely causes to investigate: duplicate peers, overlapping condensation output,
incorrect depth assignment, or confusion over inclusive endpoints. The logs do not
identify which producer or correction step introduced the conflict.

## Expected outcomes

1. Reproduce the conflicting candidate in a minimal sanitized fixture and identify
   the route that introduced or retained the conflicting ranges.
2. Generation/correction uses explicit persisted-coordinate, depth, and inclusive-bound
   semantics consistent with the core validator.
3. A bounded repair produces a complete newly validated candidate, or fails clearly
   without changing the durable conversation boundary.
4. Diagnostics identify conflicting depths and bounds plus operation/attempt identity
   without dumping the full conversation by default.
5. Valid hierarchical overlap, source coverage, and preserved-history contracts remain
   unchanged across supported submission routes.

## Regression evidence to produce

- Reject duplicates, partial intersections, and shared inclusive endpoints at one depth.
- Accept valid cross-depth nesting and disjoint same-depth peers.
- Cover unsorted candidate nodes so input order cannot hide a conflict.
- Exercise both bounded correction success and exhausted-correction rejection through
  a real submission path; verify no invalid promotion or inferred coverage occurs.
- Review `rust/core/tests/cmpct070_whole_dag.rs:1`,
  `rust/core/tests/cmpct070_promotion_validation.rs:1`, and
  `rust/agent-loop/tests/cmpct052_durable_submission.rs:1` for reusable coverage.

## Scope limits and completion

Do not delete conflicting nodes, clamp their bounds, invent uncovered ranges, reject
all cross-depth overlap, or silently install a partial candidate. Any repaired text
must satisfy the complete existing validator. Completion requires scenario-linked
regression evidence and scoped checks demonstrating both conflict prevention/recovery
and preservation of valid hierarchy.

## Implementation investigation (2026-10-22)

The incident executable revision and rejected candidate bodies remain unavailable.
Checkout `fd32e995` and installed fspec 0.10.9 are present-day observations only.
No claim is made that inclusive endpoint confusion caused the incident.

The current fresh prompt says all ranges should be non-overlapping, which is
inaccurate for valid cross-depth hierarchy. Incremental instructions promote D0
to D1 and D1 to D2 without explicitly checking newly colliding peers. The strict
validator correctly rejects peers but reports only a generic overlap reason,
omitting conflicting depth and bounds. The planned correction improves producer
guidance and diagnostics, not acceptance rules or automatic candidate editing.

Scenario-linked runtime HTTP/handoff tests cover duplicate, contained,
shared-endpoint and unsorted conflicts, valid disjoint peers with cross-depth
nesting, successful replacement and exhausted rejection. A property test covers
arbitrary larger endpoints, genuine partial intersections at shared endpoints,
input ordering and unchanged valid candidate text. Features preceded tests;
all four scenarios have test mappings before implementation.

## ACDD and validation evidence

The runtime tests are now all in
`rust/agent-loop/tests/bug233_peer_range_generation.rs`, one test per scenario,
including the arbitrary-endpoint property test. This satisfies fspec's enforced
one-feature/one-test-file mapping. Shared setup remains in a separate helper file
under 300 lines and reuses the existing durable-session fixture and test helpers.

- Observed RED: missing same-depth generation rules, missing depth/range details
  in correction and exhausted failure, and missing arbitrary conflict diagnostics.
  `/tmp/bug232-233-red-final.log` and `/tmp/bug233-red-core.log` preserve the runs.
- Observed initial GREEN: all four scenarios passed unchanged after the producer
  contract and diagnostic correction. `/tmp/bug232-233-green-final.log`.
- A follow-up RED runtime logging assertion confirms correction warnings lacked
  target correlation. `/tmp/bug232-233-log-target-red.log`; a generator span is
  added only after that failure, and final verification passes all seven sibling
  scenarios, including the runtime logging assertions.
- The validator still sorts complete candidates and rejects the whole candidate
  on every peer intersection. It now retains the previous peer's start as well as
  end only to report `depth=D1, ranges=0-1 and 1-1 (inclusive)`. No surviving subset,
  rewritten coordinate, generated payload, or permissive parser is introduced.
- Six whole-candidate/promotion compatibility tests and three existing builder
  validation tests pass. `cargo check` passes for core, CLI and agent-loop.
- Shared Clippy is blocked by another worker's denied `expect_used` in
  `common/src/logging/privacy.rs`. Legacy handoff fixtures omit required root
  attribution, and four BUG-211 cases fail an unchanged token-count fixture
  assumption before building. Detailed commands and limits are recorded in the
  sibling BUG-232 attachment; these unrelated failures are not concealed or fixed.
- Both feature syntax checks, both coverage audits, and `git diff --check` pass;
  global tag/check failures do not name the new features. Current coverage is
  100% with exactly one runtime/property test per scenario and implementation links.

No overlapping `compaction_builder.rs` edit is made; timing/runway remains owned
by BUG-234/235. The incident root producer and runtime revision remain unconfirmed.

### Final results

- `cargo test --offline --profile ci-test -p codelet-agent-loop
  --test bug232_malformed_generation --test bug233_peer_range_generation
  --no-fail-fast -j 4`: **7 passed, 0 failed**. The property explicitly covers genuine
  partial intersections as well as shared endpoints, randomized order, disjoint
  peers and cross-depth hierarchy. `/tmp/bug232-233-complete-green.log`.
- `cargo check` and `cargo clippy`, both scoped to core/CLI/agent-loop libraries
  with `--offline --profile ci-test --lib -j 4`: **both exit 0** after the other
  worker corrected its unrelated common-logging lint failures.
  `/tmp/bug232-233-final-check.log`, `/tmp/bug232-233-final-clippy.log`.
- Targeted Rust formatting, whitespace checks, syntax validation and coverage audit
  pass. Full `fspec validate`: **2062 valid features**. This feature has **100%**
  coverage: four scenarios, one runtime/property test per scenario, implementation
  links, no missing mappings. Global tag/check baseline failures remain documented.
- The final combined timing worker changes reuse the operation identity for bounded
  correction. No builder, deadline, runway or cancellation edits are owned by this
  card, and no permissive parsing, clamping, node deletion or invented summary is
  introduced. No commit or branch was created.

BUG-232 and BUG-233 are both `done`, with registered component/completion tags.
The final global tag run reports 526 unrelated violating files and no violation
for either new feature. Scenario and implementation coverage audits both pass.
