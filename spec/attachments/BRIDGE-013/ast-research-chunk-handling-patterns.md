# AST Research: Chunk Handling Patterns in AgentView

## Research Summary

This document analyzes the current chunk handling architecture in AgentView.tsx to understand what needs to change for BRIDGE-013.

## Key Functions Found

### 1. attachToSession (useSessionStreamManager.ts:22)
```
src/tui/hooks/useSessionStreamManager.ts:22:8:function attachToSession(
```
This is the low-level function for attaching a handler to a session via GlobalSessionStreamManager.

### 2. useSessionStreamManager (useSessionStreamManager.ts:36)
The hook that should be used at the component level for persistent handler registration. Currently NOT used in AgentView.

### 3. cleanupCurrentSessionHandler References
Found 12 usages of `cleanupCurrentSessionHandler` in AgentView.tsx:
- Line 2034: definition
- Line 2325: before parent watcher attach
- Line 2819: before handleSubmit attach
- Line 4730: before resume attach
- Line 4790: cleanup on return
- Line 4878: cleanup on error
- Line 4885: cleanup on error
- Line 5181: before watcher attach
- Line 5334: before instance attach
- Line 5632: before session attach
- Line 5751: cleanup
- Line 5756: cleanup

### 4. sessionCleanupRef Assignments
Found 6 places where `sessionCleanupRef.current` is assigned with `attachToSession`:
1. Line 2331: `attachToSession(parentId, ...)` - parent watcher
2. Line 2825: `attachToSession(activeSessionId, ...)` - **handleSubmit (main user input)**
3. Line 4733: `attachToSession(sessionId, ...)` - resume session
4. Line 5189: `attachToSession(selectedWatcher.id, ...)` - watcher view
5. Line 5342: `attachToSession(instance.sessionId, ...)` - instance attach
6. Line 5635: `attachToSession(selectedSession.id, ...)` - session switch

### 5. setConversation Calls
96 references to `setConversation` in AgentView.tsx, indicating extensive inline conversation state updates scattered throughout the component.

## Analysis

### Current Architecture (Problematic)
1. **Handler registration tied to user actions**: Handlers are registered inside callbacks like `handleSubmit`, meaning when TUI is idle, no handler is registered
2. **Duplicate code**: Each `attachToSession` call has its own inline chunk processing logic
3. **Complex cleanup**: 12 places call `cleanupCurrentSessionHandler`, making state management error-prone
4. **No persistent handler**: The `useSessionStreamManager` hook exists but is not used for the main session view

### Problems This Causes
1. Bridge input not displayed when TUI is idle (no handler registered)
2. Watcher input may not display consistently
3. Difficult to maintain - 6 different places with chunk handling logic
4. Race conditions possible with cleanup/registration timing

## Proposed Solution

### Replace 6 attachToSession calls with 1 useSessionStreamManager hook
```typescript
// At component level in AgentView
useSessionStreamManager(currentSessionId, handleChunk);
```

### handleChunk uses processStreamingChunk
```typescript
const handleChunk = useCallback((chunk: StreamChunk) => {
  setConversation(prev => {
    const updated = [...prev];
    processStreamingChunk(chunk, updated, ctx);
    return updated;
  });
}, [ctx]);
```

### Simplify handleSubmit
```typescript
const handleSubmit = useCallback(async () => {
  setConversation(prev => [...prev, { type: 'user-input', content: inputValue }]);
  sessionSendInput(currentSessionId, inputValue, thinkingConfig, images);
  setInputValue('');
}, [currentSessionId, inputValue, thinkingConfig, images]);
```

### Remove sessionCleanupRef pattern
- Delete `sessionCleanupRef` definition
- Delete `cleanupCurrentSessionHandler` helper
- Delete all 12 cleanup calls

## Files to Modify

1. `src/tui/components/AgentView.tsx` - Main refactor
2. `src/tui/utils/chunkProcessor.ts` - May need to add envelope tracking
3. Tests for above files

## Existing Infrastructure (Ready to Use)

| Component | Location | Status |
|-----------|----------|--------|
| `useSessionStreamManager` | `src/tui/hooks/useSessionStreamManager.ts` | ✅ Ready |
| `processStreamingChunk` | `src/tui/utils/chunkProcessor.ts` | ✅ Ready |
| `GlobalSessionStreamManager` | `src/tui/services/globalSessionStreamManager.ts` | ✅ Ready |
| `useRustSessionState` | Returns `isLoading` | ✅ Ready |

## Conclusion

The infrastructure is already in place. The fix requires:
1. Using `useSessionStreamManager` hook at the top level
2. Routing all chunks through `processStreamingChunk`
3. Simplifying `handleSubmit` to just send input
4. Removing the cleanup ref pattern
