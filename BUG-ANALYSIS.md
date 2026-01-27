# Bug Analysis: Shift+Left Navigation from New Agent Session

## Problem
When creating a new agent session and immediately pressing Shift+Left, navigation goes directly to BoardView instead of checking the session's position in the order and navigating to the correct previous session.

## Root Cause

The issue is a timing gap between session creation and active session tracking:

1. **Session Creation** (`AgentView.tsx:2543`):
   ```typescript
   await sessionManagerCreateWithId(activeSessionId, modelPath, project, sessionName);
   ```
   This creates the session in Rust SessionManager but **does NOT** set it as the active session.

2. **Active Session Setting** (`session_manager.rs:3900`):
   ```rust
   manager.set_active_session(uuid);
   ```
   This only happens later when `sessionAttach` is called during streaming setup.

3. **Navigation Logic Dependency** (`session_manager.rs:3311`):
   ```rust
   let active = self.active_session_id.read().expect("active_session lock poisoned");
   ```
   The `get_prev_session()` method depends on knowing which session is currently active to determine navigation position.

## The Gap

There's a timing window where:
- Session exists in SessionManager (can be found in `sessions` map)
- Session is NOT marked as active (active_session_id is None)
- User can press Shift+Left during this window
- Navigation logic has no active session reference, defaults to "go to board"

## Files Involved

- `src/tui/components/AgentView.tsx` (line 2543) - Session creation
- `codelet/napi/src/session_manager.rs` (lines 3900, 3311) - Active session tracking
- `src/tui/utils/sessionNavigation.ts` (line 49) - Navigation logic

## Solution

The fix should set the active session immediately when creating it, not waiting for `sessionAttach`:

```rust
// In session_manager.rs create_session_with_id method
pub async fn create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<()> {
    let uuid = Uuid::parse_str(id)?;
    
    // ... existing session creation code ...
    
    // NEW: Set as active session immediately upon creation
    self.set_active_session(uuid);
    
    Ok(())
}
```

This ensures navigation works correctly from the moment the session is created, regardless of when streaming begins.