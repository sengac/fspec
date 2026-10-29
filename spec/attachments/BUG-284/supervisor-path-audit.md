# Supervisor delivery, cancellation and selected-history audit

Source-only audit of `/home/rquast/projects/fspec` at HEAD
`626eee1ec96a8e3ec616b5e6411a5c1596815709`. No production edits, builds, daemon
operations or historical-session modifications were performed. The parent owns
the preserved real-session comparison and production-code replay.

## Conclusion

The current source has a concrete path that **persists a canceled supervisor
input as an ordinary authoritative user original, then skips inference without
putting that input into runtime/provider history**. Selected compaction later
reconstructs all authoritative originals and encounters the extra user message.
That is sufficient to cause the generic `assistant, tool result or DAG ancestry`
refusal once a later checkpoint expects another message at that position.

The source proves this path exists; real-session timestamps and replay must
establish that it caused each incident. The parent's supplied first-divergence
finding is consistent with it in both sessions: a durable supervisor input
appears before the controller input expected by the checkpoint. This audit does
not assert that this explains every historical compaction failure.

The refusal string is misleadingly narrow: the branch also rejects unmatched
**user** messages. `rust/providers/src/codex/history_lineage.rs:63–91` rejects any
unmatched non-call projection other than reasoning/web-search items; it does
not restrict the refusal to assistant/tool-result/DAG types.

## Delivery and persistence path

1. `rust/sessions/src/handle_impl.rs:698–719` converts the native RPC's
   `IncomingMessageInput` to an `IncomingMessage`, then calls
   `BackgroundSession::receive_incoming_message`.
2. `rust/sessions/src/background_session/messaging.rs:30–40` stamps the message
   with `queued_input_epoch()` and enqueues it. This ordinary RPC path has **no
   idle-only gate**. `background_session/agent_message.rs:42–77` has an idle-only
   gate for the distinct AgentManager route; its protection cannot be assumed
   for direct `receive_incoming_message` RPCs.
3. `background_session/interruption.rs:21–27` returns no queued epoch when the
   session is already interrupted. An input accepted before interruption can
   instead carry an old epoch. `messaging.rs:65–76` sets the interruption flag
   and increments the epoch. `compaction_handoff.rs:6–12` treats either an active
   interruption or an epoch mismatch as canceled.
4. `rust/agent-loop/src/agent_loop.rs:45–72` dequeues the supervisor input and
   sets `cancelled_at_submission` when its epoch was absent. It does **not** mark
   supervisor input as `is_fresh_user_input`; only the regular input queue does
   that (`:48–54`). The biased select chooses regular inputs first if both
   channels are ready, so enqueue time alone does not prove execution order.
5. `agent_loop/input.rs:44–72` formats `[SUPERVISOR: ...] ...`, decrements the
   pending count and emits an IncomingMessage presentation event. This is not
   the authoritative-original persistence step.
6. `agent_loop/prompt.rs:27–163` admits the pending prompt. A genuinely new
   regular user input clears interruption only when its current epoch is valid
   (`:34–35`, `background_session/interruption.rs:29–42`). Supervisor input does
   not run that reset. Cancellation also suppresses hooks (`prompt.rs:80–82`).
7. **The critical ordering:** `prompt.rs:126–132` always persists the pending
   input, including canceled supervisor input, before the cancellation check
   at `:149–154`. Neither `cancelled_at_submission` nor the ordinary supervisor
   epoch is checked before that persistence call. The earlier cancellation
   exits at `:37–46` and `:115–124` apply to compaction retries, not this path.
8. `user_persistence.rs:10–25` calls `persist::persist_user_content`.
   `persist.rs:20–43` writes an ordinary user envelope and calls
   `submit_envelope(..., None)`: no cancellation reason, accepted/consumed flag,
   input origin, or durable disposition is attached to the user original.
   `persist.rs:177–205` submits that record through the persistence spool.
9. `prompt.rs:8–25` returns canceled by emitting Interrupted/Idle/Done. It does
   not add the persisted input to `inner_session.messages`. Returning false
   makes `agent_loop.rs:114–116` skip routing and provider execution entirely.
   The normal runtime user-message append occurs much later in
   `rust/cli/src/interactive/stream_loop.rs:482–494`.

