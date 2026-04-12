# BUG-128: Fix BRIDGE_HANDLER Per-Session Isolation

## Root Cause

`codelet/tools/src/bridge_handler.rs:61` defines a **process-global singleton**:

```rust
static BRIDGE_HANDLER: RwLock<Option<BridgeHandler>> = RwLock::new(None);
```

**Ironically**, the same file already has a **correctly per-session** map on lines 62-63:

```rust
static BRIDGE_SESSION_CONTEXTS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, Arc<BridgeSessionContext>>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));
```

The `BRIDGE_HANDLER` (command dispatch) uses the broken global singleton pattern while `BRIDGE_SESSION_CONTEXTS` (relay context) uses the correct per-session HashMap. When multiple sessions run concurrently:

1. Session A sets its bridge handler → works
2. Session B sets its bridge handler → **overwrites** Session A's handler
3. Session A's Bridge tool calls `execute_bridge_command()` → dispatches to Session B's handler
4. Session B finishes, calls `set_bridge_handler(None)` → Session A loses its handler entirely

## Architecture Context

The bridge system has two layers:

1. **`BRIDGE_HANDLER`** (broken) — Global command handler for connect/disconnect/list actions. Called by `BridgeToolFacadeWrapper::call()` via `execute_bridge_command()`.
2. **`BRIDGE_SESSION_CONTEXTS`** (correct) — Per-session relay context (broadcast receivers, input injectors). Keyed by `HashMap<Uuid, Arc<BridgeSessionContext>>`.

The handler itself doesn't do session-specific work — it just calls `handle_bridge_action(request.session_id, request.action)` which already uses the session_id from the request. So the fix is straightforward: make the handler per-session using the same HashMap pattern.

## Files to Modify

### 1. `codelet/tools/src/bridge_handler.rs` (PRIMARY)

**Current broken code (line 61):**
```rust
static BRIDGE_HANDLER: RwLock<Option<BridgeHandler>> = RwLock::new(None);
```

**Fix — convert to per-session HashMap:**
```rust
static BRIDGE_HANDLERS: Lazy<RwLock<HashMap<Uuid, BridgeHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
```

Note: `Lazy`, `RwLock`, `HashMap`, and `Uuid` are already imported in this file. `once_cell::sync::Lazy` is already used for `BRIDGE_SESSION_CONTEXTS`.

**Update `set_bridge_handler` (lines 69-73):**
```rust
// OLD:
pub fn set_bridge_handler(handler: Option<BridgeHandler>) {
    if let Ok(mut guard) = BRIDGE_HANDLER.write() {
        *guard = handler;
    }
}

// NEW:
pub fn set_bridge_handler(session_id: Uuid, handler: Option<BridgeHandler>) {
    if let Ok(mut guard) = BRIDGE_HANDLERS.write() {
        match handler {
            Some(h) => { guard.insert(session_id, h); }
            None => { guard.remove(&session_id); }
        }
    }
}
```

**Update `execute_bridge_command` (lines 123-144):**
```rust
// OLD:
pub fn execute_bridge_command(request: BridgeRequest) -> BridgeResult {
    let handler = match BRIDGE_HANDLER.read() {
        Ok(guard) => guard.clone(),
        Err(_) => { return BridgeResult { ... }; }
    };
    match handler {
        Some(h) => h(request),
        None => BridgeResult { ... },
    }
}

// NEW — use request.session_id to look up the correct handler:
pub fn execute_bridge_command(request: BridgeRequest) -> BridgeResult {
    let handler = match BRIDGE_HANDLERS.read() {
        Ok(guard) => guard.get(&request.session_id).cloned(),
        Err(_) => {
            return BridgeResult {
                success: false,
                message: "Failed to acquire bridge handler lock".to_string(),
                connections: None,
            };
        }
    };
    match handler {
        Some(h) => h(request),
        None => BridgeResult {
            success: false,
            message: "Bridge handler not configured - BridgeTool requires session context".to_string(),
            connections: None,
        },
    }
}
```

**Update `has_bridge_handler_for_session` (lines 149-161):**
```rust
// OLD:
pub fn has_bridge_handler_for_session(session_id: Uuid) -> bool {
    let has_handler = BRIDGE_HANDLER.read()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    let has_context = BRIDGE_SESSION_CONTEXTS.read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false);
    has_handler && has_context
}

// NEW — check the per-session handler map:
pub fn has_bridge_handler_for_session(session_id: Uuid) -> bool {
    let has_handler = BRIDGE_HANDLERS.read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false);
    let has_context = BRIDGE_SESSION_CONTEXTS.read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false);
    has_handler && has_context
}
```

