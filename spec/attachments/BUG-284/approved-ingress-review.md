# BUG-284 minimal ingress and admission review

Read-only recommendation, following `final-preparation-review.md` and
`approved-implementation-handoff.md`. The focused CLI extraction is approved.
No shared interface scope is released by this note; no source edits, builds or
native mutations were performed.

## Minimal behavior, without an external retry protocol

Use one synchronous **core retention transaction** and a thin **awaitable session
submission operation** around an owned blocking worker. Core owns immutable
original/receipt storage; sessions owns queue publication; agent-loop owns
admission. Do not introduce a new daemon, queue framework or provider protocol.

Recommended submission order:

1. Allocate one server-local input UUID and capture the actual submission epoch,
   origin and immutable canonical payload before offloading. Capturing the epoch
   only after slow storage would incorrectly refresh an input across interrupt.
2. The owned worker durably retains Pending using that UUID, then publishes a
   receipt-bearing item to the existing bounded queue. Report success only after
   both steps. Queue refusal retains the original with a truthful non-admitted
   disposition; failure to finalize that refusal stays unresolved and never
   authorizes inference. Missing session/manager is an error, not success.
3. The async caller awaits completion. Dropping its future must not abandon the
   worker's ownership or the retained evidence. Lost transport acknowledgment is
   not proof of refusal and does not create a fresh local identity.
4. Use the same core receipt API for fresh user and supervisor inputs. Keep
   AgentManager's idle-only admission restriction; receipt retention must not
   turn its rejected Busy result into accepted work. Claiming/publishing status,
   queue permits and pending counters need one settled success/refusal path.

This fixes the acknowledged-but-not-dequeued crash window. `spawn` followed by
immediate Ok, persistence first performed on dequeue, or a volatile Pending map
does not. The existing recovery commit always appends normal conversation
(`agent-loop/persistence_retry_commit.rs:72–79`); do not reuse it unchanged for
Pending receipt recovery.

The server-local UUID need not be added to the wire schema for this repair.
Internal persistence/attempt retries retain identity. Distinct external calls
are distinct submissions, even with identical text. A public idempotency key,
lost-response lookup protocol and client retry deduplication are separate work;
do not promise them or make them prerequisites for this causal fix.

## Smallest general async acknowledgment bridge

Add two narrow object-safe completion methods to the shared handle: durable user
submission with optional ThinkingConfig, and durable supervisor submission.
An owned/boxed Send future returning `Result<(), String>` needs no dependency
change; boxed futures already exist at `core/loops/mod.rs:36–38`. Production
delegates to the session worker. Unsupported defaults fail explicitly: they
must not call an old void method and manufacture Ok.

RPC handlers await completion. The two ordinary input RPC responses must carry
`Result<(), String>`; supervisor RPC already does. This is necessary result
plumbing, not a payload redesign. Current ordinary handle calls discard errors
(`sessions/handle_impl.rs:229–245`), ordinary RPC methods return void
(`rpc/service_implementation/interaction_sessions.rs:116–120` and
`session_control.rs:95–104`), and absent supervisor handles return Ok
(`supervision_interaction.rs:49–58`). Thus core/session changes alone cannot
truthfully repair caller-visible success in every current path.

Exact shared coordination targets, not permission to edit:

- `core/src/session_manager_handle.rs` and production
  `sessions/src/handle_impl.rs`, including truthful stub behavior.
- `rpc/src/service_implementation/{interaction_sessions,session_control,
  supervision_interaction}.rs` for awaiting the two completion operations.
- `rpc/src/service_declaration/{workspace,providers_sessions}.rs` for ordinary
  response types, plus `rpc/src/mcp_dispatch/{group_01,group_15}.rs` for the inner
  error. Supervisor wire shape and supervisor result adapters need no redesign.
- Four existing TUI transport adapters: `embedded/{sessions,
  session_state_transcripts}.rs` and `websocket/{provider_sessions,conversation}.rs`.
  Their backend signatures already return Result; propagate the daemon result
  rather than discarding it. This is no UI flow change.

The handle trait and implementation are oversized existing files; their edits
and any required organization must be coordinated, not inferred from approval
of the CLI split. The old synchronous methods must not remain an unnoticed
production route that bypasses retention. Enumerate and migrate actual callers
within agreed scope rather than redesigning the whole trait.

Do not substitute a universal `block_in_place` wrapper: the existing source
explicitly documents that it panics on current-thread Tokio runtimes
(`sessions/handle_impl.rs:11–27`). Blocking on a receiver can instead freeze the
executor. The awaitable bridge makes the completion contract explicit and keeps
storage off async workers.

## Admission ownership inside already scoped implementation

Retaining Pending is not admission. Dequeue must carry its receipt identity;
hook refusal and cancellation before admission settle it without appending
`manifest.messages`. After acquiring the runtime lock, a single operation owner
settles the cancellation decision, atomic receipt promotion/reference append,
and exact runtime installation. The worker owns this obligation through caller
drop and lost acknowledgment; no later inference passes an unresolved owner.

Do not hold the current `interruption_reason` standard mutex across disk I/O:
`interrupt()` takes it before flag/epoch updates and tool abort
(`background_session/messaging.rs:65–76`). That would make interrupt wait on
storage. Use a short gate to claim/record operation ownership and cancellation
order, while the owned operation settles off-executor. An interrupt remains a
prompt request to stop dispatch/tools; its durable disposition may await the
owned admission outcome. Explicitly define which claim wins the race. If an
admission already owns publication, finish installing its exact committed input
then finalize cancellation before dispatch; do not drop the committed original.
If cancellation wins before admission ownership, settle Cancelled without
membership. A bare atomic epoch check before a slow write is insufficient.

This is a small session ownership contract, not a mandate for a general actor
framework. Retain the reviewed model-visible terminal disposition and uncertain
crash recovery requirements. Pre-prompt compaction sees admitted history;
request construction uses its explicit identity/history view to avoid duplication.

## Direct callers and bounded verification

Audit direct callers of the same worker: scheduler
(`session_manager.rs:1299–1313`; release the session-map guard before awaiting),
loop callback (`handle_impl.rs:2268–2273`), bridge callback
(`agent-loop/bridge.rs:35–73`), AgentManager
(`agent_manager_handler.rs:498–518`), manual compaction continuation and internal
retry inputs. Changed corrective/continuation text is a distinct internal input,
not conflicting reuse of the original immutable UUID. A void callback is not a
durable acknowledgment; document and coordinate any completion-shape limitation
instead of claiming it fixed. NAPI's synchronous wrapper is a separate shared
caller if that frontend's acknowledgment is included.

Minimum bridge proof: successful acknowledgment then kill before dequeue retains
the original; storage/queue refusal reaches the actual caller; an unrelated
current-thread-runtime heartbeat and interrupt remain responsive during blocked
storage; concurrent supervisor/regular receipts retain original epochs; aborting
the submission waiter cannot lose a committed receipt or dispatch before its
retention. Add deterministic admission/cancel barriers, then use the already
required actual queued fresh-input/Codex/cold/repeated-compaction tests. Keep
wire retry deduplication outside the claimed coverage.

**Next action:** native can implement/test core receipt semantics and owned
admission in isolation while the parent coordinates the exact shared result/
completion plumbing above. Until that plumbing is released and validated, do
not claim all ingress acknowledgments are durable or mark the full repair Done.
