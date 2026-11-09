# BUG-284 independent pre-implementation review

This review supersedes the execution-count and continuation-omission findings
in the earlier test-plan audit. It does not approve production changes.

## Verified current evidence

The corrected `red-reviewed` target compiled and ran nine tests: **two passing
controls, seven behavioral failures**, exit 101. The failures comprise four
exact Codex lineage rejections (six/two supervisor originals, hook cancellation,
persistence cancellation), two cold/runtime replay disagreements (runtime lock
and provider setup), and one acknowledged pending input lost after child-process
death. The missing two-input continuation and duplicate six-input continuation
were corrected before this run.

Receipt: `/tmp/fspec-bug284/receipts/red-reviewed.log`.
SHA-256: `4a953a99f128d2499e67e272fe23624dd5394226769d8fd4fa5995338e074cdf`.

The five existing BUG-276 tests and three BUG-282 tests passed. The latter include
a genuine established-DAG negative control and repeated compaction/cold
continuation for canonical tool output. Formatting, scoped compilation and
strict scoped Clippy passed for the current BUG-284 tests. These are preparation
results, not evidence that any repair has been implemented.

An independent hash comparison found all **216** captured production files
unchanged. The deployment baseline remains PID 922084, version 0.10.9, executable
SHA-256 `d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`.
No original failure history was changed or used for live continuation.

## What this RED proves and what remains

The causal tests compose actual incoming-input queue handling, the task loop,
durable originals, genuine streamed Codex checkpoints and the selected-compaction
commit. They are materially stronger than isolated cancellation or tool-text
tests. They establish the replay divergence in the existing implementation.

They are not full GREEN acceptance coverage:

- Fresh checkpoint growth currently uses direct persistence plus
  `run_agent_stream`, rather than queued fresh admission. The production
  task-loop provider factory does not share the seed fixture's request observer.
  Its seed request count alone cannot prove zero canceled dispatches.
- The new historical test's `ancestor` mutation is ordinary user-message
  ancestry, not an established DAG. Actual DAG rejection is covered by the
  existing BUG-282 regression, and must remain covered after implementation.
- The combined setup/pre-compaction scenario currently exercises provider setup
  only. Its broad error assertion must be tightened and the second branch added
  before claiming full scenario coverage.
- Receipt API introduction will require replacing the current archive-retrieval
  helper and manifest-message-count barrier with receipt-aware recovery and a
  deterministic Pending boundary. Preserve recoverability and cancellation
  assertions; a timeout after the schema change is not meaningful GREEN/RED.
- The remaining acceptance matrix includes promotion/acknowledgment races,
  unresolved dispatch recovery, model-visible cancellation/failure disposition,
  receipt/blob GC, identity conflicts, all submission entrypoints, repeated
  automatic compactions and cold continuation with observable once-only effects.
  Scenario mappings are traceability, not evidence that these branches passed.

The attached `repair-plan-audit.md` also identifies the ordinary-input RPC's
discarded errors and missing external idempotency contract. Bound the promised
acknowledgment guarantee explicitly; do not expand into concurrent RPC ownership
without coordination. No automatic continuation may reinterpret an unresolved
old admitted action as fresh permission to repeat it.

## Exact approval boundary

The production change to `rust/cli/src/interactive/stream_loop.rs` requires a
focused mechanical decomposition before its initial request handoff can change.
The attached `cli-split-proposal.md` gives source ranges, destination modules and
behavior-preservation checks. This stage must be verified independently before
adding admission semantics. Stream cancellation, rollback, tool settlement and
retry order are the material behavioral risks.

The attempted native HITL approval was **denied, known not executed**. Automatic
approval review required explicit human approval and did not accept controller
authorization from the general repair request. No retry or indirect bypass has
been made. Production remains pending that approval, and BUG-284 stays Testing.
The approved scope still excludes install, restart, commit, push, session
closure, history deletion or migration, and any claim of live repair.
