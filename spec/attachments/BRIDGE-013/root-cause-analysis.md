# Root Cause Analysis: TUI Not Displaying Bridge Input Responses

## Problem Statement

When input is sent from Telegram via the bridge, the LLM response is displayed in Telegram but NOT in the TUI. The expectation is that the TUI should also display all chunks for the session it's viewing.

## Investigation Summary

### Flow Analysis

When Telegram sends input via the bridge:

1. **Bridge injects input** → `InjectedInput` sent via `watcher_input_tx`
2. **Session processes it** → LLM receives and responds
3. **Chunks emitted** via `handle_output()` to:
   - `watcher_broadcast` → Bridge subscribes → Telegram ✅ **works**
   - `GLOBAL_CHUNK_CALLBACK` → GlobalSessionStreamManager → ❌ **no handlers registered**

### Root Cause

**AgentView only registers chunk handlers during user-initiated interactions.**

Looking at `src/tui/components/AgentView.tsx`:

```typescript
// Line ~2825 - INSIDE handleSubmit()
sessionCleanupRef.current = attachToSession(activeSessionId, (chunk: StreamChunk) => {
    // chunk handling code - complex inline handler
});
```

The handler registration happens **inside `handleSubmit`** - only when the user sends input. The handler exists within a Promise that resolves when the agent completes (Done chunk). When the TUI is "idle" (displaying a session but not actively sending input), there is **NO handler registered**.

### Why Bridge → Telegram Works

The bridge uses `watcher_broadcast` (a tokio broadcast channel) which:
1. Is subscribed to via `broadcast_rx_factory` when the bridge connects
2. Runs continuously in the relay task
3. Has no dependency on user interaction state

### Why TUI Display Fails

The TUI uses `GlobalSessionStreamManager.registerHandler()` which:
1. Only gets called inside `handleSubmit()` 
2. Only persists for the duration of the agent interaction
3. Is cleaned up when `Done` chunk is received or on error

When bridge input arrives and the TUI is idle:
1. `GLOBAL_CHUNK_CALLBACK` fires with `(session_id, chunk)`
2. `GlobalSessionStreamManager.handleChunk()` is called
3. No handlers are registered for the session → chunks are dropped

## Current Architecture Problems

### 1. Duplicate Code
The inline handler in `handleSubmit()` (~lines 2825-3500) duplicates logic that already exists in `processStreamingChunk()` in `chunkProcessor.ts`.

### 2. Complex Coordination
Multiple places register handlers:
- `handleSubmit()` for user input
- Various `/resume`, `/switch` command handlers
- Watcher view handlers

Each has its own inline chunk processing logic.

### 3. No Persistent Handler
The `useSessionStreamManager` hook exists but is NOT used in AgentView for the main session view.

## Proposed Architecture: Single Persistent Handler

### Core Principle
**ONE handler, always registered when viewing a session.**

### Implementation

1. **Use `useSessionStreamManager` hook** for persistent registration:
```typescript
// In AgentView.tsx - at component level
const handleChunk = useCallback((chunk: StreamChunk) => {
  // Use processStreamingChunk from chunkProcessor.ts
  setConversation(prev => {
    const updated = [...prev];
    processStreamingChunk(chunk, updated, chunkProcessorContext);
    return updated;
  });
}, [chunkProcessorContext]);

useSessionStreamManager(currentSessionId, handleChunk);
```

2. **Simplify `handleSubmit`** - just send input:
```typescript
const handleSubmit = useCallback(async () => {
  // Add user message to conversation
  setConversation(prev => [...prev, { type: 'user-input', content: inputValue }]);
  
  // Send to session - chunks flow through persistent handler
  sessionSendInput(currentSessionId, inputValue, thinkingConfig, images);
  
  // Clear input
  setInputValue('');
}, [currentSessionId, inputValue, thinkingConfig, images]);
```

3. **Track completion via `rustSnapshot.isLoading`**:
```typescript
// isLoading comes from useRustSessionState hook
// It reflects session status from Rust (running/idle)
// No need to track Done chunk manually
```

### Benefits

1. **DRY**: Single chunk processing path via `processStreamingChunk()`
2. **Simple**: No handler coordination, no cleanup refs
3. **Always works**: Bridge, watcher, and user input all flow through same handler
4. **Testable**: `processStreamingChunk` is already unit tested

### Existing Infrastructure

- `processStreamingChunk()` in `chunkProcessor.ts` - handles ALL chunk types
- `useSessionStreamManager()` hook - persistent registration with cleanup
- `useRustSessionState()` hook - provides `isLoading` status

## Migration Path

1. Add persistent handler using `useSessionStreamManager`
2. Refactor `handleSubmit` to just send input
3. Remove inline chunk handling from `handleSubmit`
4. Remove `sessionCleanupRef` pattern
5. Update tests to reflect simpler architecture

## Related Work Units

- **BRIDGE-012**: Global chunk callback architecture (done) - established the routing mechanism
- **BRIDGE-013**: This fix - use routing mechanism with persistent handler
