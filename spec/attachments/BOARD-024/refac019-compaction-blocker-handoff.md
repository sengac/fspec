# REFAC-019 / BOARD-024: native compaction blocker

## Verified checkpoint, 2026-10-28

User authorization for the proper preparatory refactor is resolved. REFAC-019
was created for that scope; BOARD-024 remains the chooser/full-details feature.
Native session `4d8eedb6-fe1d-43cb-9aa9-70252fb8cd61` was verified using
`codex/gpt-6.1-sol` before resumption. It uses unchanged daemon36373, PID922084,
workspace `/home/rquast/projects/fspec`.

REFAC-019 reached canonical **implementing** at 19:23:54 UTC. BOARD-024 remains
**testing**, not Done. BUG-283 is canonical **done**. The native session is now
**Idle**, with no pending pause or HITL request and no committed compaction.

## Exact failure and action state

At 19:27:45 UTC the native turn terminated with `compaction_failed`. Bounded
transcript readback returned:

```text
terminal compaction failed: compaction commit failed: compaction candidate rejected: selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry
```

The failed action is committing a terminal-overflow compaction candidate for the
existing native session. Outcome: **rejected before commit**, not uncertain or a
pending user approval. Both saved `compaction` and `compaction_commit` are null.
Read-only source inspection shows terminal recovery blocks inference until durable
reconciliation. The provider rejects mismatched assistant/tool-result/DAG ancestry.
No reset, history edit, lineage weakening, model switch, daemon restart, new worker
session, or blind compaction retry was performed. No safe supported reconciliation
for this specific mismatch was established within the approved six-module scope.

Continuation requires resolution of the native checkpoint/compaction failure,
or a separately authorized supported recovery/repair plan. Do not treat the prior
proper-refactor approval as permission to bypass lineage checks.

## Completed and preserved work

- REFAC-019 Markdown design and AST/review documentation are registered.
- Six executable scenarios are mapped across three features, each with exactly
  one test source file. They retain the original acceptance criteria. Fspec initially
  rejected one feature mapping to three files; specifications/mappings were split
  through supported commands and the transition then succeeded through the normal
  one-request approval. No gate was disabled.
- Original behavior baseline: **145 tests passed across 11 targets, zero failures
  or ignored tests**, Cargo exit 0. Includes embedded/WS, trait surface, session
  intent, Mux, attachment, isolated/shift-right creation, modal/help/click regressions.
- Six original ordered contract snapshots are frozen: 234 Action variants,
  backend declarations/defaults and both implementations, and 123 RPC declaration
  and implementation methods. Snapshot hashes are attached and remain unchanged.
- Latest refactor suite: **4 passed, 2 expected failures**, exit 101. Remaining
  causal RED is oversized integration sources and schema fingerprints ignoring
  nested declaration inputs. Ordered contracts, backend compatibility, MCP exact
  membership and resolver adversarial cases pass.
- Independent review caught discarded cfg/cfg_attr attributes in the new test
  source resolver. A causal failing fixture was run first; the resolver now rejects
  unsupported attributes on includes/composition definitions/entries while keeping
  ordinary item attributes. Raw RED and corrected-suite evidence are attached.
- BOARD-024's prior **1 pass/16 fail** chooser RED and all 16 attachments remain.
  Deeper chooser-dependent assertions are still unreached; no feature GREEN claim.

## Production and preservation limits

No refactor extraction or BOARD-024 production implementation has been applied.
The six original integration entrypoints and rpc/build.rs match the independent
pre-resumption copies byte-for-byte. Only refactor specifications, test support,
test dependencies/lockfile, contract snapshots and evidence were added during this
resumption. All 22 protected BUG-283/navigation/test/mapping hashes still match.
No install, commit or push occurred.

Independent read-only baseline copies are under
`/tmp/board024-controller-baseline/`, including `protected.json` and six snapshots.
Do not rewrite these to accept later differences.

## Resume checks after safe recovery

Recheck daemon/session/model/card state and existing process outcomes before any
new work. Preserve all current changes. Do not repeat the completed specification,
baseline capture or test-helper work. Finish REFAC-019 decomposition and unchanged
contract/behavior checks before BOARD-024 implementation. Preserve original RPC
method order, include real nested schema inputs and migrate positive and negative
source-shape checks honestly. Keep all touched maintained sources below 300 lines.

BOARD-024 still needs its deferred canonical fixture corrections, actual typed
response/generation assertions, all deeper details/scroll/link/attachment behavior,
causal RED/GREEN, regression/quality checks, review, mappings, Markdown evidence and
canonical Done. Its multi-file test layout may need the same proper feature split
required by Fspec's one-feature/one-test-file gate; do not weaken it.
