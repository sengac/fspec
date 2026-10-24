# PROV-153 integration design

> Historical design: BUG-241 supersedes boolean buffering start/stop, visible
> moderation receipt, and full-snapshot user notifications. See
> `spec/attachments/BUG-241/codex-metadata-upstream-comparison.md` and the BUG-241
> specifications for the corrected upstream semantics and presentation lifecycle.

Upstream audited at `bb64b89` in `/tmp/codex`. Capture headers before body conversion.
Transport worker PROV-148 owns `model.rs`/`streaming.rs` and was asked to use
`HttpClientExt::send_streaming` plus `eventsource_stream` instead of GenericEventSource.

## Provider API

`codelet_providers::codex::metadata::CodexMetadataHandle`: Clone + Default.

- `apply_headers(&http::HeaderMap)` preserves the allowlisted HTTP metadata.
- `apply_event(&serde_json::Value)` merges response.headers over top-level headers.
- `begin_stream() -> CodexMetadataGuard`; Drop clears transient buffering on errors/cancellation.
- `finish()` clears transient buffering; does not clear diagnostic identifiers.
- `status() -> CodexStatus` returns only sanitized presentation fields.
- `turn_state() -> Option<String>` and `models_etag() -> Option<String>` expose private transport metadata separately.
- `with_observer(impl Fn(CodexStatus) + Send + Sync + 'static) -> Self` delivers changed snapshots synchronously.
- `CodexStatus::notification() -> String` uses fixed labels, bounded identifier allowlists,
  recognized verification names and moderation presence, never raw reasons or metadata.

Model builder: `CodexResponsesModel::with_metadata_handle(self, handle) -> Self`.
Provider builder: `CodexProvider::with_metadata_handle(self, handle) -> Self`.
Transport calls begin_stream inside the stream, headers before consuming body, apply_event
before normal event handling. Metadata processing never yields assistant text, tools or usage.

## Session/UI integration

Optional runtime handle on BackgroundSession with default None, initialized only for Codex.
New small session module constructs observer using Weak<BackgroundSession> and existing
`StreamChunk::UserNotification` with info severity. Existing TUI already renders these.
Codex-selected dispatch attaches the session handle to provider before agent creation.
Coordinate this setup seam with PROV-149 continuation wiring rather than duplicate dispatch.
No shared RPC schema changes. No persisted opaque metadata or routing payloads in this card.

Safety object/true starts buffering; object updates advisory model and counts (not raw reasons).
Explicit false clears despite upstream ignoring it. End/error/drop also clear. Model advice
never changes selection. Unknown or malformed optional fields are ignored.

## Current gate

Feature precedes tests. No production files edited. PROV-147/148 dependencies must be
coordinated before implementing transition. Disk initially full; no Cargo until safe space.
