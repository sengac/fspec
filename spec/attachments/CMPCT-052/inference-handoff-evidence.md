# CMPCT-052 inference handoff evidence

Date: 2026-10-17. This record covers the tool-loop handoff slice, not completion
of the dependent durable commit and generation gates.

## Acceptance-first sequence

The feature `spec/features/compaction-inference-handoff.feature` precedes its
five mapped integration tests in `rust/cli/tests/cmpct052_inference_handoff.rs`.
Tests drive the real streaming dispatcher and registered tools against a local
scripted HTTP/SSE provider; they count captured physical requests and inspect
actual retained tool-call/result pairs.

The original three scenarios failed behaviorally before implementation:
`/tmp/cmpct052-red.log` records 0 passed, 3 failed. The later self-target generation
scenarios failed before their implementation: `/tmp/cmpct052-generation-red.log`
records 3 passed, 2 failed. The explicit absence of an unmeasured remaining budget
failed before its implementation: `/tmp/cmpct052-budget-red.log` records 4 passed,
1 failed. These are behavioral failures, not the earlier compilation blockers.

`/tmp/cmpct052-green.log` records all five tests passing after implementation.
The command is:

```sh
cd rust
cargo test --profile ci-test -p codelet-cli --test cmpct052_inference_handoff
```

## Implemented boundary

- Accepted `inject_summary` and self-target `GenerateCompaction` results report
  `status: queued`, not a durable-commit acknowledgement.
- The stream yields immediately after recording the real tool call/result pair,
  before polling the provider loop for its next physical request.
- Rejected submissions produce `CompactionHandoffError` and no completion event.
- Queued summary estimates omit the unmeasured remaining physical-request budget.
- All five scenarios have one test mapping and implementation mappings.
- The feature passes targeted formatting, syntax validation and tag validation.

## Remaining completion gate

CMPCT-053, CMPCT-055 and CMPCT-059 own lock release, source-generation freshness,
suffix retention, durable promotion, provider lineage and inference blocking
until reconciliation. A queued handoff alone is not sufficient to mark this card
done. End-to-end verification must observe a subsequent request using the durable
committed generation, and no request/completion after a failed commit.
