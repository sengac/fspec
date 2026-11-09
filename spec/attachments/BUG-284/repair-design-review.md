# BUG-284 independent repair design review

Read-only review; no repository edits, builds or session mutations. This is a
design contract and test review, not an implementation or passing-test claim.

## Critical contract (publish early for the implementation owner)

Define **admission** as a local, durable commitment that this exact input is now
part of the conversation. It does not mean the remote model has seen it or that
the request completed. Once admitted, cancellation must not remove the input
from runtime, persisted replay or compaction history, and must not requeue it.

Use a stable input UUID plus immutable canonical content/origin identity and a
monotone durable disposition: `Pending -> Cancelled` or `Pending -> Admitted`.
No transition from Admitted back to Cancelled; execution cancellation is a
separate truthful terminal event. Pending/Cancelled input remains recoverable
as audit evidence but never enters selected conversational replay. Legacy
messages without the new versioned admission marker keep existing semantics;
never guess that an old supervisor-looking string was canceled.

**Linearization point:** a serialized admission gate owns (a) final cancellation
epoch decision, (b) durable promotion to Admitted, and (c) installation of the
same exact message into runtime. If cancellation wins before promotion, record
Cancelled and do not install or dispatch. If durable promotion wins, installation
must complete even if interruption arrives immediately afterward. Then stop
dispatch if canceled, preserving the admitted history. There must be no await or
cancellation-select that can abandon the admitted-but-not-installed runtime.
If blocking persistence is offloaded, keep the same gate/ownership while
settling its definitive outcome; do not release ownership merely because the
awaiting future was canceled.

The provider request must use the pre-append history slice plus the prompt once.
Current `cli/interactive/stream_loop.rs:482–494` deliberately clones history
before appending the prompt. Appending earlier without changing that precise
handoff duplicates the prompt. Use an explicit admitted-input identity/receipt;
never deduplicate by matching text (two intentional identical prompts are valid).

**Crash/uncertain outcome:** durable receipt/manifest is authority. A Pending
receipt does not auto-execute after restore. A durable Admitted receipt restores
the exact original into history once, even if its provider dispatch never began;
resume does not replay the old prompt as fresh input. A lost acknowledgment is
not refusal: read back the exact UUID/content/state under ownership before any
dispatch or cancellation classification. If authority cannot be resolved, block
inference with recoverable evidence; do not drop, retry with a fresh UUID, or
convert it to Cancelled.

Moving the existing persistence call later alone is insufficient. Marking
admitted at `run_selected_provider` entry is also insufficient: provider setup,
pre-prompt compaction and the pre-stream paths can fail before runtime append.
The owner must connect admission to actual runtime installation, and ensure
pre-prompt compaction sees Pending input according to the same contract.

## Storage API constraints already verified

- `core/persistence/manifest_updates.rs:40–52` is the supported single-owner
  manifest transaction. `manifest_commit.rs:105–122` rereads under a writer lease
  and promotes a revised snapshot; do not nest other session access inside its
  callback (`manifest_updates.rs:13–24`).
- `conversation_commit.rs:6–31` stores an immutable message by UUID and adds its
  reference idempotently in that transaction. It currently unconditionally adds
  normal originals; use a narrow typed admission API instead of passing a fake
  role or text prefix to hide canceled inputs.
- `ManifestCommitError::Promoted` (`manifest_commit.rs:38–43`) means rename
  succeeded but durability acknowledgment failed. Preserve identity and settle
  it explicitly. Flattening this to a generic error and retrying with a fresh
  UUID duplicates history and can grant wrong execution authority.
- `agent-loop/persistence_retry.rs:227–260` retries archival submission; its
  `RecoveryRecord` has fixed ID and content. `persistence_retry_commit.rs:72–79`
  currently flattens append errors to strings. A new admission path must retain
  the distinction between refused and promoted/uncertain outcomes.
- Stored originals are immutable: do not mutate existing envelope bytes to
  update disposition. A versioned manifest-owned disposition table bound to
  exact input UUID/content is a reasonable small extension. If instead using
  a separate receipt store, define a single authoritative commit and exact
  cross-store recovery order; do not claim two files form an atomic transaction.
- Keeping recoverable originals in `manifest.messages` avoids a new GC root,
  but then all replay/compaction readers must use one validated admitted-input
  projection. A separate receipt list requires adding it to GC roots:
  `message_cleanup.rs:50–68` currently retains only `manifest.messages`.
- A mutable disposition affects which originals are conversational. It must be
  included in/revalidated by the source authority used for compaction; existing
  prefix checks mostly bind message UUIDs. Do not allow a captured candidate to
  change projection through a later disposition edit while preserving UUIDs.

## Minimal surfaces and size constraints

Prefer small new typed modules for admission state/transitions and runtime
installation. Existing likely integration points:

| Surface | Why | Current lines |
| --- | --- | ---: |
| `core/persistence/manifest_types.rs` | versioned durable disposition identity | 152 |
| `core/persistence/conversation_commit.rs` | atomic immutable input storage/admission | 32 |
| `core/persistence/session_replay.rs` | common model-history projection | 76 |
| `core/persistence/compaction_boundary.rs` | stable/rechecked projection authority | 193 |
| `agent-loop/user_persistence.rs` | received-input receipt creation | 26 |
| `agent-loop/agent_loop/prompt.rs` | cancellation and admission ownership | 164 |
| `agent-loop/agent_loop.rs` | carry receipt through dispatch | 254 |
| `sessions/background_session/interruption.rs` | serialized cancel/admit gate | 83 |
| `cli/session/mod.rs` | optional staged-input field/module wiring | 298 |
| `cli/interactive/stream_loop.rs` | actual initial runtime/request handoff | 2,564 |

