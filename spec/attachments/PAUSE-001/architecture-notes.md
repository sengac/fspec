# Interactive Tool Pause for Browser Debugging - Architecture Notes

## Problem Statement

When using `WebSearchTool` with `headless: false`, the browser window is visible but the tool completes immediately after extracting content. Users cannot interactively inspect the page, use Chrome DevTools, or debug issues before the tool returns results to the agent.

## Design Principle: Per-Session State (NOT Global)

**CRITICAL**: All pause state MUST be per-session, following the exact same pattern as `isLoading`.

This is a **multi-session coding agent**. Multiple sessions can run concurrently. Global state would cause sessions to interfere with each other.

### The `isLoading` Pattern (to follow exactly)

```
BackgroundSession.status: AtomicU8          ← Per-session state
        ↓
session_get_status(session_id) → "running"  ← NAPI queries SPECIFIC session
        ↓
useRustSessionState hook
        ↓
React: isLoading = status === 'running'     ← Derived from session state
```

### The `isPaused` Pattern (identical structure)

```
BackgroundSession.status: AtomicU8          ← Paused = 3 (new enum variant)
BackgroundSession.pause_state: RwLock<...>  ← Per-session pause details
        ↓
session_get_status(session_id) → "paused"   ← NAPI queries SPECIFIC session
session_get_pause_state(session_id) → {...} ← NAPI queries pause details
        ↓
useRustSessionState hook (extended)
        ↓
React: isPaused = status === 'paused'       ← Derived from session state
       pauseInfo = getPauseState(sessionId)
```

## Architecture Components

### 1. SessionStatus Enum (extend existing)

**File: `codelet/napi/src/session_manager.rs`**

```rust
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    Paused = 3,  // NEW
}
```

### 2. BackgroundSession (add pause fields)

**File: `codelet/napi/src/session_manager.rs`**

```rust
pub struct BackgroundSession {
    // ... existing fields ...
    
    /// Current status (lock-free) - now includes Paused
    status: AtomicU8,
    
    /// Pause state details (per-session, NOT global)
    pause_state: RwLock<Option<PauseState>>,
    
    /// Condvar for blocking tool until user responds
    pause_response: (Mutex<Option<PauseResponse>>, Condvar),
}
```

### 3. Pause Types (in tools crate for tool access)

**File: `codelet/tools/src/tool_pause.rs`**

```rust
/// Kind of pause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    Continue,  // Press Enter to resume
    Confirm,   // Press Y to approve, N to deny
}

/// Tool's request to pause
#[derive(Debug, Clone)]
pub struct PauseRequest {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

/// User's response to pause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseResponse {
    Resumed,      // Enter pressed (Continue)
    Approved,     // Y pressed (Confirm)
    Denied,       // N pressed (Confirm)
    Interrupted,  // Esc pressed (any)
}

/// Pause state for UI display
#[derive(Debug, Clone)]
pub struct PauseState {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}
```

### 4. Handler Pattern (tools → session bridge)

The tools crate cannot depend on napi crate (circular dependency). So we use a handler pattern where the stream loop (which has session context) registers a handler that tools can call.

**File: `codelet/tools/src/tool_pause.rs`**

```rust
/// Handler type - captures session context in closure
/// Returns PauseResponse (blocks until user responds)
pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>;

/// Thread-local handler (set per agent execution, not global singleton)
thread_local! {
    static PAUSE_HANDLER: RefCell<Option<PauseHandler>> = RefCell::new(None);
}

/// Set handler for current thread (called by stream loop)
pub fn set_pause_handler(handler: Option<PauseHandler>) {
    PAUSE_HANDLER.with(|h| *h.borrow_mut() = handler);
}

/// Request pause and block until user responds
/// If no handler set, returns Resumed (no-op)
pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    PAUSE_HANDLER.with(|h| {
        if let Some(handler) = h.borrow().as_ref() {
            handler(request)
        } else {
            PauseResponse::Resumed
        }
    })
}
```

### 5. Stream Loop Integration

**File: `codelet/napi/src/session_manager.rs` (in agent loop)**

```rust
// Before running agent, set pause handler with session context
let session_for_pause = Arc::clone(&session);
set_pause_handler(Some(Arc::new(move |request: PauseRequest| {
    // 1. Store pause state in session
    session_for_pause.set_pause_state(Some(request.into()));
    
    // 2. Set status to Paused
    session_for_pause.set_status(SessionStatus::Paused);
    
    // 3. Block on condvar until UI signals response
    let response = session_for_pause.wait_for_pause_response();
    
    // 4. Clear pause state
    session_for_pause.set_pause_state(None);
    
    // 5. Set status back to Running
    session_for_pause.set_status(SessionStatus::Running);
    
    response
})));

// Run agent...

// After agent completes, clear handler
set_pause_handler(None);
```

### 6. BackgroundSession Methods

