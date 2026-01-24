# VIEWNV-001 Research Notes: Unified Shift+Arrow Navigation

## Executive Summary

This document captures the research findings for implementing unified Shift+Left/Right arrow navigation across all major TUI views: BoardView, AgentView, and SplitPaneView.

## Current Architecture

### Three Main Views

| View | File | Lines | Purpose |
|------|------|-------|---------|
| BoardView | `src/tui/components/BoardView.tsx` | ~493 | Kanban board showing work units |
| AgentView | `src/tui/components/AgentView.tsx` | ~6800 | Main AI conversation view |
| SplitSessionView | `src/tui/components/SplitSessionView.tsx` | ~463 | Split pane for watcher sessions |

### Existing Navigation Patterns

#### BoardView (No Shift+Arrow navigation currently)
- Has `viewMode` state: `'board' | 'checkpoint-viewer' | 'changed-files-viewer' | 'agent'`
- Enter on work unit → opens AgentView
- Tab switches between board/stash/files panels

#### AgentView (TUI-049: Shift+Arrow implemented)
- **Shift+Left**: Switch to previous session
- **Shift+Right**: Switch to next session
- **WATCH-013**: When in watcher session, navigation filters to sibling watchers only
- `/parent` command switches from watcher to parent session

Key code location: `switchToSession` function at lines 3901-4053

```typescript
// Current logic simplified:
const switchToSession = (direction: 'prev' | 'next') => {
  let sessions = sessionManagerList();
  
  // WATCH-013: Filter to siblings if in watcher
  if (currentSessionId) {
    const parentId = sessionGetParent(currentSessionId);
    if (parentId) {
      sessions = sessions.filter(s => sessionGetParent(s.id) === parentId);
    }
  }
  
  // Navigate with wrap-around
  const targetIndex = direction === 'next'
    ? (currentIndex + 1) % sessions.length
    : (currentIndex - 1 + sessions.length) % sessions.length;
  
  // Detach, attach, restore conversation...
};
```

#### SplitSessionView (No Shift+Arrow navigation currently)
- **Left/Right arrows**: Switch active pane between parent and watcher
- **Tab**: Toggle turn-select mode
- No session switching capability

### Session Management APIs

From `@sengac/codelet-napi`:
- `sessionManagerList()` - List all background sessions
- `sessionGetParent(sessionId)` - Get parent session ID (null if not a watcher)
- `sessionGetWatchers(sessionId)` - Get watcher session IDs for a parent
- `sessionAttach(sessionId, callback)` - Attach to session for streaming
- `sessionDetach(sessionId)` - Detach from session
- `sessionGetMergedOutput(sessionId)` - Get conversation chunks
- `sessionManagerCreateWithId(...)` - Create new session

### Dialog Patterns

| Dialog | File | Purpose |
|--------|------|---------|
| Dialog | `src/components/Dialog.tsx` | Base modal with ESC handling |
| ThreeButtonDialog | `src/components/ThreeButtonDialog.tsx` | 3 horizontal button options |
| NotificationDialog | `src/components/NotificationDialog.tsx` | Auto-dismissing notifications |
| ErrorDialog | `src/components/ErrorDialog.tsx` | Error display |

## Proposed Architecture

### Linear Navigation Model

The user wants a flattened linear navigation through all views and sessions:

```
[Board] ↔ [Session1] ↔ [S1.Watcher1] ↔ [S1.Watcher2] ↔ [Session2] ↔ [S2.Watcher1] ↔ ... → [Create?]
```

- **Shift+Right**: Move forward through this list
- **Shift+Left**: Move backward through this list
- At the **end** (right edge): Prompt to create new session
- At the **beginning** (left edge from first session): Return to Board

### Proposed New Files (SOLID/DRY/Separation of Concerns)

#### 1. `src/tui/utils/sessionNavigation.ts` - Pure Navigation Logic

```typescript
// Types
interface NavigationContext {
  currentView: 'board' | 'agent' | 'split-pane';
  currentSessionId: string | null;
  parentSessionId: string | null; // null if not a watcher
}

type NavigationTarget =
  | { type: 'board' }
  | { type: 'session'; sessionId: string }
  | { type: 'watcher'; sessionId: string; parentId: string }
  | { type: 'prompt-create-session' }
  | { type: 'none' };

// Pure functions
function buildNavigationList(): NavigationItem[];
function getNextNavigationTarget(ctx: NavigationContext): NavigationTarget;
function getPrevNavigationTarget(ctx: NavigationContext): NavigationTarget;
```

