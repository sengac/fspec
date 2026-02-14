# AST Research: Session Association Architecture

## Summary

Analysis of the current Fspec and Bridge tool wrapper implementations to understand the session lookup mechanism and identify changes needed for session-at-construction architecture.

## Key Findings

### 1. FspecToolFacadeWrapper Structure (wrapper.rs:354)

```rust
pub struct FspecToolFacadeWrapper {
    /// The underlying facade providing name, schema, and param mapping
    facade: BoxedFspecToolFacade,
    // NO session_id field - this is the problem
}
```

**Location**: `codelet/tools/src/facade/wrapper.rs`

**Issue**: The wrapper has no `session_id` field, so it must look up the current session at call time.

### 2. BridgeToolFacadeWrapper Structure (wrapper.rs:710-720)

```rust
pub struct BridgeToolFacadeWrapper {
    facade: BoxedBridgeToolFacade,
    // NO session_id field - same problem as Fspec
}
```

**Location**: `codelet/tools/src/facade/wrapper.rs`

### 3. Registration Functions (fspec_registration.rs)

```rust
pub fn claude_fspec_tool() -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ClaudeFspecFacade))
}
// Similar for gemini_fspec_tool(), openai_fspec_tool(), zai_fspec_tool()
```

**Location**: `codelet/tools/src/facade/fspec_registration.rs`

**Issue**: No session_id parameter - tools are created without session context.

### 4. Thread-Local Storage for Fspec (fspec_handler.rs:91-114)

```rust
thread_local! {
    static CURRENT_FSPEC_SESSION: RefCell<Option<Uuid>> = const { RefCell::new(None) };
}

pub fn set_current_fspec_session(session_id: Option<Uuid>) {
    CURRENT_FSPEC_SESSION.with(|cell| {
        *cell.borrow_mut() = session_id;
    });
}

pub fn get_current_fspec_session() -> Option<Uuid> {
    CURRENT_FSPEC_SESSION.with(|cell| *cell.borrow())
}
```

**Location**: `codelet/tools/src/fspec_handler.rs`

**Issue**: Thread-local storage doesn't survive async boundaries when tasks migrate between threads.

### 5. Global RwLock for Bridge (bridge_handler.rs:62-84)

```rust
static CURRENT_BRIDGE_SESSION: RwLock<Option<Uuid>> = RwLock::new(None);

pub fn set_current_bridge_session(session_id: Option<Uuid>) {
    if let Ok(mut guard) = CURRENT_BRIDGE_SESSION.write() {
        *guard = session_id;
    }
}

pub fn get_current_bridge_session() -> Option<Uuid> {
    CURRENT_BRIDGE_SESSION
        .read()
        .ok()
        .and_then(|guard| *guard)
}
```

**Location**: `codelet/tools/src/bridge_handler.rs`

**Issue**: Global RwLock is even worse - can be overwritten by any concurrent session.

### 6. Provider create_rig_agent() Signatures

| Provider | File | Line | Signature |
|----------|------|------|-----------|
| Claude | claude.rs | 298 | `pub fn create_rig_agent(&self, preamble: Option<&str>, thinking_config: Option<Value>)` |
| Gemini | gemini.rs | 100 | `pub fn create_rig_agent(&self, preamble: Option<&str>, thinking_config: Option<Value>)` |
| OpenAI | openai.rs | 116 | `pub fn create_rig_agent(&self, preamble: Option<&str>, thinking_config: Option<Value>)` |
| ZAI | zai.rs | 191 | `pub fn create_rig_agent(&self, preamble: Option<&str>, thinking_config: Option<Value>)` |
| Codex | codex/mod.rs | 121 | `pub fn create_rig_agent(&self, preamble: Option<&str>, thinking_config: Option<Value>)` |

**Issue**: None of these accept `session_id` - so tools are built without session context.

### 7. Agent Building in NAPI (session_manager.rs:4202-4222)

```rust
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $session:expr, $output:expr, $thinking:expr) => {
        match $inner.provider_manager_mut().$getter() {
            Ok(provider) => {
                let agent = codelet_core::RigAgent::with_default_depth(
                    provider.create_rig_agent(None, $thinking.clone())
                );
                // ... 
            }
        }
    };
}
```

**Location**: `codelet/napi/src/session_manager.rs`

**Key observation**: `session.id` is available in scope but not passed to `create_rig_agent()`.

### 8. Session Context Setup (session_manager.rs:4379-4383)

```rust
// REFAC-008-FIX: Use per-session handler storage to prevent race conditions
// when multiple sessions run concurrently. Set current session before setting
// handler so the handler is associated with this session.
codelet_tools::set_current_fspec_session(Some(session.id));
codelet_tools::set_fspec_handler_for_session(session.id, Some(fspec_handler));
```

**Location**: `codelet/napi/src/session_manager.rs`

**Issue**: This sets the thread-local AFTER the tools were already created (without session_id).

## Required Changes

### Struct Changes
1. Add `session_id: Uuid` field to `FspecToolFacadeWrapper`
2. Add `session_id: Uuid` field to `BridgeToolFacadeWrapper`

### Function Signature Changes
1. `claude_fspec_tool(session_id: Uuid)` → returns wrapper with session_id stored
2. `gemini_fspec_tool(session_id: Uuid)` → same
3. `openai_fspec_tool(session_id: Uuid)` → same
4. `zai_fspec_tool(session_id: Uuid)` → same
5. Same pattern for `*_bridge_tool()` functions

### Provider Changes
1. `create_rig_agent(session_id: Uuid, preamble: Option<&str>, thinking_config: Option<Value>)`
2. Pass session_id to tool registration functions

### Caller Updates
1. NAPI `run_with_provider!` macro: pass `session.id`
2. CLI single-shot mode: generate `Uuid::new_v4()`
3. Tests: can use `Uuid::nil()` for non-handler tests

### Deprecations
1. Remove `set_current_fspec_session()` / `get_current_fspec_session()`
2. Remove `set_current_bridge_session()` / `get_current_bridge_session()`
3. Keep `set_fspec_handler_for_session()` - still needed
4. Keep `set_bridge_session_context()` - still needed
