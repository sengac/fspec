# BUG-243 / BUG-245 archival API contract

The BUG-244 delivery proposal requests full archival export independently from
the reduced model replay. Existing originals retrieval is intentionally stricter
than archival display: older structured records may lack envelope UUIDs, and
large dialogue must remain available for display without raising evidence limits.

Planned additive core export, without editing sessions `replay_content.rs`:

```rust
pub fn get_session_archival_page(
    session_id: uuid::Uuid,
    cursor: Option<ArchiveCursor>,
    max_bytes: usize,
) -> Result<ArchivePage, ArchiveError>;
```

Types are exported from `codelet_core::persistence`:

- `ArchiveCursor`: version, session_id, revision, end_turn (exclusive),
  prefix_hash, turn_index, byte_offset, optional record_hash.
- `ArchivePage`: session_id, source_revision, end_turn, optional fragment,
  next_cursor. One page carries at most one original-envelope fragment.
- `ArchiveFragment`: turn_index, message_id, timestamp, record_kind,
  byte_offset, total_bytes, json_fragment.
- `ArchiveRecordKind`: `Conversation`, `ProviderState`, `LegacyText`.

`json_fragment` contains a UTF-8-safe fragment of canonical envelope JSON.
Concatenate fragments for a record in increasing byte_offset, requiring the
same message_id/total_bytes and final byte count, then deserialize the complete
JSON envelope. Timestamps and message IDs outside the envelope are authoritative
persisted record identity. ProviderState records carry only bounded identity
markers; exclude them from conversational display explicitly. LegacyText is an
explicit display projection of a pre-envelope stored text record, never claimed
as an exact original envelope. Structured metadata is preserved, including its
original UUID when present; missing display-envelope fields may receive persisted
defaults without changing the underlying original.

Pass the returned cursor unchanged. It pins an immutable manifest prefix and
exclusive snapshot end; appended suffixes cannot leak into the snapshot.
Changed/missing prefixes or referenced source records fail explicitly. No summary
substitution, inference installation, token update or provider reset occurs.
Each serialized page fits max_bytes (1024..61440), including cursor and escaped
fragment overhead. Large envelopes span multiple pages without truncation.
The prefix hash includes canonical persisted record contents as well as ordered
manifest references. Metadata changes under an existing message ID invalidate
the snapshot explicitly. A resumed partial record also verifies its canonical
envelope hash. Canonical key ordering makes fragments stable across store reloads.

The existing `get_session_message_envelopes` / `envelopes_for_manifest` remain
the **only reduced model-context replay** paths. This API is archival display
only. The exact-evidence originals contract retains its current typed missing-
envelope and oversized-envelope behavior for actual conversation records.

The exported API is implemented in `rust/core/src/persistence/archive.rs`, with
types, record projection, and snapshot validation in `archive/`. Executable RED
for oversized archival dialogue is recorded in `/tmp/bug243-integrated-red.log`;
checkpoint-separation RED is in `/tmp/bug243-red-runtime.log`. Scoped GREEN
validation uses the existing release-slim target and serialized Cargo lock.
