# BUG-284 — recurring compaction failures: evidence-backed analysis

## Findings

The latest two live failures are caused by a durable/runtime conversation split:
canceled supervisor inputs were preserved as ordinary conversational originals,
but were never included in the provider checkpoint. Subsequent compaction
reconstructs those originals and the strict lineage validator correctly refuses
the changed order. The error says “assistant, tool result or DAG ancestry” even
when the first mismatch is a **user** message.

This is not another instance of BUG-282's output decoding defect. All 441 saved
tool outcomes in the two latest checkpoints match their rehydrated originals.
Their first differences are six and two additional supervisor originals.
The actual production `CodexHistory::prepare` API reproduces both failures from
preserved real histories. In cloned diagnostic requests only, omitting those
exact extra inputs makes validation pass. Altering a tool outcome still fails.
No live history was filtered, reset, repaired or replayed into the daemon.

The older failures contained overlapping defects. Both had tool outputs changed
by the exact pre-BUG-282 quote/backslash transform. The first also contained nine
additional canceled user originals. The prior encoding-focused investigation
found a real defect but did not discover this second divergence behind it.

## How many fixes and failures

For the scoped BUG-278/BUG-282 history: **two production repair efforts**, plus
**one substantive BUG-282 test/review correction**, and **at least five documented
live failures across five sessions**. This is not a claim that the entire
repository has only two compaction-related repairs. Adjacent BUG-276 cancellation
behavior is directly relevant to this newly isolated cause.

| UTC, 2026-10-28 | Session | Established evidence |
| --- | --- | --- |
| 02:29:30 | `ba15d5a4…` | Earlier macOS generation-advancement failure, preserved BUG-278 receipts |
| 08:40:33 | `7c55a508-ff74-4a68-895f-c01e9bdbb03c` | 233 altered outcomes; nine extra post-cancellation user originals |
| 10:06:14 | `c09c6649-9724-42d6-8aa7-005a885b8d27` | 165 altered outcomes; outcome correction alone clears isolated replay |
| 19:27:45 | `4d8eedb6-fe1d-43cb-9aa9-70252fb8cd61` | Six extra canceled supervisor originals; tool outcomes match |
| 20:22:53 | `7b41d57c-d7aa-47f3-b2c7-b689012703c4` | Two extra canceled supervisor originals; tool outcomes match |

Eight matching daemon diagnostic lines represent four Linux incidents, not eight.
The last terminal is classified `provider_error` despite the precise compaction
lineage diagnostic; terminal-label-only counting undercounts recurrence. BUG-282's
two Done transitions do not represent two different production repairs.

## Runtime provenance

Current daemon: PID `922084`, port `36373`, Fspec `0.10.9`, registered start
`2026-10-28T12:07:57Z`. `/proc/922084/exe` resolves to
`/home/rquast/.local/bin/fspec`; it matches the installed and retained release
binary with SHA256:

`d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`

ELF build ID: `7c55e2fe77fcff2023c744d8556699919769c6c4`.
Matching Cargo dependency fingerprints and inspected compiled LLVM bitcode show
BUG-278 generation binding and BUG-282 canonical output preservation in the
retained build. The later failures must not be dismissed as use of the earlier
explicitly unpatched PID `366915`.

Current source HEAD is `626eee1ec96a8e3ec616b5e6411a5c1596815709`; BUG-278 is in
ancestor `042d3974`. The final executable is stripped, lacks source attestation,
and was built before the HEAD commit was recorded. Its exact whole source tree
cannot be certified as clean HEAD from these facts. The detailed provenance
attachment separates binary identity, compiled-code evidence and this limit.

## First exact divergence and causal path

For `4d8eedb6…`, the six extra originals occupy zero-based manifest indices
391–396, persisted at 13:23:43.247826–.284359 UTC, immediately after the
13:23:43.240094 `user_cancelled` terminal. Provider projection index 490 expects
the later controller input. Reconstructed durable history instead presents the
first canceled supervisor input.

For `7b41d57c…`, extra originals are manifest indices 291–292, persisted at
20:15:05.034036 and 20:15:38.436745 UTC. The `user_cancelled` terminal occurred
at 20:15:05.027928. Provider projection index 425 expects the subsequent accepted
controller input; durable replay presents the earlier supervisor input.

Exact message IDs, timestamps, SHA256s and projected-item hashes are in the
private preserved evidence and shareable metadata receipts. No prompt text or
credentials are needed in this report.

Source trace, relative to the repository:

1. Direct incoming-message RPC queues supervisor input with an interruption
   epoch (`sessions/.../messaging.rs:30–40`). Its route differs from the
   AgentManager idle-only gate.
2. `agent-loop/src/agent_loop.rs:45–72` marks a canceled submission/old epoch;
   supervisor inputs do not clear interruption as fresh regular user input does.
3. **`agent_loop/prompt.rs:126–132` persists the input first.** It is an ordinary
   user original without a durable admitted/canceled disposition.
4. **`prompt.rs:149–154` then checks cancellation and skips execution.** Other
   cancellation checks after locking and before dispatch can create similar
   windows. Runtime/provider history therefore need not contain the original.
