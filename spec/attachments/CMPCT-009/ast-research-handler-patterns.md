# AST Research: inject_summary Handler Patterns

## Research Date: 2026-03-09
## Work Unit: CMPCT-009

---

## 1. SessionSearch Handler Pattern (to follow)

**File:** `codelet/napi/src/session_search_handler.rs:35`

```rust
pub fn create_handler(project_path: PathBuf) -> SessionSearchHandler {
    Arc::new(move |action: SessionSearchAction, session_id: Uuid| {
        match action { ... }
    })
}
```

- Returns `SessionSearchHandler` (Arc<dyn Fn>)
- Captures `project_path` (PathBuf) in closure
- Synchronous closure — no async

---

## 2. InjectSummary Tool Registry (from CMPCT-008)

**File:** `codelet/tools/src/inject_summary.rs:49`

```rust
pub fn set_inject_summary_handler(session_id: Uuid, handler: Option<InjectSummaryHandler>) {
    // Inserts or removes handler from global HashMap
}
```

**Type:** `InjectSummaryHandler = Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>`

---

## 3. partition_for_compaction (core dependency)

**File:** `codelet/cli/src/session/system_reminders.rs:208`

```rust
pub fn partition_for_compaction(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    // Returns (latest system reminders, compactable messages)
}
```

- Finds LAST occurrence of each SystemReminderType
- Only latest version of each type goes to system_reminders
- Everything else goes to compactable

---

## 4. count_tokens (token estimation)

**File:** `codelet/common/src/token_estimator.rs:78`

```rust
pub fn count_tokens(text: &str) -> usize {
    ESTIMATOR.count_tokens(text)
}
```

- Free function wrapping global ESTIMATOR singleton
- Returns usize (will need cast to u64)

---

## 5. extract_message_text (message content extraction)

**File:** `codelet/cli/src/interactive_helpers.rs:135`

```rust
fn extract_message_text(message: &Message) -> String {
    // Handles User and Assistant variants
    // Extracts text from OneOrMany<UserContent/AssistantContent>
}
```

- Private function in interactive_helpers
- inject_summary_handler will need its own copy or make it public

---

## 6. BackgroundSession Inner Session

**File:** `codelet/napi/src/session_manager.rs:921`

```rust
pub inner: Arc<Mutex<codelet_cli::session::Session>>
```

- `Mutex` is `tokio::sync::Mutex` (async mutex, imported at line 39)
- Requires `tokio::task::block_in_place(|| runtime_handle.block_on(async { ... }))` from sync context

---

## 7. Bridge Handler block_in_place Pattern

**File:** `codelet/napi/src/session_manager.rs:5532-5547`

```rust
let bridge_handler: codelet_tools::BridgeHandler = Arc::new(move |request| {
    tokio::task::block_in_place(|| {
        runtime_handle.block_on(async {
            // async operations here
        })
    })
});
```

- `runtime_handle` captured as `tokio::runtime::Handle::current()` at line 5375
- `block_in_place` required because we're in a multi-threaded tokio runtime

---

## 8. Handler Registration Point

**File:** `codelet/napi/src/session_manager.rs:5363-5368`

```rust
// AMGR-001: Register SessionSearch handler for this session
let session_search_handler = crate::session_search_handler::create_handler(
    std::path::PathBuf::from(&session.project),
);
codelet_tools::set_session_search_handler(session.id, Some(session_search_handler));
```

inject_summary handler registration goes immediately after this block.

---

## 9. Handler Cleanup Point

**File:** `codelet/napi/src/session_manager.rs:5573-5578`

```rust
set_pause_handler(None);
// Clean up per-session handlers
codelet_tools::set_fspec_handler_for_session(session.id, None);
codelet_tools::set_session_search_handler(session.id, None);
codelet_tools::set_bridge_handler(None);
codelet_tools::remove_bridge_session_context(session.id);
```

inject_summary handler cleanup goes alongside these.

---

## 10. Session Structure (for handler access)

**File:** `codelet/cli/src/session/mod.rs:29-44`

```rust
pub struct Session {
    provider_manager: ProviderManager,  // private - use provider_manager() accessor
    pub messages: Vec<rig::message::Message>,
    pub turns: Vec<ConversationTurn>,
    pub token_tracker: TokenTracker,
}
```

- `provider_manager` is PRIVATE — access via `session.provider_manager()`
- `context_window()` returns `usize` — needs cast to `u64`
- `messages`, `turns`, `token_tracker` are all PUBLIC

---

## 11. TokenTracker Structure

**File:** `codelet/core/src/compaction/model.rs:57-75`

```rust
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cumulative_billed_input: u64,
    pub cumulative_billed_output: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    // ...
}
```

---

## Key Findings

1. **Handler pattern is well-established** — SessionSearch, Fspec, Bridge all follow the same Arc<dyn Fn + Send + Sync> pattern
2. **Async mutex requires block_in_place** — Cannot just use `.block_on()` alone in tokio context
3. **extract_message_text is private** — Need to either duplicate or make public for token counting
4. **context_window is usize** — Must capture at registration time and cast to u64
5. **partition_for_compaction returns cloned messages** — Safe to use from handler closure