Additional race windows exist: the second cancellation check after acquiring
the runtime lock (`agent_loop.rs:127–129`) and the one immediately before
selected provider execution (`:210–212`) can skip after persistence. Those
windows are source possibilities, not established incident timing in this
audit. The source issue is not limited to supervisor text; canceled regular
user inputs can have the same durable/runtime split.

## How the canceled original enters selected compaction

- `rust/cli/src/session/compaction.rs:18–26` captures the durable source from the
  manifest, not a list of provider-consumed inputs.
- `rust/core/src/persistence/manifest_messages.rs:54–95` returns all referenced
  originals before any compaction, or summary plus all post-boundary originals
  afterward. It has no cancellation/consumption filter.
- `rust/core/src/persistence/session_replay.rs:10–66` turns these records into
  envelopes, preserving their user content and rehydrating blobs. Its skips
  concern legacy checkpoints, not canceled inputs.
- `rust/cli/src/session/compaction.rs:195–207` decodes every conversational
  envelope. `session/provider.rs:8–51` converts an ordinary user envelope without
  any consumption filter; the special exclusion is empty Codex metadata.
- `session/selected_compaction.rs:43–66` passes that reconstructed durable
  history to the selected request preparer.
- `providers/src/codex/history_lineage.rs:63–91` compares saved projections to
  the front of incoming projections in order. A durable-only supervisor user
  message before a later checkpointed controller user message is not skipped.
  The later expected user fails to match and triggers the generic refusal.

This explains why normal turns can continue before compaction surfaces the
problem: active inference can keep using its coherent runtime history, while
the compaction admission path reconstructs a different durable history. It is
an inference from the code paths, to be confirmed by the parent's replay.

## Existing tests expose the missing composition

`rust/agent-loop/tests/bug276_prompt_admission_cancellation.rs` already tests the
two diverging facts as desired outcomes:

- Lines 12–55: canceled regular input must be absent from runtime history but
  present exactly once in the archived originals; new input is dispatched once.
- Lines 60–95 and 100–176: cancellation during hooks/persistence keeps the
  original recoverable while dispatch/runtime inclusion is suppressed.
- Lines 221–261: supervisors queued before and during interruption must not
  enter runtime inference, while a genuinely new user input starts normally.

These tests use `ProviderType::Custom("stub")`, not Codex
(`tests/support/prompt_admission.rs:38–57`). They do not follow cancellation with
a Codex checkpoint, selected compaction, or cold continuation. Thus they verify
cancellation and recoverability locally but miss the resulting strict-lineage
conflict. The fixture's `originals` helper reads manifest references, so
“recoverable archive” here is the same authoritative original list used by
compaction, not a separate canceled-input audit ledger.

`prompt.rs` and this BUG-276 test file were introduced/updated in commit
`042d3974b4dbdf208fa006d7474edd504fd4ea5f` (2026-10-28 06:23:52 UTC), the same
aggregate commit that contains BUG-278. This is a separate adjacent repair
effort relevant to the new cause, not evidence that BUG-278's generation-domain
fix is wrong. BUG-282's canonical-output edit does not touch this path.

## Evidence still needed and proposed verification scope

The parent should correlate each durable-only message's timestamp and hash with
the corresponding IncomingMessage → Interrupted/Done event sequence, any earlier
interrupt call and the later controller input/provider attempt. A source path
and matching message shape alone do not prove which cancellation window fired.
The authoritative original has no cancellation marker, so avoid inferring
execution from persistence alone or treating a native dispatch acknowledgment as
proof of model consumption.

Any production repair needs an explicit distinction between accepted inputs
preserved for recovery/audit and conversational inputs actually admitted to a
provider history, including crash/cold-restore semantics. Simply dropping the
message, deleting history, adding it retroactively to a live checkpoint, or
letting lineage skip arbitrary unmatched user messages would not establish the
required authority and is not proposed here. Recoverability and strict lineage
must both remain intact.

Acceptance should reproduce real failure-derived inputs without exposing their
contents and cover: supervisor arrival while interrupted; interruption after
enqueue; cancellation during durable persistence; a subsequent regular input;
first and successive selected compactions; cold restore; and unchanged
side-effect receipts. Assert both truthful canceled-input retention and exact
provider-consumed conversation order. Include ordinary canceled user input and
the corresponding untouched valid-input controls. The parent is building the
exact current-production replay; this source audit makes no execution or
live-fix claim.
