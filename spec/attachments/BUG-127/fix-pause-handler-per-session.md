# BUG-127: Fix PAUSE_HANDLER Per-Session Isolation

## Root Cause

`codelet/tools/src/tool_pause.rs:66` defines a **process-global singleton**:

```rust
static PAUSE_HANDLER: RwLock<Option<PauseHandler>> = RwLock::new(None);
```

When multiple `BackgroundSession` instances run concurrently, each session's agent loop calls `set_pause_handler(Some(...))` at `session_manager.rs:4789` which **overwrites** the handler for all other sessions. This causes:

1. `pause_for_user()` calls routing to the **wrong session's** UI state (pause request appears in Session B when Session A's tool called it)
2. Blocklist permission prompts appearing in wrong sessions
3. WebSearch `pause: true` interactions routing to wrong sessions

## Architecture Context

Pause flow:
```
Tool (e.g., WebSearch, Blocklist) → pause_for_user(PauseRequest) → PAUSE_HANDLER global
                                                                          ↓
                                                          handler closure (captures session_for_pause)
                                                                          ↓
                                                    session.set_pause_state(Some(state))
                                                    session.set_status(SessionStatus::Paused)
                                                    session.wait_for_pause_response()  // BLOCKS
                                                                          ↓
                                                    TUI detects Paused status, shows prompt
                                                    User responds → session.resume_from_pause(response)
                                                                          ↓
                                                    wait_for_pause_response() unblocks → returns response
```

The handler closure itself correctly captures a specific session. But the **global slot** means the wrong closure runs.

## Files to Modify

### 1. `codelet/tools/src/tool_pause.rs` (PRIMARY)

**Current broken code (line 66):**
```rust
static PAUSE_HANDLER: RwLock<Option<PauseHandler>> = RwLock::new(None);
```

**Fix — convert to per-session HashMap:**
```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;
use uuid::Uuid;

static PAUSE_HANDLERS: Lazy<RwLock<HashMap<Uuid, PauseHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
```

**Update `set_pause_handler` (lines 68-72):**
```rust
// OLD:
pub fn set_pause_handler(handler: Option<PauseHandler>) {
    if let Ok(mut guard) = PAUSE_HANDLER.write() {
        *guard = handler;
    }
}

// NEW:
pub fn set_pause_handler(session_id: Uuid, handler: Option<PauseHandler>) {
    if let Ok(mut guard) = PAUSE_HANDLERS.write() {
        match handler {
            Some(h) => { guard.insert(session_id, h); }
            None => { guard.remove(&session_id); }
        }
    }
}
```

**Update `pause_for_user` (lines 74-84):**
```rust
// OLD:
pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    let handler = match PAUSE_HANDLER.read() {
        Ok(guard) => guard.clone(),
        Err(_) => return PauseResponse::Resumed,
    };
    match handler {
        Some(h) => h(request),
        None => PauseResponse::Resumed,
    }
}

// NEW:
pub fn pause_for_user(session_id: Uuid, request: PauseRequest) -> PauseResponse {
    let handler = match PAUSE_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => return PauseResponse::Resumed,
    };
    match handler {
        Some(h) => h(request),
        None => PauseResponse::Resumed,
    }
}
```

**Update `has_pause_handler` (lines 86-90):**
```rust
// OLD:
pub fn has_pause_handler() -> bool {
    PAUSE_HANDLER.read()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

// NEW:
pub fn has_pause_handler(session_id: Uuid) -> bool {
    PAUSE_HANDLERS.read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}
```

### 2. `codelet/napi/src/session_manager.rs` (SET/CLEAR)

**SET handler (line 4789):**
```rust
// OLD:
set_pause_handler(Some(pause_handler));

// NEW:
set_pause_handler(session.id, Some(pause_handler));
```

**CLEAR handler (line 5435):**
```rust
// OLD:
set_pause_handler(None);

// NEW:
set_pause_handler(session.id, None);
```

**Import update (line 39):**
```rust
// Verify the import still works — it imports the function by name:
use codelet_tools::tool_pause::{PauseKind, PauseRequest, PauseResponse, PauseState, set_pause_handler, PauseHandler};
```

### 3. `codelet/tools/src/web_search.rs` (CALLERS of `pause_for_user`)

The `WebSearchTool` (or its wrapper) needs to have a `session_id` field. Check the struct definition.

**Open page pause (line ~644-655):**
```rust
// OLD:
let response = pause_for_user(PauseRequest {
    kind: PauseKind::Continue,
    tool_name: "WebSearch".to_string(),
    message: format!("Page loaded: {url}"),
    details: None,
});

// NEW:
let response = pause_for_user(self.session_id, PauseRequest {
    kind: PauseKind::Continue,
    tool_name: "WebSearch".to_string(),
    message: format!("Page loaded: {url}"),
    details: None,
});
```