```rust
impl BackgroundSession {
    /// Set pause state (called by handler)
    pub fn set_pause_state(&self, state: Option<PauseState>) {
        *self.pause_state.write().unwrap() = state;
    }
    
    /// Get pause state (called by NAPI)
    pub fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.read().unwrap().clone()
    }
    
    /// Block until UI signals response
    pub fn wait_for_pause_response(&self) -> PauseResponse {
        let (lock, cvar) = &self.pause_response;
        let mut response = lock.lock().unwrap();
        while response.is_none() {
            response = cvar.wait(response).unwrap();
        }
        response.take().unwrap()
    }
    
    /// Signal resume (called by NAPI from UI)
    pub fn signal_pause_resume(&self) {
        let (lock, cvar) = &self.pause_response;
        *lock.lock().unwrap() = Some(PauseResponse::Resumed);
        cvar.notify_one();
    }
    
    /// Signal confirm (called by NAPI from UI)
    pub fn signal_pause_confirm(&self, approved: bool) {
        let (lock, cvar) = &self.pause_response;
        *lock.lock().unwrap() = Some(if approved {
            PauseResponse::Approved
        } else {
            PauseResponse::Denied
        });
        cvar.notify_one();
    }
    
    /// Signal interrupt (called by NAPI from UI)
    pub fn signal_pause_interrupt(&self) {
        let (lock, cvar) = &self.pause_response;
        *lock.lock().unwrap() = Some(PauseResponse::Interrupted);
        cvar.notify_one();
    }
}
```

### 7. NAPI Exports

**File: `codelet/napi/src/session_manager.rs`**

```rust
#[napi(object)]
pub struct NapiPauseState {
    pub kind: String,  // "continue" or "confirm"
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

#[napi]
pub fn session_get_pause_state(session_id: String) -> Result<Option<NapiPauseState>> {
    let session = get_session(&session_id)?;
    Ok(session.get_pause_state().map(|s| NapiPauseState {
        kind: match s.kind {
            PauseKind::Continue => "continue".to_string(),
            PauseKind::Confirm => "confirm".to_string(),
        },
        tool_name: s.tool_name,
        message: s.message,
        details: s.details,
    }))
}

#[napi]
pub fn session_pause_resume(session_id: String) -> Result<()> {
    let session = get_session(&session_id)?;
    session.signal_pause_resume();
    Ok(())
}

#[napi]
pub fn session_pause_confirm(session_id: String, approved: bool) -> Result<()> {
    let session = get_session(&session_id)?;
    session.signal_pause_confirm(approved);
    Ok(())
}
```

### 8. React Hook Extension

**File: `src/tui/hooks/useRustSessionState.ts`**

```typescript
export interface PauseInfo {
  kind: 'continue' | 'confirm';
  toolName: string;
  message: string;
  details?: string;
}

export interface RustSessionSnapshot {
  status: string;
  isLoading: boolean;
  isPaused: boolean;       // NEW: status === 'paused'
  pauseInfo: PauseInfo | null;  // NEW: from sessionGetPauseState
  model: SessionModel | null;
  tokens: SessionTokens;
  isDebugEnabled: boolean;
  version: number;
}

function fetchFreshSnapshot(sessionId: string, version: number): RustSessionSnapshot {
  const status = rustStateSource.getStatus(sessionId);
  const isLoading = status === 'running';
  const isPaused = status === 'paused';
  const pauseInfo = isPaused ? rustStateSource.getPauseState(sessionId) : null;
  
  return {
    status,
    isLoading,
    isPaused,
    pauseInfo,
    model: rustStateSource.getModel(sessionId),
    tokens: rustStateSource.getTokens(sessionId),
    isDebugEnabled: rustStateSource.getDebugEnabled(sessionId),
    version,
  };
}
```

### 9. InputTransition Component

**File: `src/tui/components/InputTransition.tsx`**

```tsx
// In loading phase, check isPaused
if (animationPhase === 'loading') {
  if (isPaused && pauseInfo) {
    return <PauseIndicator pauseInfo={pauseInfo} />;
  }
  return <Text dimColor>{currentThinkingText}</Text>;
}
```

## Data Flow Summary

```
1. Tool calls pause_for_user(request)
        ↓
2. Handler (with session context) is invoked
        ↓
3. Handler stores state in BackgroundSession.pause_state
        ↓
4. Handler sets BackgroundSession.status = Paused
        ↓
5. Handler blocks on BackgroundSession.pause_response condvar
        ↓
6. React hook polls session_get_status() → "paused"
        ↓
7. React hook calls session_get_pause_state() → pause details
        ↓
8. UI renders PauseIndicator
        ↓
9. User presses Enter/Y/N/Esc
        ↓
10. UI calls session_pause_resume() / session_pause_confirm()
        ↓
11. Condvar is signaled, handler unblocks
        ↓
12. Handler clears pause_state, sets status = Running
        ↓
13. Tool continues execution
```

## Why Thread-Local Handler (not global static)?

The handler uses `thread_local!` instead of a global static because:

1. **Prevents session interference**: Each agent runs in its own thread/task. Thread-local ensures each agent's pause handler is isolated.

2. **Matches execution model**: Tools execute in the context of a specific agent task. Thread-local naturally scopes the handler to that execution context.

3. **No mutex contention**: Thread-local access is lock-free, unlike RwLock for global static.

## File Changes Summary

| File | Change |
|------|--------|
| `codelet/napi/src/session_manager.rs` | Add `Paused` to SessionStatus, add `pause_state` and `pause_response` to BackgroundSession, add NAPI exports |
| `codelet/tools/src/tool_pause.rs` | Types + thread-local handler mechanism |
| `codelet/tools/src/lib.rs` | Export tool_pause |
| `codelet/common/src/web_search.rs` | Add `pause: bool` to action variants |
| `codelet/tools/src/web_search.rs` | Call `pause_for_user()` when pause=true |
| `src/tui/hooks/useRustSessionState.ts` | Add `isPaused`, `pauseInfo` |
| `src/tui/components/InputTransition.tsx` | Render PauseIndicator when paused |
