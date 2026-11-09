# BUG-327 root cause: session replay drops stored envelope metadata

## Problem

`get_session_message_envelopes` / `envelopes_for_manifest`
(`rust/core/src/persistence/session_replay.rs:29-37`) build a new envelope:

```
{"uuid": stored.id, "parentUuid": null, "timestamp": stored.created_at,
 "type": stored.role, "provider": metadata.provider or "unknown",
 "message": {"role", "content"}, "requestId": null}
```

and then merge `stored.metadata` with `entry(key).or_insert(value)`. Because
`parentUuid`, `timestamp`, `requestId` and `message` are already present, the
stored values never win: the original `parentUuid`, the original envelope
`timestamp`, the `requestId`, and the stored message object's `id`, `model`,
`stop_reason` and `usage` are all discarded. The full envelope is persisted by
the write path (`rust/agent-loop/src/persistence_retry_commit.rs:44-84`), so the
loss happens only on read.

Current callers read only role/content (resume via `Replay::parse` in
`rust/sessions/src/replay_content.rs`, compaction in
`rust/cli/src/session/compaction.rs`), so there is no visible symptom today,
but the function's contract — return the stored envelopes — is broken.

## Design

- Start from the stored metadata object (the persisted envelope).
- Set `uuid` from `stored.id` (the row identity is authoritative).
- `parentUuid`, `timestamp`, `type`, `provider`, `requestId` come from the
  stored envelope; they are inserted from the row only when the stored metadata
  has no envelope (see implementation notes on rows written without one).
- Keep the stored `message` object and overwrite only its `role` and `content`
  (content is the blob-rehydrated value computed above).
- Keep `strip_native_metadata`, the compaction (nil uuid) overrides and blob
  rehydration unchanged.

## Alternatives rejected

- Swapping `or_insert` for overwrite: stored keys would replace the
  rehydrated content and the authoritative `uuid`, and the message object would
  still be rebuilt from scratch.
- Leaving it because no caller reads the fields: the function is the replay
  contract for every consumer (Paseo, restore), and losing ids breaks parent
  chains and per-request usage.

## Wire / API changes

None in signatures. Envelope JSON now carries the stored `parentUuid`,
`timestamp`, `requestId` and message `id`/`model`/`stop_reason`/`usage`.