The last two are size hazards. Do not casually add lines to the 298-line Session
module or embed more policy in the large stream loop. If that handoff needs
editing, make an explicit mechanical extraction plan before behavior changes,
preserving existing request/prompt ordering. Avoid TUI/RPC/test-helpers/Cargo.lock
overlap and avoid using this fix to rewrite unrelated provider state machinery.

## Essential regression matrix

1. Real-failure-derived canceled supervisor input, followed by the controller
   input: exact production selected-history replay must agree, strict ancestry
   still rejects unrelated mutations, and failed historical files remain intact.
2. Supervisor before interrupt, supervisor during interruption, regular queued
   prompt before interruption; each remains recoverable but absent conversation
   when cancellation wins before admission. A later fresh input runs once.
3. Interrupt while persistence is held, after persistence but before admission,
   while admission promotion is held, and immediately after promotion but before
   dispatch. Assert the winning disposition and exact runtime/durable equality.
4. Provider construction error and pre-prompt compaction failure before dispatch:
   no durable/runtime divergence and no accidental fresh retry of canceled input.
5. Real Codex loopback continuation, two successive compactions and cold restore
   after the canceled/admitted patterns, with all preexisting side effects once.
6. Child-process death at Pending receipt, admitted-original promotion, runtime
   installation, and provider attempt start. Cold recovery must never infer
   remote completion or auto-execute canceled/pending input.
7. Lost acknowledgment after manifest rename: same UUID readback settles exactly
   once; unresolved storage blocks inference. Test actual rename/sync failpoints.
8. Duplicate receipt retry, conflicting content under same UUID, unknown state
   version, dangling disposition, and illegal downgrade all fail closed.
9. Identical prompt text with distinct UUIDs remains two inputs; a repeated UUID
   remains one input. Include images/blob-backed user content.
10. Pending/Cancelled disposition survives originals retrieval and compaction
    without turning canceled instructions into actionable DAG content. Existing
    admitted originals and every strict-lineage negative control remain intact.

BUG-276's stub tests must retain their safety intent, but their historical
“archived original” assertion cannot alone define model-history membership.
Add real Codex composition coverage instead of deleting cancellation assertions.

## Follow-up: index domains and truthful cancellation (source rechecked)

The smallest sound storage shape is likely to keep **only admitted conversation
originals in `manifest.messages`**, and retain Pending/Cancelled receipt UUIDs in
a separate manifest-owned collection rooted for GC. Admission can append the
exact stored original reference and transition its receipt in the same manifest
transaction. This preserves the established message-index domain. It still
requires receipt validation, replay audit access, GC roots and fork policy; it
does not require a separate file to be treated as transactional authority.

Keeping all receipts in `manifest.messages` and filtering only model replay is
materially broader than one replay fix. Exact current coupling is:

- `provider_state/types.rs:54–63,181–195` hashes raw `MessageRef` slices and checks
  those anchors; changing a separate disposition without changing references
  does not change this hash. It must not retrospectively change a checkpoint's
  established conversation under the same anchor.
- `provider_state/replay.rs:12–15` compares checkpoint `prefix_len` with the raw
  compaction boundary. `provider_state/fork.rs:27–38` requires an exact checkpoint
  at `child.messages.len()` and reanchors using that same count.
- `compaction_boundary.rs:115–119,151–164` and
  `compaction_commit.rs:93–102,118–122,145–148` bind UUID-prefix length, DAG
  coverage and `compacted_before_index` to that same raw domain.
- `manifest_messages.rs:79–93` skips raw references by the boundary;
  `originals.rs:28–52,73–78` exposes raw turn indices to original retrieval.
- `session_replay.rs:12–17` validates committed boundaries/provider metadata
  before loading conversation. Filtering in one downstream consumer therefore
  cannot redefine those upstream authorities.

If the all-receipts-in-messages design is chosen, specify raw archival indices
versus admitted conversational indices explicitly and test every mapping. Bind
the validated immutable projection to each source/checkpoint. Never allow a
later metadata edit to reclassify an established original. Pending entries at a
captured prefix cannot later acquire conversation membership behind that prefix.

An admitted request canceled before provider dispatch needs a truthful durable
**execution disposition**, not just a hidden receipt flag: a later model must
not see that admitted request as an unexplained unfinished instruction. Preserve
the original, retain the terminal cancellation evidence, and use a supported,
canonical local control event or equivalent authoritative projection that is
installed identically in runtime, cold restore and compaction. Do not fabricate
an assistant response or completion. Existing `_transcript` decoding is only UI
chunks (`sessions/replay_content.rs:53–60`), so a transcript terminal alone does
not communicate cancellation to subsequent model history.

If a crash leaves Admitted without a definitive execution terminal, retain that
state as unresolved; do not infer remote execution, completion or cancellation.
Reconcile it before future inference and do not automatically redispatch the old
input. Pending on crash likewise stays Pending until explicit recovery policy
settles it. Add separate tests for zero-provider-call cancellation after
admission, cancellation after an uncertain send, and the next fresh request
after cold restore: check the exact control disposition as well as input count.
