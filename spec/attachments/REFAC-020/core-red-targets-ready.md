# REFAC-020 core acceptance tests ready for parent-run RED

## ACDD chronology and scope

Parent approved the bounded core plan/API and replaced the worker role before
repository work. Native REFAC-020 discovery captured a user story, four rules,
six concrete examples and the approved architectural boundary. Specifying
preceded `precise-lossless-coverage-mutations.feature`; native format, validate
and validate-tags succeeded. AST research was attached before the successful
native transition to testing. Only then were the focused Rust tests created.

The feature has 15 scenarios and 15 corresponding business acceptance test
functions, each carrying all three exact Given/When/Then step comments. Static
scenario/test/comment reconciliation reports zero mismatches. The transaction
target also contains one ignored subprocess-only worker entrypoint; it is test
infrastructure, not an extra acceptance scenario or a passing behavior claim.

No production/core help/CLI/Cargo source edits, builds, behavioral test runs,
migrations, installs, restarts, commits or history/security changes occurred.
The original four command source hashes still match the preimplementation plan.
All new maintained Rust test sources are below 300 physical lines.

## Parent preparation

The core crate currently has no shared test-helper dev dependency. Before RED,
parent should add `codelet-test-helpers.workspace = true` under core
`[dev-dependencies]`; workspace registration already exists. This worker left
Cargo files to the parent's exclusive ownership. A missing dependency/compiler
failure is not causal RED.

A read-only `rustfmt --check` parsed the new files without syntax errors, but
reported formatting differences. Log: `/tmp/refac020-core-test-format-check.txt`.
Formatting is pending, not claimed passing. It must remain scoped to these new
test roots/helper directories and retain the physical source ceiling.

## Exact Cargo targets

Run from `rust/`, parent-scheduled only:

```sh
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_selected_coverage -j 2
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_scoped_generation -j 2
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_coverage_transactions -j 2
```

Test root files retain their exact Cargo target names. Shared fixtures live in
`tests/refac020/support.rs`; each target's ordinary helper modules live in its
matching directory. No alternative executable, daemon, installed fspec binary,
new runtime builder, production sidecar or process-global environment is used.
Transaction children execute only the current integration-test executable's
ignored worker entry, with isolated root/command/IPC environment on that child.

## Intended causal failures versus controls

- Selected linking uses an independently computed valid canonical hash, not the
  unavailable new show view, so its RED reaches the wrong-duplicate mutation.
- Indexed unlink must retain the first row and exactly one identical obsolete
  occurrence, then permit implementation removal only inside the selected row.
- Selector rejection exercises stale/malformed hashes, exact file mismatch,
  bounds, negative/fractional indices, unpaired arguments and incompatible modes.
- Readback alone tests the explicit machine response, ordered row indices,
  unknown-member-sensitive hashes, object-order invariance and array-order
  sensitivity while preserving legacy JSON and exact bytes.
- Generation scope checks unrelated mapped drift and missing sidecars before
  checking machine output. Preservation checks raw retained metadata before
  parsing the new report, so lossiness is not masked by a missing response mode.
- Blocked-removal/corruption tests execute actual generation and inspect its
  refusal/unchanged bytes before checking preview report shape. They must expose
  destructive old behavior rather than merely fail to parse its legacy output.
- Preview tests cover create/update/skip, exact additions/empty removals, no-op
  byte preservation and no file/directory/lock creation during dry-run.
- Lock tests use real cross-process fs2 locks and explicit ready/go/entered/result
  handshakes. Before release they require no completion or sidecar mutation;
  then they commit a new mapping plus unknown receipt under the held guard.
  Successful completion after release must retain that whole latest scenario,
  proving read-after-lock rather than merely a delayed/stale atomic write.
- Competing indexed children share a fresh revision: exactly one must commit,
  one must report stale/hash failure, and complete mapping arrays must match the
  winning addition without loss. File-only atomic replacement is insufficient.

Two scenarios are expected preservation controls that may already be GREEN:
normal file/story-step validation, and unselected legacy duplicate selection.
Do not count those as newly established RED. The subprocess worker is ignored
outside its parent-controlled invocation. No executed counts are asserted here.

## Approved limitations and next gate

Only participating link/unlink/global and selected generation paths are in the
lock guarantee. Scenario/create/delete writers, report-to-sidecar writes and
the old daemon are nonparticipants. Parent must prohibit concurrent nonparticipant
writes during the exclusive pinned one-time migration; no repository-wide
transaction claim is made. Do not expand this repair to those commands.

The new acceptance feature intentionally has no bootstrap sidecar yet. Parent
approved including REFAC-020 features in the post-GREEN/review validated scoped
CLI migration; do not run destructive global generation or directly edit one.

Next required gate: parent records causal target-specific RED against unchanged
production, resolves fixture/compiler problems without treating them as RED,
and explicitly authorizes production. Until then core remains in testing.

Independent-review follow-up: `core-proof-gaps-red-ready.md` supersedes this
receipt's initial 15-scenario counts and pins with 17 scenarios and strengthened
lock/metadata/ambiguity/path-identity/exact-inventory checks. Preserve this earlier
receipt as the original ACDD readiness checkpoint, not the current source pin.
