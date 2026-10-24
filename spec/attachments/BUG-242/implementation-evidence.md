# BUG-242 implementation evidence

Historical evidence for the initial failure-only implementation follows. It is
superseded by `architecture-approval.md` and `full-repair-evidence.md`; it is NOT
evidence that the approved normal-timeout repair is complete.

## Design and ownership

Chosen repair-plan option: deliberate typed settlement failure when a genuine
outcome is unavailable. Dropping rig's requester does not cancel its independent
ToolServer actor. Claiming cancellation or success would be dishonest. The new
barrier stops synthesis before admission when native obligations are unresolved.

`CompletionModel::settle_tool_results` defaults to no native obligations. Codex
matches captured results by BOTH native item ID and call ID, uses rig's real
Responses result serializer, rejects conflicting outcomes, preserves native
items and the generation, and retains completed evidence once. The ordinary
replay validator still rejects missing outcomes. No reset, isolated lineage,
timeout increase, fallback DAG, automatic tool retry, or publication change.

Owned production files: agent-loop `compaction_execution.rs` and new
`compaction_settlement.rs`; Codex `model.rs` and new `settlement.rs`; the small
default hook in patched rig's `completion/request.rs`. No changes to session
persistence, provider history modules, `background_output`, or BUG-243's files.
Existing runway capture is reused rather than unnecessarily refactored.

## ACDD chronology

1. Read AGENTS, testing guide, FOUNDATION, repair plan; inspect AST timeout sites,
   Codex checkpoint publication, replay authority and independent rig actor.
2. Capture three rules, four examples, architecture notes and estimate 5.
3. Create and validate `compaction-checkpoint-settlement.feature` BEFORE tests;
   transition specifying -> testing through the serialized command wrapper.
4. Create actual-Codex integration target and synchronization fixtures.
5. Executable RED, before any production edits: **1 passed, 3 failed** with the
   genuine `Codex replay has no recorded outcome for call blocked-call`.
   Raw log: `spec/attachments/BUG-242/bug242-red.log`.
6. Transition testing -> implementing, add minimal boundary integration.
   First implementation attempt had compiler errors (hook anchoring, borrowed
   iteration and Arc inference); these are NOT behavioral RED evidence.
   That attempt is retained honestly at `/tmp/bug242-green.log`.
7. The second GREEN invocation was cancelled while still waiting for Cargo's
   artifact lock; `/tmp/bug242-green2.log` contains no test execution. Its targets
   were consolidated with the regression suite to avoid repeated build queues.
   Corrected implementation GREEN: pending. Scoped compilation and clippy pass,
   with only an unused function warning from reused pre-existing test support.

## Exact commands

Run from `rust/`, preserving exit status with `set -o pipefail`:

```sh
cargo test -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-red.log
cargo test -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-green.log
cargo test -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-green2.log
cargo test -p codelet-agent-loop -p codelet-providers --test bug242_checkpoint_settlement --test bug229_codex_runway --test bug234_elapsed_runway --test codex_replay_regressions --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-green-regressions.log
cargo test -p codelet-agent-loop -p codelet-providers --test bug242_checkpoint_settlement --test bug234_elapsed_runway --test codex_replay_regressions --no-fail-fast --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-green-final.log
cargo check -p codelet-agent-loop --test bug242_checkpoint_settlement --profile release-slim -j2 2>&1 | tee /tmp/bug242-check-agent.log
cargo clippy -p codelet-agent-loop -p codelet-providers -p rig-core --lib --test bug242_checkpoint_settlement --profile release-slim -j2 2>&1 | tee /tmp/bug242-clippy.log
```

All board and coverage mutations use
`python3 /tmp/fspec-bug242-252-command.py <command> <args>`.

## Scenario mapping

One scenario maps to one test in
`rust/agent-loop/tests/bug242_checkpoint_settlement.rs`, in feature order:

1. `scenario_a_blocked_first_checkpointed_call_fails_settlement_deliberately`
2. `scenario_partial_batches_preserve_genuine_evidence_without_inventing_later_outcomes`
3. `scenario_completed_checkpoint_evidence_crosses_the_synthesis_barrier`
4. `scenario_both_publication_routes_preserve_the_prior_dag_on_settlement_failure`

There are exactly 17 unique, literal `@step` annotations, each appearing once.
Coverage metadata has one test mapping per scenario and five production anchors.
Coverage is scenario traceability, NOT a claim of 100% line/branch instrumentation.

## Test boundaries and limitations

- Real `CodexResponsesModel`, its actual SSE adapter, checkpoint history, physical
  request serializer/admission and real persisted originals are exercised.
  Only the external HTTP model endpoint and a synchronized blocking tool are
  fixtures. Notify boundaries prove started, unstarted and late-completed work;
  no sleep is used to guess whether the first tool started.
- The two publication-entry tests inject the actual Codex execution kernel at
  the registered handler seam, avoiding remote credentials/global config. They
  do not test the remote credential/provider factory setup. The direct route
  invokes the real GenerateCompaction tool with an injected generator handler;
  it does not exercise the unmodified factory's success/deferred-apply route.
- This repair intentionally does not introduce cancellation-and-ack semantics
  into rig's tool actor. Unknown executions fail deliberately; no false cancelled
  outcome is manufactured. A separate cancellation protocol would require a
  broader design. Tool outcomes arriving after cutoff do not rewrite history.
- Replay and elapsed-runway regressions: pending. Codex occupancy regression
  `bug229_codex_runway` is NOT GREEN: its fixture fails before this repair's path
  (`bug210_completion_metrics.rs:110`, expected 90336, observed 11371). Request
  admission now counts actual tokens while that fixture asserts byte-derived
  constants. Its other large repeated-text boundary test remained CPU-bound for
  roughly three minutes and was intentionally terminated (SIGTERM). No success
  is claimed for that target. The complete failed/capped log is attached as
  `bug242-green-regressions.log`; the parent has been notified, and the unrelated
  fixture is not modified here. Other targets are rerun with `--no-fail-fast`.
- Rustfmt passes all six owned Rust files outside patched rig. The patched rig
  request file has pre-existing formatting failures, verified against `git show
  HEAD` with the crate's correct edition 2024. The new four-line hook introduces
  none of those differences; unrelated upstream formatting is intentionally
  untouched. Evidence: `/tmp/bug242-rig-format.log` and
  `/tmp/bug242-rig-format-baseline.log`. `git diff --check`, targeted Gherkin/tag
  validation, exact step checks and coverage-file audit pass.
- No workspace/unscoped Cargo commands, global config edits, commits or branches.
  Card must remain validating for the parent's independent audit, never done.
