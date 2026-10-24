# Independent completion audit

Status: done after the final finish audit; no commits. Earlier validating entries
below retain their original chronology and are superseded by the final section.

## Prerequisite chronology

1. Read BUG-247/248 repair plans, features, tests, prior logs, durability/framing
   contracts, BUG-246 coordination contract and current shared persistence code.
2. Added four stable-ID acceptance scenarios and the corresponding executable
   integration target `bug247_stable_message_identity` before implementation.
3. Ran `cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity`.
   `/tmp/bug247-stable-red.log` records the missing API E0599 in
   `conversation_commit.rs:20`. This is compilation RED, not behavioral RED.
4. Added the primitive in `messages/write.rs`, preserving the supplied UUID,
   reconciling under the stable writer lock, comparing the actual disk record,
   rejecting incompatible role/content/metadata, and resynchronizing existing
   bytes plus directories before acknowledging a retry. No duplicate append or
   new timestamp is used for an existing matching identity.
5. Ran `cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity --test bug248_reference_evidence --no-fail-fast`.
   `/tmp/bug247-248-audit-first-run.log` shows core compiled, then the core test
   dependency `codelet-agent-loop` failed E0308 at `agent_loop.rs:403`:
   `StreamChunk::error` requires `String`, but the new persistence admission gate
   supplies `&str`. No behavioral result is claimed for that invocation.
6. Repeated the same scoped command with output at
   `/tmp/bug247-248-audit-red-green.log`; results remain pending.

## Ownership

No modifications to `conversation_commit.rs`, manifest writers, agent-loop,
provider-state, journal or transcript files were made by this audit. Attempts to
message BUG-243 owner `44135c59-e9f1-4233-a3db-31d97e8ebc4e`, journal owner
`bc2c91d4-3bec-46f2-b35e-1a32328d3022` and parent
`906cec88-f3fd-42ca-8c88-b778da3d6f32` were rejected as busy. This attachment and
the durability contract record the exact API and prerequisite status for them.

## Quality evidence

Both changed feature files passed serialized `fspec format` and `fspec validate`.
All board/coverage commands use `/tmp/fspec-bug242-252-command.py`. New stable-ID
scenario mappings are linked. Direct rustfmt on changed Rust files succeeded.
Global `fspec validate-tags` reports unrelated repository tag violations; it is
not a clean global result. No destructive power-loss test was performed.

The following scoped quality commands now pass:

- `cargo check --profile release-slim -j 2 -p codelet-core`
  (`/tmp/bug247-248-audit-check.log`).
- `cargo clippy --profile release-slim -j 2 -p codelet-core --lib`
  (`/tmp/bug247-248-audit-clippy.log`).
- `rustfmt --edition 2021 --check rust/core/src/persistence/messages/write.rs rust/core/tests/bug247_stable_message_identity.rs rust/core/tests/bug248_reference_evidence.rs`.
- Serialized `fspec audit-coverage durable-message-before-manifest` and
  `fspec audit-coverage message-jsonl-tail-recovery` (path existence checks only).

Global tag validation exits 1: 1563 feature files pass and 526 have violations
(`/tmp/bug247-248-audit-tags.log`). Neither audited feature is listed as violating.
BUG-246 owner `c2793a18-b209-46f8-b9a5-6330234080f7` received both the compilation
blocker and successful core API/check/clippy handoff. Core test retry output is
at `/tmp/bug247-248-audit-behavior2.log`.

## First behavioral execution

The stable-ID ambiguity, conflict (including unchanged manifest bytes), and
concurrency scenarios pass. The conversation manifest-acknowledgement scenario
failed with `NotFound` because the test did not reset global stores between its
temporary roots. Added the same explicit `reset_stores_for_tests` fixture boundary
used by the other global-store tests; no conversation production change was
needed. The initial failure is not represented as a production defect.

The same run exposed a real BUG-248 referenced-tail data-loss regression, repaired
only after that executable RED. See BUG-248's completion audit. All four scoped
targets now run together in `/tmp/bug247-248-audit-green.log`; their final results
remain to be recorded.

## Final recorded results

BUG-247 and BUG-248 remain `validating`; no commits or done transitions were made.

- `/tmp/bug247-248-audit-final-tests.log` executes the four requested targets:
  durability 5/5, stable identity 3/4, reference evidence 2/2, tail recovery 7/7.
  Its sole failure was the conversation test's misplaced temporary-root reset,
  not a production failure. The entire command is **not** claimed GREEN.
- Corrected the reset at the actual `stable conversation` session creation site
  and verified its location. The test also requires a completed real
  `message.directory` barrier before the injected manifest failure, so an unrelated
  pre-append error cannot satisfy the scenario.
