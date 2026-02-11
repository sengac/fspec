# REFAC-008: Technical Analysis - Global Session Stream Subscription

## Executive Summary

**The Problem**: When a user navigates away from a session (Shift+Left/Right), detached sessions that invoke the `fspec` tool will **deadlock forever** because the `FspecCommandRequest` StreamChunk is never delivered to TypeScript.

**Root Cause**: Event handling for `FspecCommandRequest` is embedded in `AgentView.tsx`, which only subscribes to the *currently active* session. Detached sessions continue running in Rust but have no callback attached to receive their events.

**The Solution**: `GlobalSessionStreamManager` becomes the **SOLE subscriber** to all session streams. AgentView no longer calls `sessionAttach`/`sessionDetach` directly - it registers with the manager to receive forwarded events.

---

## Problem Analysis

### Current Architecture (Flawed)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CURRENT BROKEN ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   AgentView.tsx                                                              │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │ sessionAttach(sessionId, cb)   ◄── Only subscribes to ONE session  │    │
│   │                                                                     │    │
│   │ cb handles:                                                         │    │
│   │   - Text, Thinking, ToolCall                                        │    │
│   │   - FspecCommandRequest  ◄──── PROBLEM: Only works when attached   │    │
│   │   - Done, Error, etc.                                               │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│   When user presses Shift+Right:                                             │
│   ┌────────────────────────────────┐    ┌────────────────────────────────┐  │
│   │ Session A (DETACHED)           │    │ Session B (ATTACHED)           │  │
│   │                                │    │                                │  │
│   │ Agent running...               │    │ Agent running...               │  │
│   │ Invokes fspec tool             │    │ Events → AgentView.tsx         │  │
│   │ Emits FspecCommandRequest      │    │ FspecCommandRequest handled ✓  │  │
│   │         ↓                      │    │                                │  │
│   │ BUFFERED (no callback)         │    │                                │  │
│   │         ↓                      │    │                                │  │
│   │ wait_for_fspec_response()      │    │                                │  │
│   │         ↓                      │    │                                │  │
│   │ ██████ DEADLOCK ██████         │    │                                │  │
│   │ (blocks forever)               │    │                                │  │
│   └────────────────────────────────┘    └────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Code Flow That Causes Deadlock

1. **User switches sessions** (Shift+Left/Right):
   ```typescript
   // AgentView.tsx line ~5107
   sessionDetach(currentSessionId);  // Callback cleared, is_attached=false
   await resumeSessionById(targetSessionId);  // Attach to new session
   ```

2. **Detached session invokes fspec tool**:
   ```rust
   // session_manager.rs line ~4317
   session_for_fspec.handle_output(StreamChunk::fspec_command_request(fspec_request));
   ```

3. **handle_output() buffers but doesn't forward** (no callback attached):
   ```rust
   // session_manager.rs line ~1089
   if self.is_attached() {  // FALSE - session is detached
       if let Some(cb) = ... {
           let _ = cb.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking);
       }
   }
   // Chunk is buffered but never sent to TypeScript
   ```

4. **Rust blocks forever waiting for response**:
   ```rust
   // session_manager.rs line ~1221
   let result = rx.recv().unwrap_or_else(|e| { ... });  // BLOCKS FOREVER
   ```

### Critical Constraint: Rust Only Supports ONE Callback Per Session

Both `sessionAttach()` and `sessionSubscribe()` call the same underlying `attach()` function:

```rust
pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>) {
    *self.attached_callback.write().expect("callback lock poisoned") = Some(callback);
    // This REPLACES the callback, doesn't add to a list
}
```

**This means we CANNOT have both GlobalSessionStreamManager and AgentView call attach on the same session** - whichever calls it second will replace the first callback.

---

## Chosen Solution: Option B - GlobalSessionStreamManager as SOLE Subscriber

### Why Option B Is Correct

| Principle | How Option B Satisfies It |
|-----------|---------------------------|
| **Single Responsibility** | AgentView only renders UI; manager handles subscriptions |
| **Single Source of Truth** | ONE place manages all session subscriptions |
| **Separation of Concerns** | Event handling logic extracted from AgentView |
| **Consistency** | Follows existing pattern of `globalStreamListener.ts` |
| **Composability** | Handlers can be added/removed independently |
| **Testability** | AgentView can be tested with mocked manager |

### Why Not Option A (Multiple Callbacks in Rust)

Option A would be a **band-aid** that allows the current messy architecture to continue:
- Distributed subscription logic across multiple files
- AgentView still managing subscriptions (violates SRP)
- Event handling logic remains in view component
- Harder to reason about event flow

---

