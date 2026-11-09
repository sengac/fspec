# Prior compaction repair audit: BUG-278 and BUG-282

Audit performed read-only against `/home/rquast/projects/fspec`, HEAD
`626eee1ec96a8e3ec616b5e6411a5c1596815709`. No build, daemon mutation, source edit,
session replay or live-repair attempt was performed. Paths below are relative to
that repository unless absolute. This report audits prior repair evidence; a
separate investigation owns current failed-session causation and runtime proof.

## Findings first

The requested history establishes **two distinct production repair efforts**:
BUG-278 fixes generation-domain conflation; BUG-282 removes destructive second
decoding of canonical textual tool outcomes. BUG-282 was subsequently reopened
for a substantive test/specification correction; that is a review iteration,
**not a third production fix**. Multiple fixture corrections and failed build or
test attempts also must not be counted as independent production repairs.

These two cards are not an exhaustive count of every compaction-related repair
in repository history. Earlier related cards have other failure modes. A count
of observed incidents must be assembled independently from actual failure
receipts, not inferred from card numbers or repeated appearances of one error.

Neither card's final technical evidence claimed an installed/live repair. Both
explicitly preserve that limitation. Treating either Done status as proof that
subsequent daemon sessions would work exceeded what those records established.

## BUG-278: generation domains

**Proven source defect and fix.** Selecting a model before a durable compaction
calls `reset_provider_state` (`rust/sessions/src/model_switch.rs:76–100`), which
can advance provider generation independently of durable generation. The old
caller supplied durable generation + 1 to an already-advanced provider and
triggered `generation must advance with a selected model`.

Current `rust/providers/src/codex/compaction.rs:18–79` validates the selected
checkpoint against committed replay, computes provider generation + 1, and
binds it to the separately supplied durable generation through `_codexCompaction`
metadata plus the SHA256 of the source generation/checkpoint. Lines 82–126
validate that binding on restore. `rust/cli/src/session/compaction.rs:54–85`
retains durable generation + 1 and invokes the selected-history check;
lines 107–115 repeat provider preparation under the manifest transaction.
The strict advancing-generation guard remains intact.

**Commit/source provenance.** Current repository history introduces the BUG-278
test and associated fix in `042d3974b4dbdf208fa006d7474edd504fd4ea5f`
(`fix: harden sessions and Codex device authorization`, 2026-10-28 06:23:52 UTC).
The original Mac controller receipt names a different originating history:
`9b490e8917e8928cc87ab3ca6bad920bf7afbc39`, observed after verification at
2026-10-28 04:17 UTC. All **21** `owned_source_sha256` entries in
`spec/attachments/BUG-278/bug278-controller-verification.json` match current
source bytes. Thus the validated source is present here; this does not prove
which executable a later daemon loaded.

**Recorded checks.** The final receipt reports 105 top-level tests: 7 acceptance,
33 CLI, 24 provider, 13 core, 11 fresh-process and 17 session cases. These are
historical recorded runs, not tests rerun during this audit. The executed RED
receipts distinguish the original generation error from a separate missing
binding safety RED. A failed compilation was explicitly excluded as RED.

**What the primary acceptance fixture actually exercises.**
`rust/cli/tests/bug278_codex_compaction_lineage.rs:12–92` executes two durable
promotions, publication, fresh-runtime restoration, genuine temporary effect
preservation and loopback continuation. Its setup creates synthetic messages,
explicitly commits checkpoints and directly appends call/result metadata
(`rust/cli/tests/fixtures/bug278.rs:140–228`); it does not reproduce the full
native streaming/persistence accumulation that later BUG-282 implicated.
Additional regressions did cover actual child processes, as recorded in the
controller receipt, but the two-compaction positive fixture remains deterministic
integration evidence, not a replay of the failed historical session.

**Explicit limitation in the existing record.**
`bug278-controller-verification.json.native_session_final_observation` records
one native compaction with aligned generations on the unchanged daemon and
states that it is not live execution of the newly built fix. The final Markdown
also states no installed-binary replacement/restart or historical-session repair.

## BUG-282: canonical textual tool outcomes

**Proven source defect and fix.**
`rust/cli/src/interactive/stream_handlers.rs:182–201` keeps the original rig
ToolResult in runtime messages and now emits its text unchanged (joining text
parts). Before `626eee1e`, it additionally stripped outer quotes and replaced
literal escapes. Native BackgroundOutput persisted the altered event text while
checkpoint/runtime retained canonical text; selected-history validation could
therefore reject the durable replay. The commit diff verifies the only intended
production behavior change is removing that second decoding. The existing unit
module was also mechanically extracted.

Current `rust/cli/src/session/selected_compaction.rs:43–66` reconstructs durable
messages and validates against the selected preparer. The generic failure at
`rust/providers/src/codex/history_lineage.rs:63–91` can represent an assistant,
tool-result or DAG mismatch. Reproducing that error with a synthetic encoding
fixture did not establish that every real occurrence had that encoding cause.

**Commit/source provenance.** The behavior fix and corrected test set are in
current HEAD `626eee1ec96a8e3ec616b5e6411a5c1596815709` (2026-10-28 11:51:15 UTC).
The three current BUG-282 test/helper hashes match the review's final hashes.
Targeted `git diff HEAD` for the audited production and BUG-282 test files is
empty. Running executable identity is outside this audit.

