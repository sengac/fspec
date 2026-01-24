# AST Research: Navigation Components

## Research Date: 2024-01-24
## Work Unit: VIEWNV-001

## 1. useInput Hooks in Target Components

All three views use `useInput` from ink for keyboard handling:

### BoardView.tsx
- Line 257: Main useInput handler for viewMode === 'board'
- Line 354: Loading view ESC handler
- Line 379: Error view ESC handler

### AgentView.tsx
- Line 4978: Main useInput handler (very large, ~200 lines)
- Contains shift+arrow handling at lines 5583-5599

### SplitSessionView.tsx
- Line 164: Main useInput handler
- Currently only handles Left/Right for pane switching, Tab for select mode, ESC

## 2. Session Management APIs Used

From @sengac/codelet-napi:
- sessionManagerList() - List all background sessions
- sessionGetParent(sessionId) - Get parent session ID
- sessionGetWatchers(sessionId) - Get watcher IDs for a parent
- sessionAttach(sessionId, callback) - Attach to session
- sessionDetach(sessionId) - Detach from session
- sessionGetMergedOutput(sessionId) - Get conversation chunks

## 3. Existing Navigation Patterns

### TUI-049 in AgentView (lines 3901-4053)
```typescript
const switchToSession = useCallback((direction: 'prev' | 'next') => {
  let backgroundSessions = sessionManagerList();
  
  // WATCH-013: Filter to sibling watchers when in a watcher session
  if (currentSessionId) {
    const currentParentId = sessionGetParent(currentSessionId);
    if (currentParentId) {
      backgroundSessions = backgroundSessions.filter(s => sessionGetParent(s.id) === currentParentId);
    }
  }
  
  if (backgroundSessions.length < 2) return; // Need 2+ to switch
  
  // Calculate target index with wrap-around
  // Detach, attach, restore conversation...
});
```

### Shift+Arrow Detection (lines 5583-5599)
```typescript
if (input.includes('[1;2D') || input.includes('\x1b[1;2D') || (key.shift && key.leftArrow)) {
  handleSessionPrev();
}
if (input.includes('[1;2C') || input.includes('\x1b[1;2C') || (key.shift && key.rightArrow)) {
  handleSessionNext();
}
```

## 4. View Mode Detection

### isWatcherSessionView (AgentView lines 1446-1512)
```typescript
useEffect(() => {
  if (!currentSessionId) {
    setIsWatcherSessionView(false);
    return;
  }
  const parentId = sessionGetParent(currentSessionId);
  if (parentId) {
    setIsWatcherSessionView(true);
    // Setup split view with parent conversation
  } else {
    setIsWatcherSessionView(false);
  }
}, [currentSessionId]);
```

## 5. Files to Create

### src/tui/utils/sessionNavigation.ts
Pure functions for navigation logic:
- buildNavigationList(sessions, getWatchers, getParent)
- getNextNavigationTarget(context)
- getPrevNavigationTarget(context)

### src/tui/hooks/useSessionNavigation.ts
React hook:
- Wraps pure navigation logic
- Provides handleShiftLeft, handleShiftRight
- Manages showCreateSessionDialog state

### src/components/CreateSessionDialog.tsx
- Reuses Dialog base component
- Two options: Confirm / Cancel
- Calls sessionManagerCreateWithId on confirm

## 6. Integration Points

### BoardView.tsx
- Add useInput handler for shift+right
- Import and use navigation hook
- Trigger setViewMode('agent') when navigating to session

### AgentView.tsx
- Refactor switchToSession to use shared navigation logic
- Add create session dialog state
- Handle navigation to BoardView on shift+left from first session

### SplitSessionView.tsx
- Add shift+left/right handlers (separate from regular left/right)
- Bubble navigation events up to parent AgentView
