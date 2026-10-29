# BUG-284 supplemental proposal audit

Status: read-only proposal review; no implementation, build, daemon action or
production approval. Reviewed `repair-design.md`, SHA-256
`59d8b7fb227f7a9cf9de8ae57383b3f031d801e74af947f936fdebc212d9c2c9`.
Line references below refer to that snapshot.

The receipt-only archival design is preferable to filtering established
conversation references. It preserves the existing index and checkpoint hash
domains, retains canceled evidence, and correctly leaves old failed sessions
fail-closed. The proposed refactor/semantic split is also appropriate as a
proposal. Two recovery contracts need to become explicit before implementation.

## Required criterion A: admitted input canceled before dispatch

The design retains admitted membership and records a separate execution outcome
(lines 18–21, 58–63), but does not specify how the next model learns that the old
request was canceled or failed before dispatch. A hidden receipt flag is not
enough. Existing `_transcript` replay produces UI chunks and skips conversation
conversion (`rust/sessions/src/replay_content.rs:53–60`).

Acceptance criteria:

1. Given an immutable input has been durably admitted and installed, when a
   deterministic barrier interrupts before the provider send, then its input
   remains exactly once in runtime and admitted originals, the provider receives
   zero requests, and a durable terminal disposition states the known fact that
   dispatch did not occur. It does not claim remote refusal or completion.
2. Before any subsequent inference, canonical host cancellation/failure evidence
   is represented through an explicit supported control event or equivalent
   authoritative model projection. The original input bytes remain unchanged;
   no assistant completion is fabricated. Merely emitting a UI notification or
   storing a nonconversational receipt does not satisfy this criterion.
3. A different fresh user input then runs once. Capture its actual selected
   request and prove the canceled instruction is presented as canceled, not as
   an unexplained unfinished task. Runtime, cold restore and selected compaction
   must agree on the exact disposition content, identity and order.
4. Repeat across two automatic compactions and cold continuation. The resulting
   summary must retain the cancellation fact; the fixture's canceled action
   receives zero tool calls while the fresh permitted action executes once.
5. Inject failure while persisting/publishing the terminal disposition. Further
   inference remains blocked until it is authoritatively reconciled; no path
   silently resumes the old instruction or removes its admitted original.

Apply the same test shape to provider construction and pre-prompt compaction
failure after admission (lines 60–63). The actual failure reason must remain
truthful; calling every such failure a user cancellation would be incorrect.

## Required criterion B: admitted input with uncertain dispatch after crash

Lines 83–87 restore admitted input and permit a fresh instruction to continue,
but do not define how an admitted input without a definitive terminal is
reconciled. A local attempt-start record does not prove whether a provider or a
tool saw the request. Do not infer that either nothing happened or it completed.

Acceptance criteria:

1. Kill an isolated child process at separately controlled points: after
   admission before dispatch preparation, after durable attempt-start before
   sending, after sending before acknowledgment, and after an observed tool
   effect before recording its terminal result. Preserve the fixture's durable
   evidence and external effect counter across child restarts.
2. Cold restore includes the admitted original once and never automatically
   requeues it or repeats its effects. Pending stays Pending and audit-only;
   Cancelled stays non-admitted. A crash alone changes neither disposition.
3. If evidence cannot establish a definitive execution outcome, mark that
   execution state unresolved/unknown and block automatic continuation that
   could repeat the action. A new arbitrary user turn must not accidentally
   convert the old request into renewed execution authority.
4. Permit future inference only through an explicit recovery decision and a
   truthful, durable model-visible disposition, with unsupported uncertainty
   remaining a recoverable error. No demand for nonexistent proof of remote
   completion; no guessed cancellation or synthetic success.
5. A lost manifest acknowledgment is settled by the same UUID and exact
   immutable content under ownership. If readback/barrier settlement remains
   uncertain, verify zero additional provider dispatches and an actionable
   recovery diagnostic. Retry never creates a fresh submission identity.

These criteria do not claim general exactly-once remote effects are obtainable.
They require no blind replay under uncertainty and exact treatment of outcomes
the local evidence actually establishes.

## Remaining proposal clarifications and scope control

- The durable recovery spool at lines 32–36 must preserve canonical bytes and
  blob evidence through failure before manifest receipt promotion. GC roots for
  committed receipts alone cannot prove that gap is protected; currently
  `message_cleanup.rs:50–68` roots only manifest message references. Test cleanup
  after each failed promotion, followed by exact receipt recovery.
- Define submission UUID versus provider-attempt/internal-continuation identity
  (lines 10–14, 40, 72–73). An attempt retry must not append another admitted
  original; a genuinely new continuation is not permission to reuse the prior
  immutable input UUID with changed content. Inventory the existing initial and
  restart request paths before wiring admission.
- Fork/merge treatment is omitted. A fork must not inherit execution rights to
  Pending input, and copying an admitted canceled request must retain its
  cancellation context or reject that unsupported boundary. This is an
  acceptance/policy gap, not authorization to expand production scope here.
- The 18 proposed CLI modules are a decomposition map, not yet proof that every
  file can remain below 300 lines. The explicit pre-edit extents, branch/drop/
  rollback order inventory and independently green mechanical stage already
  requested at lines 137–147 are necessary. Avoid adding lifecycle redesign to
  the mechanical extraction.
- The "durable retention before submission acknowledgment" promise (lines
  38–42) conflicts with the proposed no-RPC-change scope (109–110) if it includes
  caller-visible ordinary input success: `sessions/handle_impl.rs:229–245`
  discards `BackgroundSession::send_input` errors;
  `rpc/service_implementation/interaction_sessions.rs:116–120` returns no result;
  `rpc/mcp_dispatch/group_15.rs:47–60` reports success after transport completion.
  Durable persistence beneath those APIs cannot distinguish accepted input from
  storage/queue refusal to their caller. Define the bounded guarantee explicitly
  and report the interface limitation; do not silently expand production scope.
- Existing background entrypoints are synchronous
  (`background_session/messaging.rs:30–40,47–63`). Off-executor persistence plus
  retention-before-return needs a concrete await/acknowledgment bridge; a detached
  worker alone cannot provide it. The supervisor Result path can propagate
  errors, unlike the ordinary-input path above.
- Specify whether "retry keeps UUID" means in-process persistence/attempt retry
  only. Current `rpc-types/src/lib.rs:1153–1160` has no caller submission ID.
  A lost external response cannot be safely deduplicated as a retry of a newly
  server-generated UUID without an explicit idempotency contract; identical
  text is not identity. No wire/API change is authorized by this observation.

## Approval state

Automatic approval review rejected the attempted authorization of the oversized
CLI refactor: the controller assertion was not the required explicit human
approval. The blocked target is production decomposition of
`rust/cli/src/interactive/stream_loop.rs`, including indirect replacements or
bypasses. No retry, workaround, or production change was performed in this
review. Explicit approval remains pending; proposal/specification/causal RED
preparation may continue within the parent-stated boundary. This audit is not
approval and does not relax that boundary.
