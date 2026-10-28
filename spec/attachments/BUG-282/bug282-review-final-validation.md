# BUG-282 safety review — final validation

This report supersedes first-pass negative-coverage and completion claims.
It does not replace or rewrite the original accepted RED, raw archive or
historical 81-test receipts. No production code changes in this review.

## Corrected executable scope

Three Gherkin scenarios map 1:1 to three tests in
`rust/agent-loop/tests/bug282_canonical_tool_outcomes.rs`:

1. Real streamed object/escape output **and canonical JSON-string output with
   retained outer quotes** match event, runtime, durable original, checkpoint
   and physical continuation text exactly.
2. First/successive admitted compaction, publication, cold restore and real
   continuation retain authentic outcomes without repeating effects.
3. Seven actual invalid-state variants fail safely, as detailed below.

| Negative variant | Actual mutation | Exact typed `CompactionCommitError::Rejected` reason |
| --- | --- | --- |
| Outcome | Change existing function-call output payload | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry |
| CallArguments | Change actual function-call arguments; retain its IDs/name | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: tool call ancestry |
| CallName | Change actual function-call name; retain its IDs/arguments | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: tool call ancestry |
| OutputIdentity | Change outcome call ID, separately named | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry |
| Assistant | Change genuine existing assistant text | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry |
| MissingOutcome | Remove the genuine tool-result original's manifest reference; retain the archived record and checkpoint | incompatible provider state: checkpoint identity or source anchor |
| EstablishedDag | Alter existing checkpoint DAG text after successful real generation-1 commit/publication, cold restore and continuation | selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry |

All cases require no promoted identity, no publication, unchanged post-injection
manifest/checkpoint/referenced originals, unchanged pre-injection archived
original values including the removed outcome record, and blocked inference.
No new message substitutes for a missing outcome or an established DAG.

The first review acceptance run preserved the genuine missing state but expected
a length-specific refusal; it failed 2 passed/1 failed. The actual source-anchor
refusal arises from the changed prefix hash while the length still fits.
Only that exact expected diagnostic was corrected. This is not production RED
and the invalid state was not replaced. Both subsequent corrected executions
pass 3/3, with all seven refusals named in the raw log.

## Fresh quality results

All commands below exit **0** under the archived isolated runner, offline,
ci-test/debug0, -j2, using the original workspace/cache path:

```text
cargo test --offline --profile ci-test -j2 -p codelet-agent-loop --test bug282_canonical_tool_outcomes -- --nocapture
cargo check --offline --profile ci-test -j2 -p codelet-cli -p codelet-agent-loop --lib --test bug282_canonical_tool_outcomes
cargo clippy --offline --profile ci-test -j2 -p codelet-cli -p codelet-agent-loop --lib --test bug282_canonical_tool_outcomes -- -D warnings
rustfmt --check --edition 2021 --config skip_children=true cli/src/interactive/stream_handlers.rs cli/src/interactive/stream_handler_tests.rs agent-loop/tests/bug282_canonical_tool_outcomes.rs agent-loop/tests/support/bug282.rs agent-loop/tests/support/bug282_negative.rs
```

Scoped feature validation: valid, zero errors. Scoped tag validation: valid,
zero errors. Coverage audit: **all 7 mapped file instances found, all mappings
valid**. Three test mappings, one per scenario, with 11 exact step annotations.
Test/helper sizes are **207, 249, 260 lines**, all under 300. No lint suppression.

The old 81 distinct test receipts remain intact: 78 unchanged regression tests
were not rerun here, while the old three acceptance executions are superseded by
the fresh corrected acceptance. Verified unchanged production/test source hashes
justify retaining those prior regression receipts; no fresh 81-test run is
claimed. The earlier library-only build is also retained, not rerun or relabeled.

## Final source hashes

| File | SHA256 |
| --- | --- |
| `rust/agent-loop/tests/bug282_canonical_tool_outcomes.rs` | `3e7d49a32269f30a39f26c79d7f47cf4bbcb51308eff8bb152d8d900d5cc7390` |
| `rust/agent-loop/tests/support/bug282.rs` | `6dd5170ae32a028ab6c907d5e2e5849a41d9930f8228fe2a466d23a124584939` |
| `rust/agent-loop/tests/support/bug282_negative.rs` | `a0c37d9fdd89b55770871db94b09bbc32ecf4a0b10a72f7c683ecd3f0e21826d` |
| `spec/features/canonical-streamed-tool-outcomes.feature` | `af5fca71469bcd9669c6fc2e0288c3ce477341165a4741b8b853871526e06c35` |
| `spec/features/canonical-streamed-tool-outcomes.feature.coverage` | `fd78753a2502db1911842d822a4c9841764cfeab845b5e80460cc0e4b7db58aa` |

The handler and lineage/source-anchor guards are unchanged from before this
review. Their hashes are included in `final-selected-source.sha256` and the
passing preservation receipt. The full prior source manifest verifies except
the two deliberately revised BUG-282 test/helper files; the negative helper is
new test code only. BUG-280/281 handoff specification hashes also verify.

## Evidence chain and safety

| Artifact | SHA256 |
| --- | --- |
| Original accepted RED log, unchanged | `f6c2947dff78696c5804e281ef8bd1e2bf81f8e0930ca0bcc3a074183fc25af1` |
| Original first-pass raw archive, unchanged | `044cbf8c0d5680e63f825a31b2e0ebf6c14b79385dbb0eb37cccf89d6da64e50` |
| Review network-failure receipt, preserved | `a77c5dc8395b85110befeeaed85861eec14e13eae84c1b1088cb8d46ce4a5cad` |
| `bug282-review-findings.md` | `cc8e21e966d918de05db120efd1e7ec7279a0723f638d9a9610cd58a94daf08c` |
| `bug282-review-raw-evidence.tar.gz` | `67473082c7907466a7e22540acd06d51351dd6b1d8a9c053cd97b622426b3be2` |

The new archive contains review raw logs, exact commands/exit receipts, per-run
source manifests, final selected sources, scoped specification receipts and
verified byte-identical copies of selected prior receipts. Its internal
`artifact-files.sha256` verifies every archived artifact before packaging.
`bug282-review-final-hashes.sha256` separately covers this final report, findings,
raw archive, final selected sources and the preserved recovery receipt.

No original attachment is overwritten. BUG-280/281 remain Validating. The old
failed compaction session remains intact, with no recovery/resume/compaction
retry, token/history edits, protected history or credential reads. No extra
agent/session, model change, daemon restart, install, denied documentation or
standalone executable action, commit/push or security change. No pending denial
or approval is introduced. Bounded historical evidence still cannot establish
exclusive incident causation or live repair; this review claims neither.