## New Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NEW ARCHITECTURE (OPTION B)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   GlobalSessionStreamManager.ts (SOLE SUBSCRIBER)                            │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │ Singleton that OWNS the callback for ALL sessions                   │    │
│   │                                                                     │    │
│   │ On session creation:                                                │    │
│   │   sessionAttach(sessionId, globalCallback)                          │    │
│   │                                                                     │    │
│   │ globalCallback receives ALL events:                                 │    │
│   │   ┌─────────────────────────────────────────────────────────────┐  │    │
│   │   │                    StreamChunk                               │  │    │
│   │   │                         │                                    │  │    │
│   │   │    ┌────────────────────┼────────────────────┐              │  │    │
│   │   │    ▼                    ▼                    ▼              │  │    │
│   │   │ FspecCommandRequest   Text/Thinking    TokenUpdate          │  │    │
│   │   │    │                  ToolCall/Result   ContextFill         │  │    │
│   │   │    │                  Done/Error        etc.                │  │    │
│   │   │    │                       │                                │  │    │
│   │   │    ▼                       ▼                                │  │    │
│   │   │ FspecCommandHandler   Forward to registered UI handlers     │  │    │
│   │   │ (executes command,    (AgentView receives via callback)     │  │    │
│   │   │  sends result back)                                         │  │    │
│   │   └─────────────────────────────────────────────────────────────┘  │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│   AgentView.tsx (PURE VIEW - NO sessionAttach/sessionDetach calls)           │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │ // Register with manager to receive events                          │    │
│   │ const { registerForSession, unregisterForSession } =                │    │
│   │   useSessionStreamManager();                                        │    │
│   │                                                                     │    │
│   │ useEffect(() => {                                                   │    │
│   │   if (sessionId) {                                                  │    │
│   │     registerForSession(sessionId, handleUIChunk);                   │    │
│   │     return () => unregisterForSession(sessionId);                   │    │
│   │   }                                                                 │    │
│   │ }, [sessionId]);                                                    │    │
│   │                                                                     │    │
│   │ // handleUIChunk receives ONLY UI-relevant events                   │    │
│   │ // Does NOT receive FspecCommandRequest - manager handles those     │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│   Multiple Sessions (ALL subscribed via manager)                             │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                      │
│   │ Session A    │  │ Session B    │  │ Session C    │                      │
│   │ (detached)   │  │ (displayed)  │  │ (detached)   │                      │
│   │      │       │  │      │       │  │      │       │                      │
│   │      └───────┴──┴──────┴───────┴──┴──────┘       │                      │
│   │                        │                          │                      │
│   │                        ▼                          │                      │
│   │          GlobalSessionStreamManager               │                      │
│   │          (receives events from ALL)               │                      │
│   │                        │                          │                      │
│   │         ┌──────────────┼──────────────┐          │                      │
│   │         ▼              ▼              ▼          │                      │
│   │   FspecHandler    AgentView     Other handlers   │                      │
│   │   (any session)   (Session B)   (future)         │                      │
│   └──────────────────────────────────────────────────┘                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## File Structure

```
src/tui/
├── services/
│   ├── sessionService.ts                 # Existing - session creation/restoration
│   ├── globalSessionStreamManager.ts     # NEW - SOLE subscriber to all sessions
│   └── fspecCommandHandler.ts            # NEW - FspecCommandRequest handling
│
├── hooks/
│   ├── useSessionNavigation.ts           # Existing
│   └── useSessionStreamManager.ts        # NEW - Hook to register for session events
│
├── store/
│   ├── globalStreamListener.ts           # Existing (WorkUnitsWatcher only)
│   └── sessionStore.ts                   # Existing
│
└── components/
    └── AgentView.tsx                     # REFACTORED - No sessionAttach calls
```

---

## File Responsibilities

### `globalSessionStreamManager.ts` (NEW)

```typescript
/**
 * Global Session Stream Manager
 * 
 * SOLE subscriber to all session streams. AgentView and other components
 * do NOT call sessionAttach directly - they register with this manager.
 * 
 * Key constraint: Rust only supports ONE callback per session.
 * This manager owns that callback and multiplexes events to handlers.
 */

type SessionChunkHandler = (sessionId: string, chunk: StreamChunk) => void;

interface GlobalSessionStreamManager {
  // Called by sessionService when session is created
  subscribeToSession(sessionId: string): void;
  
  // Called when session is destroyed
  unsubscribeFromSession(sessionId: string): void;
  
  // Components register to receive events for specific sessions
  registerHandler(sessionId: string, handler: SessionChunkHandler): () => void;
  
  // Global handlers receive events from ALL sessions (e.g., FspecCommandHandler)
  registerGlobalHandler(handler: SessionChunkHandler): () => void;
}

// Singleton instance
export const globalSessionStreamManager: GlobalSessionStreamManager;

// Initialization (called once at app startup)
export function initGlobalSessionStreamManager(): void;
```

### `fspecCommandHandler.ts` (NEW)

