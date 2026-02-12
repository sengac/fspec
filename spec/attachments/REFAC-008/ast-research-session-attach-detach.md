# AST Research: Session Attach/Detach Usage Analysis

## Research Scope

This document analyzes the usage of `sessionAttach` and `sessionDetach` in the codebase, with focus on AgentView.tsx and related files, to inform the REFAC-008 refactoring.

## Research Commands

```bash
# Find sessionAttach calls in AgentView.tsx
fspec research --tool=ast --pattern='sessionAttach' --lang=tsx --path=src/tui/components/AgentView.tsx

# Find sessionDetach calls in AgentView.tsx
fspec research --tool=ast --pattern='sessionDetach' --lang=tsx --path=src/tui/components/AgentView.tsx
```

---

## sessionAttach Locations in AgentView.tsx

| Line | Context | Purpose |
|------|---------|---------|
| 89 | Import | Import from @sengac/codelet-napi |
| 2427 | `/parent` command handler | Attach to parent session when navigating to parent |
| 2920 | handleSubmit callback | Attach during new message submission |
| 4952 | resumeSessionById | Attach when resuming a session |
| 5427 | handleSubmit (watcher sessions) | Attach for watcher session input |
| 5581 | handleSubmit fallback | Attach when provider match creates new session |
| 5875 | effect hook | Attach when auto-resuming work unit's attached session |

**Total: 6 active call sites** (excluding import)

---

## sessionDetach Locations in AgentView.tsx

| Line | Context | Purpose |
|------|---------|---------|
| 93 | Import | Import from @sengac/codelet-napi |
| 2421 | `/parent` command handler | Detach from current before navigating to parent |
| 5013 | handleContinue | Detach before continuing interrupted session |
| 5107 | navigateToSession (onDetachConfirm) | Detach when user confirms leaving running session |
| 5121 | navigateToSession (idle branch) | Detach from idle session when navigating away |
| 5997 | handleExit onConfirm | Detach when user confirms exit from running session |
| 7806 | handleConfirm dialog handler | Detach on exit confirmation |
| 7819 | handleConfirm dialog handler | Detach on exit confirmation (duplicate) |

**Total: 7 active call sites** (excluding import)

---

## FspecCommandRequest Handling Locations

### Location 1: Lines 4778-4873 (handleStreamChunk callback)
- Part of the main stream handling logic
- Receives FspecCommandRequest chunks and executes via fspecCallback
- Sends result back to Rust via sessionSendFspecResult

### Location 2: Lines 3517-3597 (handleSubmit sessionAttach callback)
- Duplicate handling code inside the sessionAttach callback
- Same logic as Location 1
- Both locations need to be extracted and centralized

---

## Other Files Using sessionAttach/sessionDetach

### src/tui/services/sessionService.ts
- Line 182: `sessionAttach()` - Called during session creation
- Line 286: `sessionAttach()` - Called during session restoration/resumption

### src/tui/hooks/sessionAttachment.ts
- Lines 26, 37: Helper functions wrapping attach/detach

### src/tui/store/globalStreamListener.ts
- Line 40: `sessionDetach` - Part of global listener interface

---

## Key Finding: Single Callback Constraint

The Rust `attach()` function REPLACES the callback:

```rust
pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>) {
    *self.attached_callback.write().expect("callback lock poisoned") = Some(callback);
    // This REPLACES, doesn't add to a list
}
```

This means:
- Only ONE subscriber can receive events per session
- GlobalSessionStreamManager MUST be the sole subscriber
- AgentView must register with the manager, not call sessionAttach directly

---

## Impact Analysis

### Files to Modify
1. **AgentView.tsx**: Remove 6 sessionAttach and 7 sessionDetach calls
2. **sessionService.ts**: Integrate with GlobalSessionStreamManager
3. **sessionAttachment.ts**: May be deprecated in favor of manager

### Files to Create
1. **globalSessionStreamManager.ts**: Sole subscriber to all sessions
2. **fspecCommandHandler.ts**: Centralized FspecCommandRequest handling
3. **useSessionStreamManager.ts**: Hook for components to register for events

---

## Risks Identified

1. **Race conditions**: Must subscribe before first input is sent
2. **Memory leaks**: Must unsubscribe when sessions are destroyed
3. **Event ordering**: Manager processes synchronously before forwarding
4. **Callback timing**: New session creation must integrate with manager lifecycle
