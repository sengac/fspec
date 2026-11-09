# BUG-332 root cause: session replay invents envelope fields for rows stored without one

## Problem

BUG-327 made replay start from the stored envelope, but kept fallbacks for rows that
have none (`rust/core/src/persistence/session_replay.rs:33-48`):

- `parentUuid` → `null`, `requestId` → `null`, `timestamp` → the row's `created_at`,
  `type` → the row's role, `provider` → `"unknown"`, `message` → `{}`.

So a row with no envelope replays as a plausible message with an invented provider and
no parent instead of being reported. There is no backward-compatibility requirement for
such rows.

Who writes rows without an envelope:

- Production conversation rows all come from `RecoveryRecord`s committed in
  `rust/agent-loop/src/persistence_retry_commit.rs::commit`; their metadata is a
  serialized `MessageEnvelope` (`agent-loop/src/persist.rs:177-230`). The only gap is
  `commit`'s `if metadata.contains_key("message")` (`persistence_retry_commit.rs:46`):
  a record without one is stored as-is, without an envelope.
- The compaction summary row (`Uuid::nil`) is synthesized in memory by
  `get_session_messages` (`rust/core/src/persistence/manifest_messages.rs:57-78`) with
  only `_synthetic`, `_compactionSummary` and `_compactedBeforeIndex` metadata.
- `append_message` / `append_message_with_metadata`
  (`manifest_messages.rs:1-18` → `manifest_updates.rs:54-76`) store the caller's
  metadata as-is. They have no production callers; dozens of test fixtures use them to
  build sessions they then replay or resume.
- Legacy provider-state checkpoint rows are skipped before envelope handling
  (`provider_state::legacy::is_checkpoint`, `session_replay.rs:20`) and stay skipped.

## Fix

- Replay requires the stored envelope: a conversation row whose metadata lacks any of the
  envelope keys `parentUuid`, `timestamp`, `type`, `provider`,
  `message` (object) or `requestId` fails replay with
  `stored message <id> has no message envelope (missing <key>)`. Replay still sets the
  row identity (`uuid`), the role and the blob-rehydrated content.
- Writers that did not write an envelope now write a real one:
  - the compaction summary row carries a full envelope (user message, provider
    `"compaction"`, `parentUuid` null, timestamp = compaction time, `requestId` null);
  - `append_message` / `append_message_with_metadata` build a `MessageEnvelope` for the
    row (role, the session's provider, the content as a text message, `parentUuid` /
    `requestId` null) and merge caller metadata over it, so fixtures persist rows of the
    production shape;
  - `persistence_retry_commit::commit` refuses a conversation record without a message
    envelope instead of storing it bare.
- `message_envelope::text_envelope_metadata` builds those plain-text envelope fields
  (used by the append writers and the compaction summary).
- The legacy Codex checkpoint row detector
  (`provider_state/legacy.rs::is_checkpoint`) accepts the envelope fields those rows
  now carry when they are written through the append helpers.
- Fixtures that wrote conversation rows below the helpers write real envelopes:
  `core/tests/shared_message_store_cleanup_safety.rs` (raw `MessageStore` rows that a
  session references) and `cli/tests/bug243/{fixtures,authority}.rs` (originals compared
  byte-for-byte with what was stored).

## Alternatives rejected

- Keeping the fallbacks for "rows stored without an envelope": there is no data to stay
  compatible with, and invented `provider: "unknown"` / null parents hide a writer bug.
- Exempting the compaction summary from the check: it would keep a second, implicit
  envelope shape; giving it a real envelope keeps one replay contract.
- Rewriting every fixture to build envelopes by hand: the fixture helpers are the
  writers; fixing them once gives every fixture the production shape.

## BUG-326's `#[serde(default)]` on `current_context_output_tokens` (reviewed, unchanged)

`TokenUsage` (`rust/core/src/persistence/manifest.rs:21-51`) carries
`#[serde(default)]` on every one of its fields (`current_context_tokens`,
`cumulative_billed_input`, `cumulative_billed_output`, `cache_read_tokens`,
`cache_creation_tokens`, `reasoning_tokens`) and derives `Default`; the new field
follows that shared convention. The defaults are counters' zero values that a manifest
legitimately has before any request is measured, not a substitute for a value that
should have been there, so this is a schema-wide design rather than a fallback added by
BUG-326. Making only the new field required would make the schema inconsistent, and
making the whole `TokenUsage` schema required is a separate manifest-schema decision
outside this bug. Left unchanged.