**Screenshot pause (line ~720-731):** Same pattern, add `self.session_id`.

**FindInPage pause (line ~802-813):** Same pattern, add `self.session_id`.

**IMPORTANT**: Verify the WebSearch wrapper has `session_id`. The `WebSearchToolFacadeWrapper` should have it since TOOL-012 required all tools to store session_id at construction time.

### 4. `codelet/tools/src/blocklist/middleware.rs` (CALLERS of `pause_for_user`)

**check_bash_command (line ~165-170):**
```rust
// OLD:
let response = pause_for_user(PauseRequest {
    kind: PauseKind::Triple,
    tool_name: "Bash".to_string(),
    message: result.reason.unwrap_or_else(|| "Command requires approval".to_string()),
    details: Some(command.to_string()),
});

// NEW — need session_id parameter:
let response = pause_for_user(session_id, PauseRequest { ... });
```

**check_file_path (line ~228-233):** Same pattern.

**IMPORTANT**: The blocklist middleware functions need `session_id` threaded through. Check the function signatures of `check_bash_command` and `check_file_path`. They may need an additional `session_id: Uuid` parameter, which must be propagated from the tool wrapper that calls them.

### 5. `codelet/tools/src/lib.rs` (RE-EXPORTS)

Verify the re-export at the crate level:
```rust
pub use tool_pause::{set_pause_handler, pause_for_user, has_pause_handler, ...};
```

## Reference Pattern

Use `FSPEC_HANDLERS` in `codelet/tools/src/fspec_handler.rs`:
```rust
static FSPEC_HANDLERS: Lazy<RwLock<HashMap<Uuid, FspecHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
```

## Callers Summary Table

| Function | File | Line | Action |
|----------|------|------|--------|
| `set_pause_handler(Some(...))` | `session_manager.rs` | 4789 | Add `session.id` as first arg |
| `set_pause_handler(None)` | `session_manager.rs` | 5435 | Add `session.id` as first arg |
| `pause_for_user(req)` | `web_search.rs` | ~644 | Add `self.session_id` as first arg |
| `pause_for_user(req)` | `web_search.rs` | ~720 | Add `self.session_id` as first arg |
| `pause_for_user(req)` | `web_search.rs` | ~802 | Add `self.session_id` as first arg |
| `pause_for_user(req)` | `blocklist/middleware.rs` | ~165 | Thread `session_id` through function |
| `pause_for_user(req)` | `blocklist/middleware.rs` | ~228 | Thread `session_id` through function |

## Tests to Update

### Existing tests in `tool_pause.rs` (lines 92-265)

All tests use `set_pause_handler(None)` and `pause_for_user(request)`. Update to pass `Uuid::new_v4()` or a test session_id.

### Existing integration tests

Search for test files:
- `codelet/tools/tests/tool_pause_test.rs`
- `codelet/cli/tests/stream_loop_pause_test.rs`
- `codelet/napi/tests/pause_integration_test.rs`

Each will need session_id parameters added.

### New tests to add

1. **Session isolation**: Register handlers for sessions A and B. Call `pause_for_user(session_a_id, ...)`. Assert only session A's handler was invoked.
2. **Cleanup isolation**: Clear session A's handler. Assert session B's handler still works.
3. **No handler for session**: Call `pause_for_user(unknown_session_id, ...)`. Assert returns `PauseResponse::Resumed`.

## Checklist

- [ ] Modify `tool_pause.rs`: Replace `RwLock<Option<...>>` with `Lazy<RwLock<HashMap<Uuid, ...>>>`
- [ ] Update `set_pause_handler` signature to accept `session_id: Uuid`
- [ ] Update `pause_for_user` signature to accept `session_id: Uuid`
- [ ] Update `has_pause_handler` signature to accept `session_id: Uuid`
- [ ] Update `session_manager.rs:4789`: Pass `session.id`
- [ ] Update `session_manager.rs:5435`: Pass `session.id`
- [ ] Update `web_search.rs` (3 call sites): Pass `self.session_id`
- [ ] Update `blocklist/middleware.rs` (2 call sites): Thread `session_id` through
- [ ] Update blocklist middleware function signatures if needed
- [ ] Update all existing tests in `tool_pause.rs`
- [ ] Update tests in `tool_pause_test.rs`, `stream_loop_pause_test.rs`, `pause_integration_test.rs`
- [ ] Add new per-session isolation tests
- [ ] Verify `cargo build` passes
- [ ] Verify `cargo test` passes
- [ ] Verify `npm run build` passes