5. `core/src/persistence/manifest_messages.rs:54–95` and
   `cli/src/session/compaction.rs:195–207` reconstruct the manifest's
   conversational originals; `selected_compaction.rs:43–66` validates this view.
6. `providers/src/codex/history_lineage.rs:63–91` detects the extra user entry
   before a later checkpointed turn and refuses the candidate.

Normal active turns can continue because they use coherent in-memory history;
the discrepancy surfaces when compaction or restore uses the durable view.
Fresh sessions avoided an already-divergent history temporarily, but the same
controller interrupt/queued-followup pattern recreated the problem. The failed
sessions must stay intact. This evidence does not establish which of the narrow
post-persistence cancellation checks fired for every individual input; the
durable/runtime split and its causal effect on all four Linux replays are proven.

## Actual production-code replay

`replay_production.rs` links the existing ci-test provider library and invokes
the unmodified `CodexHistory::restore`/`prepare` API. It performs no network calls,
tool execution, daemon operations or writes to original session data. Requests
are reconstructed from real ordered originals, with SHA256-checked blob content,
following the text/tool serialization subset observed in these sessions.

| Session | Original replay | Diagnostic clone only |
| --- | --- | --- |
| `4d8eedb6…` | Exact lineage error | Omit six unmatched supervisors: accepted; tampered outcome: rejected |
| `7b41d57c…` | Exact lineage error | Omit two unmatched supervisors: accepted; tampered outcome: rejected |
| `7c55a508…` | Exact lineage error | Restore 233 canonical outputs: still rejected; also omit nine unmatched users: accepted |
| `c09c6649…` | Exact lineage error | Restore 165 canonical outputs: accepted |

All 398 changed earlier outputs exactly match the old stripping/unescaping
sequence. These cloned counterfactuals isolate causal differences; they are not
a proposed history-editing repair. Full raw evidence remains local in protected
directories. Final replay exited zero; receipt and source/artifact hashes are
attached. Initial harness setup/compiler/schema mistakes were corrected and
are not counted as product failures or causal RED evidence.

## Why prior verification missed it

BUG-278 addressed provider/durable generation domains. BUG-282 addressed
destructive second decoding of canonical outcomes. Both were real defects.
BUG-282's first review additionally overstated four negative tests; its reopened
review fixed the tests without introducing another production repair.

The BUG-282 fixture meaningfully exercises streaming/tool persistence, two
compactions and cold runtime reconstruction, but bypasses the native task-loop
input channels and does not drive queued input/cancellation. Other tests cover
parallel lineage and cancellation separately. They do not compose them with
native persistence and subsequent automatic compaction.

BUG-276 tests explicitly assert canceled input absent from runtime yet present
in manifest originals. They use a stub provider and never follow that state with
Codex lineage validation. The combination creates the missing regression case.
The first old failure already held both defects, so a single encoding reproduction
was insufficient to classify the whole real incident. Prior Done status and unit
success did not establish that the installed/live recurrence was resolved.

## Proposed production repair — not implemented

Introduce an explicit durable input-disposition contract. Preserve every
submission for recovery/audit, but publish only admitted conversational inputs
into the provider-consumed ordered history. Runtime construction, cold restore
and compaction must use that same authoritative projection. Admission and
cancellation races need an atomic/transactional boundary with truthful receipts.
This must handle regular user input as well as supervisor input.

Do not fix this by deleting originals, filtering every supervisor by string
prefix, injecting canceled text retroactively into checkpoints, skipping
arbitrary user mismatches, or weakening lineage. Historical recovery requires a
separately reviewed evidence-backed reconciliation path; records currently lack
a durable disposition, so silent migration would be speculative.

Before implementing, specify and causally reproduce: supervisor arrival while
interrupted; queued-before-interrupt; cancellation during persistence; normal
user cancellation; fresh subsequent input; parallel calls; first and successive
automatic compactions; cold process restore; and unchanged once-only effect
receipts. Keep missing/tampered outcome, invocation, assistant and DAG refusals.

Before claiming live fixed, authorize installation separately, verify the exact
loaded executable against a build-time source manifest, then perform repeated
realistic compaction/restore/continuation with interruptions and queued input.
Neither installation nor live validation occurred in this investigation.

## Card and evidence state

BUG-284 remains **Specifying**, not Done. It has documented rules, concrete
failure examples and attachments. The remaining work is repair specification,
implementation authorization, historical-recovery design and live verification;
exact whole-source build attestation is also unavailable for this existing binary.

Native MCP has no direct card-create operation, so the supported Fspec CLI was
used and native MCP readback confirmed the card. No native session was created
or reused. Completed BUG-283 session `22032f8b…` is Idle, `codex/gpt-6.1-sol`,
has no pending input/goal and continuation disabled; its last terminal remains
completed at 13:35:33.459679 UTC. It was not touched.

Production source, failed histories, checkpoints and active REFAC-019/BOARD-024
sessions remain untouched. No restart, install, credential change, commit or push.
