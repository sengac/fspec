# AST Research: Sibling Watcher Navigation

## Target Code Location

The `switchToSession` function is defined at:
- **File**: `src/tui/components/AgentView.tsx`
- **Line**: 3667
- **Pattern**: `useCallback((direction: 'prev' | 'next') => { ... }, [...])`

## sessionGetParent NAPI Binding

Already imported and used in AgentView.tsx:
- **Import**: Line 94
- **Usage 1**: Line 1366 - Used in split view detection
- **Usage 2**: Line 2981 - Used in session type determination

## Current switchToSession Implementation

```typescript
const switchToSession = useCallback((direction: 'prev' | 'next') => {
  // Get background sessions from Rust session manager only
  const backgroundSessions = sessionManagerList();
  // ... navigates through ALL sessions
}, [currentSessionId, inputValue, handleStreamChunk, currentProvider, providerSections]);
```

## Required Modification

Add sibling filtering logic at the start of switchToSession:

```typescript
const switchToSession = useCallback((direction: 'prev' | 'next') => {
  // Get background sessions from Rust session manager only
  let backgroundSessions = sessionManagerList();
  
  // WATCH-013: Filter to sibling watchers when in a watcher session
  const currentParentId = currentSessionId ? sessionGetParent(currentSessionId) : null;
  if (currentParentId) {
    // In a watcher session - filter to only sibling watchers (same parent)
    backgroundSessions = backgroundSessions.filter(s => 
      sessionGetParent(s.id) === currentParentId
    );
  }
  
  // ... rest of existing logic unchanged
}, [currentSessionId, inputValue, handleStreamChunk, currentProvider, providerSections]);
```

## Dependencies

- `sessionGetParent` - NAPI binding from WATCH-007 (already imported)
- `sessionManagerList` - Returns all sessions
- `currentSessionId` - Currently viewed session ID

## Integration Points

1. **handleSessionPrev** (line 3820) - calls `switchToSession('prev')`
2. **handleSessionNext** (line 3825) - calls `switchToSession('next')`

## Test Approach

Unit test the filtering logic by mocking:
- `sessionManagerList()` - return test session list
- `sessionGetParent()` - return parent ID for watchers, null for regular sessions