#### 2. `src/tui/hooks/useSessionNavigation.ts` - React Hook

```typescript
interface UseSessionNavigationReturn {
  handleShiftRight: () => void;
  handleShiftLeft: () => void;
  showCreateSessionDialog: boolean;
  setShowCreateSessionDialog: (show: boolean) => void;
  createUnattachedSession: () => Promise<string>;
}

function useSessionNavigation(callbacks: {
  onNavigateToBoard: () => void;
  onNavigateToSession: (sessionId: string) => void;
}): UseSessionNavigationReturn;
```

#### 3. `src/components/CreateSessionDialog.tsx` - Confirmation Dialog

```typescript
// Reuses Dialog base, two buttons: "Yes, Create" | "Cancel"
interface CreateSessionDialogProps {
  onConfirm: () => void;
  onCancel: () => void;
}
```

### Navigation Rules

| From | Shift+Right | Shift+Left |
|------|-------------|------------|
| **Board** | First session, or prompt create | No action (already at start) |
| **Session (no watchers)** | Next session, or prompt create | Prev session, or Board |
| **Session (has watchers)** | First watcher | Prev session, or Board |
| **Watcher (has next sibling)** | Next sibling watcher | Prev sibling watcher |
| **Watcher (last sibling)** | Next session, or prompt create | Prev sibling, or Parent |
| **Watcher (first sibling)** | Next sibling | Parent session |

### File Changes Summary

| File | Change |
|------|--------|
| `src/tui/utils/sessionNavigation.ts` | **NEW** - Navigation logic |
| `src/tui/hooks/useSessionNavigation.ts` | **NEW** - React hook |
| `src/components/CreateSessionDialog.tsx` | **NEW** - Confirmation dialog |
| `src/tui/components/BoardView.tsx` | ADD Shift+Right handler |
| `src/tui/components/AgentView.tsx` | REFACTOR switchToSession, add prompt logic |
| `src/tui/components/SplitSessionView.tsx` | ADD Shift+Left/Right handlers |

## Open Questions for Example Mapping

1. **Board ← Session navigation**: When at the first session and pressing Shift+Left, should it go back to BoardView? Or wrap to the last session?

2. **Watcher creation**: Should watchers be included in the "create new session" flow, or only regular sessions?

3. **Session ordering**: The navigation list order - should it be by creation time, last updated, or some other criteria?

4. **Split pane behavior**: When navigating away from a watcher (e.g., Shift+Right to next session), should the SplitSessionView close and show a regular AgentView?

5. **Current behavior preservation**: The existing `/parent` command should still work alongside Shift+Left navigation, correct?

## Existing Test Coverage

### TUI-049 Tests: `src/__tests__/session-switching.test.ts`
- Switch to next session with Shift+Right
- Switch to previous session with Shift+Left
- Wrap around from last to first session
- Wrap around from first to last session
- No action with only one session
- No action when in resume mode
- Running session continues in background
- Input text preserved when switching

### WATCH-013 Tests: `src/__tests__/sibling-watcher-navigation.test.ts`
- Navigate forward through sibling watchers
- Regular session navigates through all sessions
- Single watcher has no siblings to navigate
- Navigate backward through sibling watchers
- Watchers of different parents are isolated

## Technical Constraints

1. **Shift+Arrow Detection**: Must handle both escape sequences and Ink key detection:
   ```typescript
   if (input.includes('[1;2D') || input.includes('\x1b[1;2D') || (key.shift && key.leftArrow))
   ```

2. **Session State Preservation**: When switching, must:
   - Save pending input to current session
   - Detach from current session
   - Skip input animation
   - Clear token display state
   - Load and display new session's conversation
   - Restore provider/model state
   - Restore token state for running sessions

3. **View Mode Management**: BoardView uses `viewMode` state to switch between sub-views. Navigation needs to coordinate with this.

## References

- TUI-049: Shift+Arrow Session Switching (implemented in AgentView)
- WATCH-013: Sibling Watcher Navigation (implemented in AgentView)
- WATCH-014: Parent command for watcher sessions
- WATCH-010: Split view for watcher sessions