```typescript
/**
 * Fspec Command Handler
 * 
 * Handles FspecCommandRequest events from ANY session.
 * Registered as a global handler with GlobalSessionStreamManager.
 */

export class FspecCommandHandler {
  onChunk(sessionId: string, chunk: StreamChunk): void {
    if (chunk.type !== 'FspecCommandRequest' || !chunk.fspecRequest) {
      return;
    }
    
    void this.handleFspecRequest(sessionId, chunk.fspecRequest);
  }
  
  private async handleFspecRequest(
    sessionId: string, 
    request: FspecRequest
  ): Promise<void> {
    const { command, argsJson, projectRoot, toolCallId } = request;
    
    try {
      const resultJson = await fspecCallback(command, argsJson, projectRoot);
      const parsed = JSON.parse(resultJson);
      
      sessionSendFspecResult(sessionId, {
        success: parsed.success ?? true,
        data: parsed.data ?? resultJson,
        error: parsed.error ?? undefined,
        systemReminder: this.buildSystemReminder(parsed.systemReminders),
        toolCallId,
      });
    } catch (error) {
      sessionSendFspecResult(sessionId, {
        success: false,
        data: '',
        error: error.message,
        toolCallId,
      });
    }
  }
}
```

### `useSessionStreamManager.ts` (NEW)

```typescript
/**
 * Hook for components to receive session stream events
 * 
 * Replaces direct sessionAttach/sessionDetach calls in components.
 * Components register interest in a session and receive forwarded events.
 */

export function useSessionStreamManager() {
  const registerForSession = useCallback((
    sessionId: string,
    handler: (chunk: StreamChunk) => void
  ) => {
    return globalSessionStreamManager.registerHandler(sessionId, (_, chunk) => {
      // Filter out events that components shouldn't handle
      if (chunk.type === 'FspecCommandRequest') {
        return; // Handled by FspecCommandHandler
      }
      handler(chunk);
    });
  }, []);
  
  return { registerForSession };
}
```

---

## Migration Strategy

### Phase 1: Create Infrastructure (Non-Breaking)
1. Create `globalSessionStreamManager.ts` with subscription logic
2. Create `fspecCommandHandler.ts` 
3. Create `useSessionStreamManager.ts` hook
4. Initialize manager at app startup (alongside existing `initGlobalStreamListener`)

### Phase 2: Integrate with Session Lifecycle
1. Modify `sessionService.createSession()` to call `manager.subscribeToSession()`
2. Modify session destruction flow to call `manager.unsubscribeFromSession()`
3. Register `FspecCommandHandler` as global handler
4. Verify FspecCommandRequest handling works for all sessions

### Phase 3: Refactor AgentView
1. Replace `sessionAttach()` calls with `useSessionStreamManager()` hook
2. Remove `sessionDetach()` calls (manager handles lifecycle)
3. Remove FspecCommandRequest handling code (now in dedicated handler)
4. Remove duplicate event handling logic

### Phase 4: Cleanup
1. Remove dead code from AgentView
2. Update tests to use new patterns
3. Update documentation

---

## Risk Analysis

| Risk | Mitigation |
|------|------------|
| Breaking existing functionality | Phased migration with tests at each step |
| Event ordering issues | Manager processes synchronously before forwarding |
| Memory leaks | Strict subscribe/unsubscribe lifecycle tied to session creation/destruction |
| Race conditions | Subscription happens in session creation before first input |
| Performance overhead | Minimal - just function call forwarding |

---

## Success Criteria

1. **Detached sessions can invoke fspec tools** without deadlock
2. **AgentView contains NO sessionAttach/sessionDetach calls** - only registers with manager
3. **FspecCommandRequest handling extracted** to dedicated handler
4. **Single source of truth** for session subscriptions (GlobalSessionStreamManager)
5. **Clean separation of concerns** - view renders, service handles events
6. **Composable handlers** that can be added/removed independently

---

## Appendix: Relevant Code Locations

### Rust (codelet/napi/src/session_manager.rs)
- Line 1058: `handle_output()` - buffers and forwards chunks
- Line 1103: `attach()` - sets callback (**REPLACES**, doesn't add)
- Line 1111: `detach()` - clears callback
- Line 1216: `wait_for_fspec_response()` - BLOCKS waiting for TypeScript
- Line 4317: Emits `FspecCommandRequest` chunk
- Line 4881: `session_attach()` NAPI function
- Line 4911: `session_subscribe()` NAPI function (also calls attach() - same behavior)

### TypeScript (src/tui/)
- `components/AgentView.tsx`: Lines 4778-4860 - FspecCommandRequest handling (TO BE REMOVED)
- `components/AgentView.tsx`: Lines 3518-3600 - Duplicate handling (TO BE REMOVED)
- `store/globalStreamListener.ts`: WorkUnitsWatcher only (NOT session streams)
- `services/sessionService.ts`: Session creation (NEEDS MODIFICATION)

### NAPI Type Definitions (node_modules/@sengac/codelet-napi/index.d.ts)
- `StreamChunk` union type with `FspecCommandRequest` variant
- `sessionAttach()` function signature
- `sessionSendFspecResult()` function signature
