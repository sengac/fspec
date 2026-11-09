# BUG-284 durable input admission design

Status: proposed repair, not implemented or deployed. The CLI decomposition
below requires explicit human approval. That approval was requested and remains
pending; production files, including indirect replacements for the handoff,
must remain untouched while the proposal and causal RED are prepared.

## Authority and retention

An input submission is not automatically a conversational message. Give every
submission a stable UUID before acknowledging queue acceptance. Preserve its
exact canonical envelope, image/blob references, origin (regular, supervisor,
MCP, or internal continuation), submission epoch and timestamp. Two submissions
with identical text have different UUIDs. A retry retains its original UUID.

Proposed manifest extension: a versioned ordered `input_receipts` collection,
whose entries bind the input UUID, immutable original content hash, origin,
epoch and disposition. Disposition is monotone: Pending -> Cancelled or
Pending -> Admitted. A separate execution outcome records interruption after
admission; it does not downgrade the admitted disposition or claim that the
remote provider received/completed a request.

Keep receipt originals outside `manifest.messages` until admitted. This keeps
existing conversation indices, DAG coverage and provider lineage restricted to
the admitted conversation; canceled instructions cannot leak through full-history
compactor retrieval. Add receipt references to message GC roots. Expose a typed
receipt retrieval API with complete original/blob evidence and disposition,
without changing existing conversational turn indices or silently filtering old
messages. Preserve successful and canceled submission receipts indefinitely
under the same explicit session deletion semantics as conversation originals.

Store immutable originals before committing their receipt reference. A durable
recovery-spool record with the same UUID protects failures before reference
promotion. Receipt creation and message promotion use the supported single-owner
manifest transaction. No nested session access inside its callback. Idempotence
checks exact content/identity; conflicting UUID reuse is refused.

Submission acknowledgment must follow durable Pending retention, not volatile
queue insertion. Queue rejection leaves a truthful retained, non-admitted
receipt. MCP and internal retry paths need the same receipt semantics, not a
supervisor-text heuristic. This changes the queue front doors as well as the
consumer, and must be covered by submission/crash tests.

## Serialized linearization and runtime installation

1. Retain Pending, run allowed prompt hooks, and acquire the runtime lock before
   final admission. Hook refusal is retained with an explicit refusal reason.
2. Under a shared cancellation/admission ownership gate, compare the input's
   epoch and cancellation-at-submission flag. If cancellation won, commit
   Cancelled. Do not install it, infer, compact it, or execute tools.
3. If admission wins, atomically promote its receipt and append its original UUID
   to `manifest.messages` once. Install that exact message into runtime before
   releasing ownership. There is no cancellable await between acknowledged
   promotion and installation. Use blocking I/O off the async executor while a
   dedicated owned worker settles this boundary; abandoning an awaiting caller
   must not abandon the worker's definitive result or leave live inference
   enabled on an unresolved runtime.
4. Cancellation after that boundary stops dispatch/execution, not membership.
   Runtime, cold replay and compaction retain the admitted message once.
5. Provider construction and pre-prompt compaction errors preserve membership.
   The next fresh input continues from it; recovery never requeues an admitted
   input as a fresh submission. Remote completion is not inferred from local
   admission or a provider-attempt receipt.

The CLI must explicitly receive the admitted-input identity. Its first request
uses the pre-append history plus this prompt once, without removing the admitted
message from authoritative runtime. The current stream handoff clones history
then appends the prompt; simply appending earlier duplicates it. A dedicated
request-history view must exclude the exact admitted identity, not matching
text. Pre-prompt compaction consumes the admitted conversation and must bind
the continuation to its committed summary rather than reinsert the original.
Synthetic continuation prompts remain explicit internal inputs, not a license
to delete or duplicate genuine submissions.

## Durability, uncertainty and crash recovery

`ManifestCommitError::Promoted` is not a refusal. Keep UUID and exact identity,
read back under ownership, retry the durability barrier, and settle that same
receipt. Dispatch only after authoritative acknowledged admission and runtime
installation. If acknowledgment cannot be resolved, block inference with the
recoverable receipt/spool evidence intact. Never retry with a fresh UUID.

After process death: Pending and Cancelled remain audit-only and never execute
automatically. Admitted restores once into conversation even if no provider
attempt started. A fresh user instruction may continue it, but restore must not
replay effects. Child-process and real sync/rename failpoint tests must prove
this at Pending storage, original promotion, runtime installation and dispatch.

Validate receipt version, identity, original/hash/blob evidence and consistent
membership on cold replay and compaction source capture/commit. Admitted entries
must occur once; non-admitted entries must not occur in conversation. Admission
appends membership; it never changes the meaning of a captured prefix. Preserve
all generation, provider projection, original-byte, DAG ancestry and tool-pair
guards. Unknown versions and contradictory/missing references refuse inference.

