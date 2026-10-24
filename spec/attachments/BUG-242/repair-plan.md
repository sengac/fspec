# BUG-242: Reconcile Codex compactor timeout settlement

## Priority and evidence

**P1 — observed session-blocking failure, with a source-supported causal chain.**
At 22:25:43 AEST on October 22, operation `38ae0a08-5cb0-4852-b017-b9a02eb454da`
for session `7a093866-f589-4d59-817e-3a86a189e9fd` failed with
`Codex replay has no recorded outcome for call call_b99ac1e445f64c1799c0d184694f2be1`.
Exploration timed out after 99.833 seconds; synthesis failed in approximately 2 ms
with 499 seconds remaining. This was not the overall operation deadline expiring.

All 662 parent records decode, and all 218 parent tool calls have saved outcomes.
The error call is absent from those records and checkpoints. The missing call belongs
to the ephemeral compactor path; its tool name and exact execution state were not logged.

## Root cause and code to inspect

- `rust/agent-loop/src/compaction_execution.rs:84`: timeout drops exploration; line 100 reuses the same agent for synthesis.
- `rust/providers/src/codex/streaming.rs:219`: provider output is checkpointed before executable calls are yielded.
- `rust/agent-loop/src/compaction_runway.rs:52`: generic recovered history retains completed calls only.
- `rust/providers/src/codex/history_lineage.rs:99`: saved provider items are reintroduced.
- `rust/providers/src/codex/history_replay.rs:18`: unanswered calls correctly fail request preparation.
- `rust/cli/src/compaction_timing.rs:37`: exploration receives half the candidate allowance.

Two authorities disagree: generic history excludes unfinished calls while Codex state
still requires their outcomes. BUG-234 introduced elapsed-time handoff but did not test
this combination with the actual Codex adapter.

## Required repair

1. Model the handoff as an explicit lifecycle transition: stop scheduling work, settle
   started/unstarted calls, capture completed evidence, and establish coherent synthesis state.
2. Preserve authentic results, call IDs, item IDs, native reasoning, and response-attempt identity.
3. Record truthful cancelled/not-executed outcomes when supported by the established protocol.
   If isolated synthesis is chosen instead, define an explicit validated generation transition
   from completed evidence; obtain architecture approval before major refactoring.
4. Reconcile both generic and provider history before synthesis admission/dispatch.
5. Preserve the absolute deadline, cancellation ownership, last committed DAG, source coverage,
   selected model, output reservation, and durable publication gates on every route.
6. Emit payload-free handoff/settlement diagnostics sufficient to identify the phase and pending
   call identities without recording tool arguments, results, or hidden reasoning.

## Regression scenarios to specify before implementation

- Real `CodexResponsesModel`: a checkpointed first tool blocks before any result; exploration
  expires; coherent tool-free synthesis can finish or returns a deliberate typed settlement failure.
- Same response contains a completed originals read followed by a blocked tool; completed evidence
  survives exactly once and unfinished execution is not represented as successful.
- A later checkpointed call has not started when cutoff occurs; it receives truthful lifecycle handling.
- Cancellation races provider publication, tool completion, and synthesis preparation.
- Registered automatic recovery and direct generation preserve their intended shared contract.
- Failed settlement/admission does not publish a DAG, emit success, or repeat tool side effects.

Use synchronization to establish boundaries, not timing-only sleeps. Existing
`bug234_elapsed_runway` streaming fixtures use ordinary OpenAI Responses through a `zai`
label; they do not exercise Codex checkpoint authority. Extend real-Codex fixtures.
Keep `codex_replay_regressions::interrupted_checkpoint_requires_a_genuine_outcome_and_remains_recoverable`.

## Do not fix this by

- Removing `MissingOutcome`, fabricating successful results, or merely deleting generic calls.
- Resetting/replacing provider state without an explicit supported lineage transition.
- Raising timeouts, retrying blindly, truncating evidence, or weakening admission/coverage checks.

## Completion evidence

- [ ] Feature scenarios precede failing actual-Codex integration regressions and implementation.
- [ ] Zero-result and partial-batch boundaries are covered through physical synthesis preparation.
- [ ] Original history remains intact; errors preserve the prior durable generation.
- [ ] Scoped tests, compilation, clippy, formatting, feature validation, and coverage mappings pass.

Related: BUG-234, BUG-243, PROV-050. Full evidence:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
