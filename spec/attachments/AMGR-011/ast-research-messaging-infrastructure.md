# AMGR-011 AST Research — Context Resolution Infrastructure

## Research Date: 2026-03-16

## Purpose
Analyze the existing infrastructure that AMGR-011 will integrate with for message context resolution.

---

## 1. Current Message Handler (agent_manager_handler.rs)

The `handle_message()` function at line 278 takes `(session_manager, calling_session_id, target_session_id, message)` and:
1. Looks up target session
2. Gets sender's role from calling session
3. Constructs `IncomingMessage { source_session_id, role_name, message, images: None }`
4. Calls `target_session.receive_incoming_message(incoming)` via try_send

**Integration point:** The `message` field of `IncomingMessage` is where resolved context will be appended.

## 2. Persistence Layer API (codelet/napi/src/persistence/mod.rs)

Public functions available for context resolution:

```rust
pub fn load_session(id: Uuid) -> Result<SessionManifest, String>
pub fn get_session_messages_full(session: &SessionManifest) -> Result<Vec<StoredMessage>, String>
pub fn get_message(id: Uuid) -> Result<Option<StoredMessage>, String>
```

- `load_session()` loads a session by UUID from the SessionStore singleton
- `get_session_messages_full()` returns ALL messages in session order (0-indexed)
- Turn index = position in `session.messages` vector

## 3. Blob Resolution (codelet/napi/src/persistence/blob_processing.rs)

```rust
pub fn is_blob_reference(s: &str) -> bool
pub fn extract_blob_hash(s: &str) -> Option<&str>
```

The `resolve_message_content()` function in session_search_handler.rs (currently private) handles blob resolution:
1. If `msg.content` is a blob reference → fetch from BlobStore
2. If `msg.blob_refs` is non-empty → join with resolved blobs
3. Otherwise → return content as-is

**Decision:** Make `resolve_message_content` pub, OR duplicate the ~20 lines in the handler.

## 4. Ripgrep Search Engine (codelet/napi/src/session_search_handler.rs)

Public helper functions for search query variant:

```rust
pub fn build_ripgrep_matcher(query: &str) -> Result<RegexMatcher, String>
pub fn ripgrep_is_match(matcher: &RegexMatcher, content: &str) -> bool
pub fn ripgrep_find(matcher: &RegexMatcher, content: &str) -> Option<(usize, usize)>
```

Uses `grep_regex::RegexMatcherBuilder` — same engine as the Grep tool.

## 5. Message Format (codelet/napi/src/session_manager.rs)

```rust
pub fn format_incoming_message(input: &IncomingMessage) -> String {
    format!("[SUPERVISOR: {} | Session: {}] {}", input.role_name, input.source_session_id, input.message)
}
```

The `IncomingMessage` struct:
```rust
pub struct IncomingMessage {
    pub source_session_id: String,
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

## 6. Current AgentManagerAction::Message Type (codelet/tools/src/agent_manager/types.rs)

```rust
Message {
    session_id: String,
    message: String,
}
```

**Change needed:** Add `context: Option<Vec<ContextReference>>` field with `#[serde(default)]`.

## 7. Implementation Plan

1. **types.rs**: Add `ContextReference` enum and `context` field to `Message` variant + `MessageDeliveredWithContext` result variant
2. **agent_manager_handler.rs**: Add `resolve_context()` function that:
   - Iterates context references
   - For each: load session → get messages → resolve blob refs → format as XML
   - Returns `(resolved_text: String, success_count: usize)`
3. **Modify `handle_message()`** to accept context, call `resolve_context()`, append to message text
4. **Make `resolve_message_content` public** in session_search_handler.rs for reuse
