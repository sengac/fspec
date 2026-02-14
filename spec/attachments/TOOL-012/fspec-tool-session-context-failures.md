# Fspec Tool Session Association Fix

## Problem Summary

The Fspec tool intermittently fails with "Fspec handler not configured" because it relies on **thread-local "current session" state** to look up handlers. This is fundamentally broken because:

1. Thread-local state can be polluted by other sessions
2. The "current" session concept doesn't survive async boundaries reliably
3. Claude Code CLI doesn't set up thread-local state at all

## Root Cause

The current architecture uses a two-step lookup:
```rust
// Step 1: Get "current" session from thread-local storage
let session_id = get_current_fspec_session()?;  // BROKEN

// Step 2: Look up handler for that session
let handler = FSPEC_HANDLERS.get(&session_id)?;
```

This fails because `get_current_fspec_session()` returns `None` or stale data.

## The Fix: Session Association at Construction

**Tools should be constructed WITH their session ID, not look it up at call time.**

### Current (Broken) Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Tool Creation (no session context):                         │
│   claude_fspec_tool() → FspecToolFacadeWrapper              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ Tool Call (tries to find session):                          │
│   1. get_current_fspec_session() → thread-local lookup      │
│   2. FSPEC_HANDLERS.get(&session_id) → handler lookup       │
│   3. handler(request)                                       │
│                                                             │
│   FAILS: Thread-local state is unreliable                   │
└─────────────────────────────────────────────────────────────┘
```

### Fixed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Tool Creation (with session context):                       │
│   claude_fspec_tool(session_id) → FspecToolFacadeWrapper {  │
│       session_id: Uuid,  // Stored at construction          │
│       facade: ...,                                          │
│   }                                                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ Tool Call (uses associated session):                        │
│   1. self.session_id → already known                        │
│   2. FSPEC_HANDLERS.get(&self.session_id) → handler lookup  │
│   3. handler(request)                                       │
│                                                             │
│   WORKS: Session ID travels with the tool instance          │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Changes

### 1. FspecToolFacadeWrapper (codelet/tools/src/facade/wrapper.rs)

```rust
pub struct FspecToolFacadeWrapper {
    facade: BoxedFspecToolFacade,
    session_id: Uuid,  // ADD: Associated session
}

impl FspecToolFacadeWrapper {
    pub fn new(facade: BoxedFspecToolFacade, session_id: Uuid) -> Self {
        Self { facade, session_id }
    }
}

impl Tool for FspecToolFacadeWrapper {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let internal_params = self.facade.map_params(args.0)?;

        // Use self.session_id directly - no thread-local lookup
        let result = execute_fspec_command_for_session(
            self.session_id,  // Pass session explicitly
            FspecRequest {
                command: internal_params.command,
                args_json: internal_params.args,
                project_root: internal_params.project_root,
                provider: self.facade.provider().to_string(),
            },
        );

        // ... handle result
    }
}
```

### 2. Registration Functions (codelet/tools/src/facade/fspec_registration.rs)

```rust
pub fn claude_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ClaudeFspecFacade), session_id)
}

pub fn fspec_tool_for_provider(provider: &str, session_id: Uuid) -> Option<FspecToolFacadeWrapper> {
    match provider {
        "claude" => Some(claude_fspec_tool(session_id)),
        "gemini" => Some(gemini_fspec_tool(session_id)),
        // ...
    }
}
```

### 3. Handler Lookup (codelet/tools/src/fspec_handler.rs)

```rust
// ADD: Direct session lookup (no "current" session concept)
pub fn execute_fspec_command_for_session(session_id: Uuid, request: FspecRequest) -> FspecResult {
    let handler = match FSPEC_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => return FspecResult::lock_error(),
    };

    match handler {
        Some(h) => h(request),
        None => FspecResult::no_handler_error(session_id),
    }
}

// DEPRECATE: Remove thread-local current session
// - set_current_fspec_session()
// - get_current_fspec_session()
// - CURRENT_FSPEC_SESSION thread_local
```

### 4. Agent Builder (codelet/providers/src/claude.rs)

```rust
// Tools are now created with session context
pub fn build_agent(session_id: Uuid, ...) -> Agent {
    client.agent(model)
        .tool(claude_fspec_tool(session_id))  // Pass session
        .tool(claude_bridge_tool(session_id)) // Pass session
        // ...
        .build()
}
```

## Apply Same Pattern to Bridge Tool

The `BridgeToolFacadeWrapper` has the same problem and should be fixed identically:

```rust
pub struct BridgeToolFacadeWrapper {
    facade: BoxedBridgeToolFacade,
    session_id: Uuid,  // ADD
}

pub fn claude_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(ClaudeBridgeFacade), session_id)
}
```

## Code Locations to Modify

| File | Change |
|------|--------|
| `codelet/tools/src/facade/wrapper.rs` | Add `session_id` field to wrappers, use in `call()` |
| `codelet/tools/src/facade/fspec_registration.rs` | Add `session_id` parameter to all registration functions |
| `codelet/tools/src/facade/bridge_registration.rs` | Add `session_id` parameter to all registration functions |
| `codelet/tools/src/fspec_handler.rs` | Add `execute_fspec_command_for_session()`, deprecate thread-local |
| `codelet/tools/src/bridge_handler.rs` | Add `execute_bridge_command_for_session()`, deprecate thread-local |
| `codelet/providers/src/claude.rs` | Pass session_id when creating tools |
| `codelet/providers/src/gemini.rs` | Pass session_id when creating tools |
| `codelet/providers/src/openai.rs` | Pass session_id when creating tools |
| `codelet/napi/src/session_manager.rs` | Pass session_id to tool creation |

## Benefits

1. **Deterministic**: Session ID is known at tool creation, not discovered at call time
2. **No thread-local state**: Eliminates race conditions and async boundary issues
3. **Explicit dependencies**: Tool's session association is visible in the type
4. **Testable**: Easy to create tools with mock session IDs for testing
5. **Works everywhere**: Claude Code CLI, Codelet TUI, any other consumer

## Test Cases

1. **Tool created with session A calls handler for session A**
2. **Multiple tools with different sessions call correct handlers**
3. **Tool call works regardless of thread**
4. **Tool call works across async boundaries**
5. **Handler not found returns clear error with session ID**
