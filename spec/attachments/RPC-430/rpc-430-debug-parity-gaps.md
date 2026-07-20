# RPC-430: /debug Command Parity Gaps — Detailed Specification

## Background

The Rust TUI port of the `/debug` slash command (RPC-055) has four critical gaps compared to the TypeScript TUI. This document specifies the fixes needed for full parity.

## Gap 1: Wrong Debug Directory Path 🔴 CRITICAL

### Current (Wrong)
```rust
// dispatch_slash_debug.rs:44
let debug_dir = std::env::var("FSPEC_DEBUG_DIR")
    .unwrap_or_else(|_| ".fspec/debug".to_string());
```

This resolves to `.fspec/debug` relative to CWD — meaning debug files are saved inside the project directory.

### Expected (TypeScript)
```typescript
// AgentView.tsx:2707
const debugDir = getFspecUserDir(); // → ~/.fspec
```

TypeScript uses `~/.fspec` (user home directory). Debug files are saved to `~/.fspec/debug/`.

### Fix Required
Replace the fallback from `.fspec/debug` to `~/.fspec` using `dirs::home_dir()` or `std::env::var("HOME")`. The `FSPEC_DEBUG_DIR` env var override should remain for testing flexibility.

**Implementation approach:**
```rust
use std::path::PathBuf;

fn resolve_debug_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("FSPEC_DEBUG_DIR") {
        return PathBuf::from(custom);
    }
    // Use HOME env var (works on all platforms)
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".fspec");
    }
    // Fallback (should never happen on real systems)
    PathBuf::from(".fspec")
}
```

---

## Gap 2: No Pre-Session Toggle 🔴 CRITICAL

### Current (Rust)
```rust
// dispatch_slash_debug.rs:37-39
let Some(session_id) = self.agent_view_store.current_session().cloned() else {
    return; // Silent no-op
};
```

When there's no active session, `/debug` silently does nothing.

### Expected (TypeScript)
```typescript
// AgentView.tsx:2713-2715
if (currentSessionId) {
  result = await sessionToggleDebug(currentSessionId, debugDir);
} else {
  result = toggleDebug(debugDir); // Global pre-session toggle
}
```

TypeScript supports toggling debug globally before any session exists. This global state is then synced to the first session when it's created.

### Fix Required
Add a `pre_session_debug_enabled` field to `App` (boolean) that tracks the global pre-session debug state. When `/debug` is called with no session:
1. Toggle `app.pre_session_debug_enabled`
2. Call `backend.set_debug_enabled` is NOT available globally — instead, store the state locally
3. Emit a scrollback notice with the toggled state

When a session is created, check `app.pre_session_debug_enabled` and if true, call `backend.set_debug_enabled(session_id, true)`.

**Implementation approach:**
- Add `pre_session_debug_enabled: bool` to `App` struct
- In `handle_slash_debug`, when no session exists, toggle the flag and emit notice
- In `refresh_session_chrome` (or session creation dispatch), propagate the flag to the new session

---

## Gap 3: No Debug Hydration on Session Attach 🔴 CRITICAL

### Current (Rust)
```rust
// dispatch_resume_search_views.rs:100-177
pub(crate) fn handle_attach_to_session(&mut self, session: SessionId) {
    // ... focus/append session ...
    self.refresh_session_chrome(session.clone());
    // NO debug state hydration
}
```

When attaching to an existing session (via `/resume` or session cycling), the debug state is NOT fetched from the backend. The `[DEBUG]` badge may not appear even if debug is enabled in Rust.

### Expected (TypeScript)
```typescript
// globalSessionStreamManager.ts:583-607
export function applyPendingDebugState(sessionId: string): void {
  const pendingState = manager.getPendingDebugState(sessionId);
  if (pendingState) {
    useSessionStore.getState().setDebugState(sessionId, pendingState.isDebugEnabled);
    return;
  }
  // Rust ground-truth fallback:
  try {
    const rustEnabled = sessionGetDebugEnabled(sessionId);
    useSessionStore.getState().setDebugState(sessionId, rustEnabled);
  } catch { /* default false */ }
}
```

TypeScript calls `sessionGetDebugEnabled(sessionId)` as a ground-truth fallback on every session activation.

### Fix Required
Add debug state hydration to `refresh_session_chrome()` or create a dedicated `hydrate_debug_state()` method. This should:
1. Call `backend.get_debug_enabled(session_id)` 
2. Store the result in `agent_view_store.set_debug_enabled(session_id, enabled)`
3. Trigger a render

**Implementation approach:**
- In `dispatch_session_chrome.rs::refresh_session_chrome()`, add a spawned task for `backend.get_debug_enabled(session_id)`
- On success, dispatch `Action::DebugEnabledLoaded(session_id, bool)` which stores the value
- Alternatively, add the hydration directly in `handle_attach_to_session()`

---

## Gap 4: No Debug Propagation on Session Creation ⚠️ WARNING

### Current (Rust)
No equivalent of the TypeScript path in `AgentView.tsx:1846-1856`:
```typescript
if (isDebugEnabled) {
  await sessionUpdateDebugMetadata(activeSessionId);
  sessionSetDebugEnabled(activeSessionId, true);
}
```

### Fix Required
When a session is created, check `app.pre_session_debug_enabled` (from Gap 2 fix). If true, call `backend.set_debug_enabled(session_id, true)` to propagate the pre-session debug state to the new session.

**Implementation approach:**
- In `dispatch_create_session_dialog.rs`, after `SessionCreated` is dispatched, check `app.pre_session_debug_enabled`
- If true, spawn `backend.set_debug_enabled(session_id, true)`

---

## Files to Modify

1. `codelet/fspec-tui/src/app/dispatch_slash_debug.rs` — Fix debug directory + pre-session toggle
2. `codelet/fspec-tui/src/app/state.rs` — Add `pre_session_debug_enabled: bool` field
3. `codelet/fspec-tui/src/app/dispatch_session_chrome.rs` — Add debug hydration on session attach
4. `codelet/fspec-tui/src/app/dispatch_create_session_dialog.rs` — Add debug propagation on session creation
5. `codelet/fspec-tui/src/components/mod.rs` — Add `DebugEnabledLoaded` action variant (if needed)

## Test Files to Create/Modify

1. Tests for debug directory resolution (`~/.fspec` default, `$FSPEC_DEBUG_DIR` override)
2. Tests for pre-session toggle (toggle with no session, then create session, verify propagation)
3. Tests for debug hydration on session attach
4. Tests for debug propagation on session creation
