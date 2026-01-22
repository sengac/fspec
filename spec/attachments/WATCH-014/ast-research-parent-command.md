# AST Research: /parent Command Implementation

## Summary
Research findings for implementing the `/parent` command in AgentView.tsx.

## Command Handling Locations

All slash commands are handled in the same section of AgentView.tsx:

| Command | Line | Pattern |
|---------|------|---------|
| `/search` | 1838 | `if (userMessage === '/search')` |
| `/resume` | 1893 | `if (userMessage === '/resume')` |
| `/watcher` | 1900 | `if (userMessage === '/watcher')` |
| `/detach` | 1907 | `if (userMessage === '/detach')` |

**Insertion Point**: `/parent` should be added after `/watcher` command handling, around line 1904.

## sessionGetParent Usage

The `sessionGetParent` NAPI binding is already imported and used in multiple locations:

| Line | Context |
|------|---------|
| 94 | Import statement |
| 1366 | Getting parent for split view detection |
| 2981 | Getting parent in split session view |
| 3675 | Sibling watcher navigation filtering |
| 3678 | Sibling watcher navigation filtering |

**Key Finding**: `sessionGetParent` is already imported and ready to use.

## Session Switching Pattern (handleWatcherSelect)

Located at line 4101, `handleWatcherSelect` demonstrates the pattern for switching sessions:

1. **setCurrentSessionId(selectedWatcher.id)** - Change current session
2. **setIsWatcherMode(false)** - Exit watcher mode 
3. **sessionAttach()** - Attach to new session for streaming
4. **sessionGetMergedOutput()** - Get buffered output
5. **processChunksToConversation()** - Restore conversation display
6. **setConversation()** - Update UI with status message

## Implementation Plan

```typescript
// WATCH-014: Handle /parent command - switch to parent session
if (userMessage === '/parent') {
  setInputValue('');
  
  if (!currentSessionId) {
    setConversation(prev => [
      ...prev,
      { type: 'status', content: 'No active session. Start a session first.' },
    ]);
    return;
  }
  
  const parentId = sessionGetParent(currentSessionId);
  
  if (!parentId) {
    setConversation(prev => [
      ...prev,
      { type: 'status', content: 'This session has no parent. /parent only works from watcher sessions.' },
    ]);
    return;
  }
  
  try {
    // Detach from current watcher session
    sessionDetach(currentSessionId);
    
    // Switch to parent session
    setCurrentSessionId(parentId);
    
    // Attach to parent session for live streaming
    sessionAttach(parentId, (_err: Error | null, chunk: StreamChunk) => {
      if (chunk) {
        handleStreamChunk(chunk);
      }
    });
    
    // Get buffered output and display
    const mergedChunks = sessionGetMergedOutput(parentId);
    const restoredMessages = processChunksToConversation(
      mergedChunks,
      formatToolHeader,
      formatCollapsedOutput
    );
    setConversation(restoredMessages);
    
    setConversation(prev => [
      ...prev,
      { type: 'status', content: 'Switched to parent session' },
    ]);
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Failed to switch to parent';
    setConversation(prev => [
      ...prev,
      { type: 'status', content: `Switch failed: ${errorMessage}` },
    ]);
  }
  return;
}
```

## Dependencies

All required NAPI bindings are already imported:
- `sessionGetParent` (line 94)
- `sessionDetach` (line 73)
- `sessionAttach` (line 69)
- `sessionGetMergedOutput` (line 71)
