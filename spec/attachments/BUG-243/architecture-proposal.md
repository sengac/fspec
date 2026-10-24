# BUG-243 additive architecture and compatibility proposal

The implementation is limited to the provider persistence binder, originals,
session replay, and new provider-state helpers. No MessageStore storage/index or
maintenance changes; no coordinate renumbering or native Codex protocol change.
The approved bounded design adds one optional versioned manifest state reference.
This proposal was sent early through AgentManager, but the
parent is running/paused and the messaging API rejects delivery rather than
queuing. It is recorded here for parent review before final acceptance. The full
bounded architecture is now user-approved in
`spec/attachments/BUG-242/architecture-approval.md`.

**Crash-safety refinement before implementation:** an unreferenced mutable
sidecar cannot distinguish missing new state from a never-migrated legacy session.
It would permit silent old-checkpoint fallback after sidecar loss. The approved
bounded architecture therefore needs an optional `provider_state` manifest
reference (`version`, `state_id`) to immutable content-addressed state. Publication
uses the existing `update_session_manifest` transaction; no separate read-only
writer interface is needed. Sync state payload before manifest-reference promotion.
Missing/corrupt referenced state fails explicitly, never selecting legacy fallback.

## Storage and ownership

`provider-state/<session UUID>/<state hash>.json` is a version-1 sidecar holding the latest
native metadata per provider/generation. The existing session writer transaction
serializes updates. Each checkpoint records source revision, prefix length and
prefix hash, provider, generation, metadata identity and native payload. Replay
checks those references before exposing top-level native provider metadata.
Checkpoint envelopes have no `message` member and cannot become dialogue.

Storage space is the sum of latest checkpoints per retained generation, not the
sum of every full-history checkpoint per turn. The current root retains every
provider generation, including reset provenance. One displaced/pending root may
remain until the next writer transaction: storage is bounded by two roots plus a
staging file, not turn count. Under ownership, the next transaction verifies the
current reference before removing unreferenced roots. There is deliberately no
cross-generation garbage collector here.
Immutable legacy source records remain owned by MessageStore maintenance.

## Legacy compatibility

An exact legacy Codex empty-assistant checkpoint signature is treated as internal
state. Originals emits a bounded explicit version-1 `provider_state` identity
marker at that *same* manifest coordinate and message ID, rather than fabricating
an original conversation envelope. Page cursors, inclusive ranges, DAG prefixes,
and all actual dialogue UUIDs keep their meaning. Missing or oversized genuine
dialogue still fails closed. No read mutates legacy source bytes.

Binding persistence migrates legacy checkpoint metadata additively and
idempotently. It adds a manifest reference/revision without changing original
message bytes, coordinates or DAG prefixes. Native reasoning, item/call identity,
outcomes, model, generation and protocol remain intact. Cold replay reads the latest verified state with no
lineage reset/fallback. Interrupted staging files are never committed state.

## Durability and limitations to audit

File data is synced before atomic rename; the directory is synced after rename.
Failures before promotion leave the old committed checkpoint; failures after
rename are explicitly reported as promoted-but-unacknowledged, never fabricated
success. Tests use real isolated temp stores and filesystem faults, not the live
user store. Reopen and syscall ordering evidence does **not** prove physical
power-loss behavior. Parent owns the final done gate and reviews this contract.

The bounded originals marker intentionally exposes identity, not megabyte native
checkpoint contents. Native checkpoint contents are continuation state, not
conversation evidence. A separate archived-display projection can be exposed by
core if requested by the BUG-244/245 owner through the parent.