### 2. `codelet/napi/src/session_manager.rs` (SET/CLEAR)

**SET handler (line 5144):**
```rust
// OLD:
codelet_tools::set_bridge_handler(Some(bridge_handler));

// NEW:
codelet_tools::set_bridge_handler(session.id, Some(bridge_handler));
```

**CLEAR handler (line 5446):**
```rust
// OLD:
codelet_tools::set_bridge_handler(None);

// NEW:
codelet_tools::set_bridge_handler(session.id, None);
```

### 3. `codelet/tools/src/facade/wrapper.rs` (NO CHANGES NEEDED)

The `BridgeToolFacadeWrapper::call()` at line ~1668 creates a `BridgeRequest` with `session_id: self.session_id` and calls `execute_bridge_command(request)`. Since `execute_bridge_command` already receives the request with session_id, and we're updating it to look up by `request.session_id`, **no changes needed here**.

The pre-check at line ~1650 (`has_bridge_handler_for_session(self.session_id)`) also already passes session_id. **No changes needed here either**.

### 4. `codelet/tools/src/lib.rs` (RE-EXPORTS)

Check the re-export. Current:
```rust
pub use bridge_handler::{
    set_bridge_handler, execute_bridge_command, has_bridge_handler_for_session,
    set_bridge_session_context, remove_bridge_session_context, get_bridge_session_context,
    handle_bridge_action, BridgeHandler, BridgeRequest, BridgeSessionContext,
    BroadcastReceiverFactory,
};
```

No changes to re-exports needed — just the signature change propagates.

## Callers Summary Table

| Function | File | Line | Action |
|----------|------|------|--------|
| `set_bridge_handler(Some(...))` | `session_manager.rs` | 5144 | Add `session.id` as first arg |
| `set_bridge_handler(None)` | `session_manager.rs` | 5446 | Add `session.id` as first arg |
| `execute_bridge_command(request)` | `facade/wrapper.rs` | ~1668 | **No change** — already has session_id in request |
| `has_bridge_handler_for_session(id)` | `facade/wrapper.rs` | ~1650 | **No change** — already receives session_id |

## Tests to Update

### Existing tests in `bridge_handler.rs` (lines 294-463)

**`with_clean_handler` (lines 300-305):**
```rust
// OLD:
fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
    set_bridge_handler(None);
    let result = f();
    set_bridge_handler(None);
    result
}

// NEW:
fn with_clean_handler<T>(session_id: Uuid, f: impl FnOnce() -> T) -> T {
    set_bridge_handler(session_id, None);
    let result = f();
    set_bridge_handler(session_id, None);
    result
}
```

All tests that use `set_bridge_handler(Some(...))` or `set_bridge_handler(None)` need session_id.

### Test files to update

- `codelet/tools/src/bridge_handler.rs` — inline tests (lines 294-463)
- `codelet/tools/src/facade/wrapper.rs` — test at ~line 2196 (`test_bridge_wrapper_uses_session_id_from_construction`)
- `codelet/tools/tests/tool_wrapper_session_association_test.rs` — test at ~line 332

### New tests to add

1. **Session isolation**: Register handlers for sessions A and B. Call `execute_bridge_command` with session A's ID. Assert only A's handler ran.
2. **Cleanup isolation**: Clear session A's handler. Assert `has_bridge_handler_for_session(session_b_id)` still returns true (if B's context is set).
3. **No handler for unknown session**: Call `execute_bridge_command` with unknown session_id. Assert error result.

## Checklist

- [ ] Modify `bridge_handler.rs`: Replace `RwLock<Option<BridgeHandler>>` with `Lazy<RwLock<HashMap<Uuid, BridgeHandler>>>`
- [ ] Update `set_bridge_handler` signature to accept `session_id: Uuid`
- [ ] Update `execute_bridge_command` to look up handler by `request.session_id`
- [ ] Update `has_bridge_handler_for_session` to check `BRIDGE_HANDLERS` map
- [ ] Update `session_manager.rs:5144`: Pass `session.id`
- [ ] Update `session_manager.rs:5446`: Pass `session.id`
- [ ] Update all existing tests in `bridge_handler.rs`
- [ ] Update tests in `facade/wrapper.rs` and `tool_wrapper_session_association_test.rs`
- [ ] Add new per-session isolation tests
- [ ] Verify `cargo build` passes
- [ ] Verify `cargo test` passes
- [ ] Verify `npm run build` passes
