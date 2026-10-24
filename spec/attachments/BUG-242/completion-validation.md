# BUG-242 completion validation

## Acceptance and test-first evidence

`compaction-checkpoint-settlement.feature` defines four scenarios with one
integration test per scenario. The executable RED artifact is
`bug242-full-repair-red2.log`: the two unresolved-outcome synthesis scenarios
fail at the settlement boundary; completed-evidence and invalid-lineage scenarios
pass. The implementation follows that RED run.

The actual Codex adapter, native checkpoint, streaming serializer, request
admission and synchronized independent tool actor are exercised. Tool outcomes
marked unknown never claim successful execution, cancellation or non-execution.
Tool effects are not retried. Invalid lineage prevents synthesis and publication.

## Validation commands

Run from `rust/` with exit status preserved:

```sh
set -o pipefail
cargo test -p codelet-providers --test codex_replay_regressions --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-provider-replay-final.log
cargo test -p codelet-agent-loop --test bug242_checkpoint_settlement --test bug234_elapsed_runway --no-fail-fast --profile release-slim -j2 -- --nocapture 2>&1 | tee /tmp/bug242-integrated-green3.log
```

Provider replay: **5 passed, 0 failed**. This includes genuine-outcome enforcement,
native tool identity after restart, reasoning interleaving and tool continuation.
Agent-loop settlement: **4 passed, 0 failed**. Elapsed-runway regressions:
**5 passed, 0 failed**. Both normal timeout scenarios physically dispatch admitted
tool-free synthesis while preserving the native checkpoint. The invalid-lineage
scenario preserves the prior durable DAG on both publication entry routes.

Rustfmt checks pass for the eight focused BUG-242 Rust files. Feature syntax,
registered tags and coverage path audit pass. Four scenario mappings contain
18 exact literal step annotations and seven implementation anchors each.
The coverage mapping is requirement traceability, not instrumented branch coverage.

Repository-wide Gherkin parsing passes for all 2,079 feature files at audit time.
Repository-wide tag validation reports violations in 526 files outside this
card's feature. BUG-242's scoped tag validation passes.

## Independent completion audit

The coordinator reviewed the approved repair plan, actual Codex settlement,
streaming history capture, synthesis preparation/admission and all four mapped
tests. Both zero-result and partial-batch cases physically send tool-free
synthesis. Genuine outcomes retain native item/call identities; conflicting
results and changed model lineage fail closed. Runtime unknown-outcome records
explicitly prohibit retry and do not assert success or cancellation. Late actor
completion cannot rewrite the settled handoff. The publication tests exercise
the registered handler seams and preserve the prior committed DAG; they do not
claim remote credential/factory coverage.

The seven production files in this card have not changed since 11:59 local time,
before the completed 13:06 integrated GREEN. The durable copies
`bug242-integrated-green3.log` and `bug242-provider-replay-final.log` contain the
actual passing 4 settlement, 5 elapsed-runway and 5 provider-replay tests.

Subsequent integrated compilation succeeds in `/tmp/bug249-250-check.log`.
Integrated core, agent-loop and sessions library Clippy also succeeds with
`--no-deps -- -D warnings` in `/tmp/bug249-250-clippy.log`. Provider/CLI/core
library Clippy succeeds in `/tmp/bug243-clippy.log`. These completed checks
supersede the earlier in-flight shared-source compilation failure in
`/tmp/bug242-integrated-check.log`; that failed invocation is not called GREEN.

The renewed focused syntax/tag/path audit is
`/tmp/bug242-243-completion-specs.log`; focused rustfmt passes in
`/tmp/bug242-243-completion-format.log`. Exact scenario/step bounds and
implementation anchors were independently checked. No new production edit was
needed by this audit.

A redundant repeat at `/tmp/bug242-completion-tests.log` was interrupted while
still waiting on the artifact lock to avoid delaying the remaining cards' new
regressions. It did not execute tests and is not completion evidence. Completion
uses the verified earlier canonical GREEN on unchanged owned sources and the
completed subsequent integrated quality checks, not a queued command.
