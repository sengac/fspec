# AST Research — MessageEnvelope callers and dependency graph

**Work unit:** RPC-031
**Date:** 2026-05-20
**Tool:** ast-grep + ripgrep

## 1. Definition site (move from)

`codelet/napi/src/persistence/message_envelope.rs` — 746 lines, 26,195 bytes

11 public types defined here (verified via ast-grep `pub struct MessageEnvelope`):

| Line | Type | Notes |
|------|------|-------|
| 28 | `pub struct MessageEnvelope` | `#[serde(rename_all = "camelCase")]` |
| 50 | `pub enum MessagePayload` | `#[serde(untagged)]` |
| 59 | `pub struct UserMessage` | discriminator: `role: String` defaulting to `"user"` |
| 74 | `pub enum UserContent` | `#[serde(tag = "type", rename_all = "snake_case")]` |
| 106 | `pub struct AssistantMessage` | discriminator: `role: String` defaulting to `"assistant"` |
| 133 | `pub enum AssistantContent` | `#[serde(tag = "type", rename_all = "snake_case")]` |
| 152 | `pub struct TokenUsagePerMessage` | input/output/cache token fields |
| 165 | `pub struct ToolUseResultMetadata` | `#[serde(rename_all = "camelCase")]` + `with_output` constructor |
| 195 | `pub enum ImageSource` | `#[serde(tag = "type", rename_all = "snake_case")]` |
| 203 | `pub enum DocumentSource` | `#[serde(tag = "type", rename_all = "snake_case")]` |
| 211 | `pub enum CacheControl` | `Ephemeral` variant only |

The `#[cfg(test)] mod tests` block contains 28 unit tests. **Line 270** has the only out-of-module dependency:
```rust
use crate::persistence::should_use_blob_storage;
```
This function lives in `codelet/napi/src/persistence/blob.rs` and will not be lifted until RPC-034.

## 2. Module wiring (already in place)

`codelet/napi/src/persistence/mod.rs`:
```rust
mod message_envelope;
…
pub use message_envelope::*;
```

This means every call site reaches the types via either:
- `crate::persistence::MessageEnvelope` (flat path), OR
- `super::MessageEnvelope` (when called from inside the persistence module via `use super::*` or explicit `super::MessageEnvelope`), OR
- `super::message_envelope::MessageEnvelope` (qualified path).

All three paths must continue to resolve after the lift.

## 3. Callers inside `codelet/napi/src/` (must keep compiling unchanged)

### `codelet/napi/src/session_manager.rs`
- Line 14: `use crate::persistence::{MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent, …}`
- Lines 4025–4144: constructs `MessageEnvelope` values for envelope-based persistence (3 sites).

### `codelet/napi/src/persistence/blob_processing.rs`
- Line 9: `use super::{MessageEnvelope, MessagePayload, UserContent, …}`
- Lines 51–170: walks payload variants to extract/inject blob references.

### `codelet/napi/src/persistence/napi_bindings.rs`
- Line 598: `let envelope: super::MessageEnvelope = serde_json::from_str(…)`
- Lines 854, 882, 905, 920: `&super::MessageEnvelope`, `super::MessagePayload::User`, `super::MessagePayload::Assistant` — exclusively `super::` qualification.

### `codelet/napi/src/persistence/tests.rs`
- 28+ test invocations using `MessageEnvelope`/`MessagePayload`/`UserMessage`/`AssistantMessage`/`UserContent`/`AssistantContent` directly (unqualified, via `use super::*` at top of file).
- Lines 2014, 2099, 2131: explicit `use super::message_envelope::MessageEnvelope` — fully-qualified path.

## 4. Callers outside `codelet/napi/`

```
$ rg -n "crate::persistence::(MessageEnvelope|MessagePayload|UserContent|AssistantContent|UserMessage|AssistantMessage|ToolUseResultMetadata|ImageSource|DocumentSource|CacheControl|TokenUsagePerMessage)" codelet
(no matches)

$ rg -n "persistence::(MessageEnvelope|MessagePayload|UserContent|AssistantContent|UserMessage|AssistantMessage|ToolUseResultMetadata|ImageSource|DocumentSource|CacheControl|TokenUsagePerMessage)" codelet/napi
(no matches — see callers above which use unqualified types under `use super::*` / `crate::persistence::*`)
```

Outside-napi crates only reference rig::message types named `UserContent`/`AssistantContent` — different types, different namespace. No persistence-envelope leak.

## 5. Lift impact summary

| File | Required change |
|------|-----------------|
| `codelet/core/src/persistence/mod.rs` | Add `pub mod message_envelope;` + `pub use message_envelope::*;` |
| `codelet/core/src/persistence/message_envelope.rs` | New file (verbatim copy of NAPI version, minus the `should_use_blob_storage` test and `MessageStore::index_len` dependencies — those are RPC-032 territory) |
| `codelet/napi/src/persistence/message_envelope.rs` | Becomes 10-line re-export shim |
| `codelet/napi/src/persistence/mod.rs` | Unchanged (`mod message_envelope; pub use message_envelope::*;` still works) |
| All callers above | **Unchanged** |

## 6. Tests that exercise the moved types

- `codelet/napi/src/persistence/message_envelope.rs::tests` — 28 inline serialization round-trips (move all except `test_blob_threshold`)
- `codelet/napi/src/persistence/tests.rs` — ~20 envelope tests in the persistence test suite, all unqualified — work unchanged
- `codelet/napi/src/persistence/lazy_init_tests.rs` — does NOT touch envelope types directly
- `codelet/napi/tests/session_persistence_test.rs` — round-trips on-disk envelopes via the NAPI persistence API

All run via `cargo test -p codelet-napi`.

## 7. Risk assessment

| Risk | Mitigation |
|------|------------|
| `serde` field reorder breaks JSONL compat | Copy file byte-for-byte. CI: `cargo test -p codelet-napi --test session_persistence_test`. |
| Out-of-module test break | The single `test_blob_threshold` test relocates to NAPI shim's `#[cfg(test)]` block. |
| Hidden `napi_derive::napi(object)` decoration | Verified — `MessageEnvelope` and friends have **no** `#[napi]` attributes. They were never NAPI-bridge types; the bridge structs live in `napi_bindings.rs` (`NapiStoredMessage`, etc.). |
| Test order parallelism / `TEST_MUTEX` | Tests in `persistence::tests` already serialize on `setup_test_env()` — unaffected. |

## 8. Build verification commands (executed during implementation)

```bash
cargo build -p codelet-core
cargo build -p codelet-napi
cargo test -p codelet-core persistence::message_envelope
cargo test -p codelet-napi persistence::tests
cargo test -p codelet-napi persistence::message_envelope
cargo test -p codelet-napi --test session_persistence_test
```