## Legacy recovery

Absent receipt schema means existing legacy semantics, not permission to infer
cancellation from text. Existing divergent sessions remain fail-closed with all
originals and provider checkpoints intact. Provide an actionable diagnostic:
explicit evidence-preserving recovery is required; no model request or automatic
tool replay is allowed. Historical diagnostic omission remains a causal probe,
never a migration. No original failed session will be edited or replayed into.

## Concrete approval boundary: CLI decomposition

Additional source scope requested: `rust/cli/src/interactive/stream_loop.rs` and
new files beneath `rust/cli/src/interactive/stream_loop/`. Session wiring stays
within the already authorized `rust/cli/src/session/` directory. No provider,
RPC, TUI, source-composition helper, dependency or Cargo changes are proposed.

The existing 2,564-line file contains one 2,300+-line async function. Moving that
function whole, textual fragment inclusion, or wrapping/bypassing it does not
satisfy the size rule or this approval boundary. Proposed typed extraction:

| Proposed file | Existing semantic region / responsibility |
| --- | --- |
| `stream_loop.rs` | Existing public entry signatures and small orchestration only |
| `stream_loop/state.rs` | Explicit stream accumulators, counters and segment reset state (525-617) |
| `stream_loop/preflight.rs` | Attribution, selected preparer, threshold and pre-prompt compaction (230-369) |
| `stream_loop/telemetry.rs` | Debug capture, token seed/projection and progress registration (371-473, 496-512) |
| `stream_loop/request.rs` | Initial effective prompt/request history handoff (474-494) |
| `stream_loop/restart.rs` | Existing compaction restart macro as typed helper (619-777) |
| `stream_loop/poll.rs` | Interrupt/CLI/notify/stall polling (779-939) |
| `stream_loop/attempt.rs` | Attempt-start/abort snapshot and rollback branches (943 onward) |
| `stream_loop/assistant.rs` | Text/reasoning/tool-call assistant item branches |
| `stream_loop/tool_result.rs` | User tool result settlement branch |
| `stream_loop/final_response.rs` | FinalResponse usage/settlement branch; delegate existing continuation helpers |
| `stream_loop/recovery.rs` | Error classifier dispatch only, returning explicit Continue/Restart/Stop |
| `stream_loop/recovery_image.rs` | Existing image recovery branch |
| `stream_loop/recovery_truncation.rs` | Existing truncation retry branch |
| `stream_loop/recovery_thinking.rs` | Existing thinking-exhaustion retry branch |
| `stream_loop/recovery_network.rs` | Existing network retry branch |
| `stream_loop/recovery_context.rs` | Existing context overflow/hook compaction recovery branches |
| `stream_loop/finish.rs` | Post-loop safety net and tracker publication (2429-2564) |

Exact match-arm extents must be recorded from the pre-extraction snapshot before
editing; the responsibilities above are not permission for semantic reordering.
Each resulting file must stay below 300 lines. Use explicit typed outcomes to
preserve break/continue/error propagation, drop(stream) before settlement,
prompt clone-before-append ordering, request_history_len rollback boundaries,
tool effect/settlement ordering and counter resets. No new runtime, unsafe code,
lint suppression, dependency, or untyped shared state.

Prove the mechanical stage independently using before/after behavioral tests
and branch/call-order inventory. Only after that passes may a separate semantic
stage wire durable admission into `request.rs` and session admission helpers.
This proposal is not an approval or an extraction already performed.

## Required acceptance and evidence

Specifications precede tests; compiled behavioral RED and exact source hashes
precede production edits. Use real task-loop dequeue/cancellation, actual durable
originals and real Codex selected-history lineage in one isolated harness. The
6/2 evidence derivatives retain causal order; they do not prove task-loop,
stream partitioning, crash durability or automatic compaction by themselves.

Acceptance must additionally cover locks/hooks/persistence/dispatch races,
fresh subsequent input, identical-text UUIDs, images, duplicate/conflicting
receipts, acknowledgment loss and unresolved storage, process death, parallel
tool outcomes, two automatic compactions and cold continuation with once-only
effects. Strict negative controls mutate assistant text, invocation, outcome,
missing evidence and DAG ancestry. Coverage gaps remain explicit until those
tests run; a passing fixture replay is not complete validation.

One build owner, scoped offline ci-test/debug0/-j2, isolated roots, loopback only.
No install/restart/commit/push and no claim of live repair. BUG-283 and concurrent
REFAC-019/BOARD-024 files and evidence remain protected. Production daemon PID
922084 and executable hash d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a
are the recorded deployment baseline, not a candidate built by this task.