- `cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity`
  then passes **4/4**, including all three manifest promotion/acknowledgement
  boundaries. Exact output: `/tmp/bug247-stable-final-green.log`.
- Thus all **18 distinct scenarios across the four scoped targets** have passing
  executions, across the full run and the corrected stable-ID rerun. This is not
  a claim that the earlier full invocation exited successfully.
- Post-production-repair `cargo check --profile release-slim -j 2 -p codelet-core`
  and `cargo clippy --profile release-slim -j 2 -p codelet-core --lib` both pass:
  `/tmp/bug247-248-audit-final-check.log`,
  `/tmp/bug247-248-audit-final-clippy.log`. Subsequent local changes are test-only.

All commands run from `rust/` using the existing target and `release-slim -j 2`.
No additional target directory, workspace-wide test, live-store mutation, or
power-loss experiment was used. Board and coverage writes were serialized through
`/tmp/fspec-bug242-252-command.py`. Every scenario has one current test mapping;
feature validation and scoped rustfmt checks pass. Repository-wide tag validation
remains a separately recorded unrelated failure.

## Final finish audit

The finishing audit rereads and verifies the actual prior Cargo RED/GREEN and
check/Clippy logs listed here, not just their prose summary. No new BUG-247
production changes are needed. Current full core sources also pass all **9/9**
BUG-247 tests: durability **5/5**, stable identity **4/4**, in
`/tmp/bug247-250-core-finish-direct-regression.log`.

The supplementary runner recompiles the entire actual core crate with matching
dependencies from the existing release-slim target and links the tracked tests
with the real shared helper crate. Command:
`python3 /tmp/bug249-core-finish-run.py core/tests/bug247_message_durability.rs core/tests/bug247_stable_message_identity.rs core/tests/bug248_reference_evidence.rs core/tests/bug248_tail_recovery.rs core/tests/bug249_journal_integrity.rs core/tests/bug249_transcript_journal.rs core/tests/bug250_transcript_outcomes.rs`.
All seven targets pass **28/28**, including both BUG-247 targets. Current full-core
Clippy with `-D warnings` exits **0** in
`/tmp/bug247-250-core-finish-direct-clippy.log`. Scoped rustfmt, feature syntax,
registered tags, and all **20/20** coverage paths pass. No power-loss claim is added.

The fresh canonical Cargo repeat is invoked with the existing profile/config and
is still waiting behind another owner's long integration-test optimization:
`/tmp/bug247-250-core-finish-tests.log`. This is not claimed as a fresh Cargo GREEN.
Completion relies on the verified earlier canonical target/check/Clippy executions
plus current full-source regression/compiler/Clippy evidence, not that queued
repeat. All current core journal/deletion repairs and **33** distinct passing
core scenarios are detailed in `spec/attachments/BUG-249/core-finish-audit.md`.

The serialized `validating -> done` transition succeeds after recording this
evidence. BUG-249/250 status remains with the parent coordinator.

## Final one-feature/one-test-file correction

The final goal audit finds that the original durability feature still combines
the five durability tests and four stable-identity tests. Per-scenario coverage
alone does not satisfy fspec's separate one-feature/one-test-file validation.
BUG-247 is reopened through specifying and testing, without bypass flags.

The four stable-identity scenarios, unchanged step-for-step, move through fspec
commands into `stable-message-commit-identities.feature`. All original test and
implementation links are retained. The test file gains only its correct feature
documentation attribute; its executable body is unchanged. fspec relinking
adjusts every test range by those two documentation lines and refreshes the
original feature's aggregate statistics.

`/tmp/bug247-one-feature-factor.log`,
`/tmp/bug247-248-one-feature-relink.log`, and
`/tmp/bug247-248-one-feature-stats.log` record the actual commands and successful
validation. The implementing and validating transitions rerun the mandatory
one-feature/one-test-file, literal step, and complete coverage gates. No reverse
ACDD, manual coverage edits, new acceptance behavior, or production edits occur.

The fresh canonical command in
`/tmp/bug242-252-follow-goal-canonical.log` has executed all five durability and
four stable-identity tests successfully, alongside all nine BUG-248 tests and
the checkpoint preservation regression. Only the feature documentation attribute
changes after that passing execution; no test assertion or production behavior
changes. The historical behavioral RED remains attached. Scoped rustfmt and
whitespace checks pass. The card remains validating for the final shared Cargo
quality gates before its final done transition.

The final shared canonical gates now complete successfully:
`/tmp/bug249-250-completion-cargo.log` ends with `CARGO_COMPLETION_EXIT=0`,
including current core/agent-loop/sessions library check and strict Clippy
`--no-deps -- -D warnings`. This validates the actual current persistence source,
not an alternate compiler build. Both factored features pass their fspec gates.
BUG-247 is therefore eligible for the final validating-to-done transition.