**One production repair, then a required review.**
`spec/attachments/BUG-282/bug282-review-findings.md:5–35` explicitly says the
first-pass review was incomplete despite Done. Its old “missing outcome” case
added invented assistant text; “invocation” changed an outcome ID rather than
the invocation; “DAG” did not alter an established ancestor; JSON-string outer
quotes lacked independent coverage. The review corrected these tests and
reopened the workflow, with no new production fix or production RED claimed.

**Historical receipts verified in this audit.**

| Artifact | SHA256 |
| --- | --- |
| `spec/attachments/BUG-282/bug282-findings.md` | `603a5b439ba0643d15f3057e18cc29e4b62866c3ee0e81b03d25e2c83f669a12` |
| `spec/attachments/BUG-282/bug282-review-findings.md` | `cc8e21e966d918de05db120efd1e7ec7279a0723f638d9a9610cd58a94daf08c` |
| Original raw archive | `044cbf8c0d5680e63f825a31b2e0ebf6c14b79385dbb0eb37cccf89d6da64e50` |
| Review raw archive | `67473082c7907466a7e22540acd06d51351dd6b1d8a9c053cd97b622426b3be2` |
| `/tmp/fspec-bug282-evidence/accepted-red.log` | `f6c2947dff78696c5804e281ef8bd1e2bf81f8e0930ca0bcc3a074183fc25af1` |
| `/tmp/fspec-bug282-review-evidence/corrected-acceptance-exact.log` | `4c0d628e8daaa2bb9e53ce5351e4c57299b74c4e2ccfec8e3f823bb8a2a76381` |

The accepted RED log contains 1 pass and 2 actual failures (event-byte divergence
and selected-compaction ancestry). Corrected review acceptance contains 3 passes,
0 failures and all seven named negative refusals. The first pass recorded 81
distinct tests; the review freshly reran the 3 revised acceptance tests while
retaining the 78 unchanged regression receipts. Reporting another fresh 81-test
run would be wrong.

## Coverage strengths and remaining realistic gaps

- The current BUG-282 positive fixture is meaningful integration testing:
  actual rig tool dispatch, once-only filesystem effects, BackgroundOutput,
  durable originals, provider checkpoints, real loopback HTTP, selected admission
  and publication. It is not merely a mocked success assertion.
- `bug282_canonical_tool_outcomes.rs:78–164` covers **two** successive manual
  compactions and cold runtime reconstruction. `support/bug282.rs:223–235`
  rebuilds the runtime in the same process. It does not restart the daemon.
- `support/bug282.rs:159–174` scripts one function call followed by one assistant
  final answer per cycle, using `response.completed` events. It does not exercise
  fragmented multi-item streaming across native persistence and compaction.
- Its input-channel receiver is discarded (`support/bug282.rs:118`) and it calls
  `run_agent_stream` directly (`:199–210`); it does not test queued user turns or
  the production session-manager task loop. Interruption flags are never driven.
- The automatic terminal-overflow trigger, compactor/DAG builder, worker overlap,
  large real histories, timing of asynchronous persistence, and live transport
  interruptions are not part of those BUG-282 acceptance tests. Summaries are
  provided by the fixture before direct `commit_selected_compaction` calls.
- Prior regressions **do include** parallel-lineage and commentary/cancellation
  controls. `/tmp/fspec-bug282-evidence/provider-regressions.log:62–79` records
  3 `codex_parallel_tool_lineage` and 4 `codex_streamed_lineage_roundtrip` passes.
  The parallel fixture seeds grouped checkpoint calls/results and checks
  interleaved projections; it does not execute concurrent native tools followed
  by automatic compaction. The commentary test emits deltas and calls a rig
  agent; its cancellation case stops after a tool result, but neither traverses
  the full native durable-output/automatic-compaction/restore combination.
- Thus “parallel results or interruption have no tests” is false. The actual gap
  is their **composition with native persistence and repeated automatic
  compaction**, and the lack of a replay sourced from the real failed checkpoint.
- Images/multipart tool outputs were explicitly out of BUG-282 scope
  (`bug282-findings.md:136–144`). Their correctness is unknown here.

## Required evidence before a future live-repair claim

1. Match the actual loaded executable/build to the relevant source fixes, not
   merely version `0.10.9`, PID changes or a passing source build.
2. Identify the first differing projected item in real failed checkpoint versus
   reconstructed selected history; trace its original streamed event and durable
   persistence. Preserve exact raw evidence locally with hashes and redact only
   separate shareable derivatives.
3. Reproduce that real divergence in an isolated replay; distinguish the old
   BUG-282 encoding defect from assistant aggregation, ordering, missing results,
   restore or other causes. Keep all strict rejection controls.
4. Test the causal repair against realistic combined paths implicated by that
   evidence, including repeated compaction/restore/continuation and relevant
   parallel, queue or interruption timing. Verify side effects are not repeated.
5. Only after authorized installation and exact loaded-build verification can
   repeated live end-to-end results support “live fixed.” No such installation,
   restart or live validation is authorized or claimed by this audit.
