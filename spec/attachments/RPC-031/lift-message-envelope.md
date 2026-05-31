# RPC-031 — Lift `MessageEnvelope` types into `codelet-core::persistence::message_envelope`

**Parent:** RPC-030
**Phase:** 1.1 (foundation — nothing else can be durable until this is done)
**Estimate:** 5 points
**Depends on:** RPC-030

---

## Goal

Move the message-envelope schema (the Claude-Code–compatible JSONL wire format) out of `codelet-napi` into `codelet-core::persistence` so that **both frontends** (TS via NAPI, Rust via `codelet-sessions`) consume the same Rust types.

On-disk JSONL format **must remain byte-identical** (the RPC-025 / RPC-026 lift precedent).

---

## Source location (move from)

`codelet/napi/src/persistence/message_envelope.rs` — **26,195 bytes, 746 lines**

Public types to move:

| Type | Purpose |
|---|---|
| `MessageEnvelope` | Outer wrapper `{ uuid, parent_uuid, timestamp, type, provider, message, request_id }` |
| `MessagePayload` | enum `User(UserMessage)` / `Assistant(AssistantMessage)` |
| `UserMessage` | `{ role, content: Vec<UserContent> }` |
| `UserContent` | enum: `Text`, `ToolResult`, `Image`, `Document`, … |
| `AssistantMessage` | `{ role, content: Vec<AssistantContent>, model, stop_reason, usage }` |
| `AssistantContent` | enum: `Text`, `ToolUse`, `Thinking` |
| `TokenUsagePerMessage` | per-message token counters |
| `ToolUseResultMetadata` | impl: `with_output(stdout, stderr) -> Self` |
| `ImageSource` | enum (base64, url, …) |
| `DocumentSource` | enum (base64, url, …) |
| `CacheControl` | enum (ephemeral, …) |

The `#[cfg(test)]` section at the bottom of `message_envelope.rs` (line 270 uses `crate::persistence::should_use_blob_storage`) must continue to compile — but since `should_use_blob_storage` will live in `codelet-core::persistence::blob` (RPC-034), update the import path during the move.

---

## Target location (move to)

`codelet/core/src/persistence/message_envelope.rs`

- Add `pub mod message_envelope;` to `codelet/core/src/persistence/mod.rs`.
- Add `pub use message_envelope::*;` in `codelet/core/src/persistence/mod.rs`.
- Crate-level re-export in `codelet/core/src/lib.rs`: `pub use persistence::{MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent, ...};` (optional, keep narrow surface).

---

## NAPI re-export shim (replace original file)

`codelet/napi/src/persistence/message_envelope.rs` becomes:

```rust
//! Re-export shim — types live in codelet-core::persistence::message_envelope.
pub use codelet_core::persistence::message_envelope::{
    MessageEnvelope, MessagePayload, UserMessage, UserContent,
    AssistantMessage, AssistantContent, TokenUsagePerMessage,
    ToolUseResultMetadata, ImageSource, DocumentSource, CacheControl,
};
```

The `pub use message_envelope::*;` in `codelet/napi/src/persistence/mod.rs` keeps every existing `crate::persistence::MessageEnvelope` import working unchanged.

---

## Audit — who currently imports these types

(From the Phase-1 research.)

### Inside `codelet/napi/src/`

| File:line | Import |
|---|---|
| `session_manager.rs:12-15` | `MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent` |
| `session_search_handler.rs:28-30` | `MessageEnvelope`-adjacent (via `get_session_messages_full` returning `StoredMessage`) |
| `persistence/blob_processing.rs` | `MessageEnvelope, MessagePayload, UserContent, AssistantContent, DocumentSource, ImageSource` |
| `persistence/tests.rs` (`#[cfg(test)]`) | Constructs envelopes for round-trip tests |

### Inside `codelet/napi/tests/`

| File | Import |
|---|---|
| `session_persistence_test.rs:34-37` | Indirect via `append_message_with_metadata` |

### Inside `codelet/core/src/`

- `compaction/trimmer_metadata.rs:4` — doc reference (no code change)
- `compaction/__tests__/trimmer.test.rs:11` — comment

(No code-level imports yet — that's the point of this card.)

---

## Acceptance criteria

1. `codelet/core/src/persistence/message_envelope.rs` contains all 11 public types listed above with identical `serde` derives and field layout.
2. `codelet/napi/src/persistence/message_envelope.rs` is a 5–10-line re-export shim.
3. `cargo build -p codelet-core` passes.
4. `cargo build -p codelet-napi` passes.
5. `cargo test -p codelet-napi --test session_persistence_test` passes (round-trip envelope on-disk format unchanged).
6. `cargo test -p codelet-napi persistence::tests` passes (existing 48 `#[test]`s).
7. Hex-diff one real `messages.jsonl` from before/after the move — must be byte-identical for the same input.

---

## Risks & notes

- The `should_use_blob_storage` reference at `message_envelope.rs:270` must be redirected when RPC-034 lands; for this card, leave a temporary `crate::persistence::should_use_blob_storage` import path (still valid because `blob.rs` is still in `napi`).
- `MessageEnvelope` is the canonical persisted shape — any field reorder breaks JSONL compatibility. Keep `serde(rename_all = …)` / `serde(tag = …)` annotations byte-for-byte.
- Do **not** add any `#[napi(object)]` to the moved types. Those decorations stay on the NAPI wire structs (`NapiStoredMessage`, etc.) in `napi_bindings.rs` — they were never on the envelope types themselves.

---

## Out of scope

- Lifting `MessageStore` / `SessionStore` / `BlobStore` — those are RPC-032 / RPC-033 / RPC-034.
- Changing the wire format. (Anything that risks JSONL compatibility is explicitly forbidden.)
