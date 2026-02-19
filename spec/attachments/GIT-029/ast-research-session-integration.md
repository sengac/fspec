# AST Research: Session Integration Points

## Work Unit: GIT-029 - TUI integration for isolated sessions

## Session Creation Scenarios

### Scenario 1: CreateSessionDialog (Shift+Right Navigation)
**Location:** `src/tui/components/AgentView.tsx` lines 4800-4860
**Trigger:** User navigates Shift+Right past last session → `CreateSessionDialog` appears → User confirms

```typescript
// Line 4820-4823
const result = await createSession({
  modelPath,
  project,
});
```

**Dialog Used:** `src/components/CreateSessionDialog.tsx`
- Simple Yes/No dialog asking "Start New Agent?"
- Uses base `Dialog` component
- **MODIFICATION NEEDED:** Add "Isolated" toggle before Yes/No buttons

### Scenario 2: Auto-Create on Mount
**Location:** `src/tui/components/AgentView.tsx` lines 4960-5010
**Trigger:** `shouldAutoCreateSession` is true when arriving at AgentView

```typescript
// Line 4982-4985
const result = await createSession({
  modelPath,
  project,
});
```

**Dialog Used:** None (automatic)
- **DECISION:** Either always create non-isolated, OR add preference setting
- Could use previous session's isolated preference

### Scenario 3: First Message Implicit Creation
**Location:** `src/tui/components/AgentView.tsx` lines 2680-2750
**Trigger:** User sends first message when no session exists

```typescript
// Line 2708-2713
await sessionManagerCreateWithId(
  activeSessionId,
  modelPath,
  project,
  sessionName
);
```

**Dialog Used:** None (implicit)
- **DECISION:** Need either a toggle or setting to control isolation
- Could show a dialog before creating, OR use a global preference

## Dialog Architecture

### Base Dialog Component
**Location:** `src/components/Dialog.tsx`

```typescript
interface DialogProps {
  children: ReactNode;
  onClose: () => void;
  borderColor?: string;
  isActive?: boolean;
}
```

Features:
- Centered modal overlay
- ESC key handling → calls onClose
- CRITICAL input priority

### Existing Dialogs (Pattern References)

1. **CreateSessionDialog** (`src/components/CreateSessionDialog.tsx`)
   - Simple Yes/No with horizontal buttons
   - Left/Right navigation, Enter select
   - Wraps `Dialog` component

2. **ConfirmationDialog** (`src/components/ConfirmationDialog.tsx`)
   - Multiple modes: yesno, visual, typed, keypress, triple
   - Risk level → border color mapping
   - Button selection with Left/Right

3. **ThreeButtonDialog** (`src/components/ThreeButtonDialog.tsx`)
   - Three horizontal button options
   - Left/Right wrap-around navigation

4. **WatcherCreateView** (`src/tui/components/WatcherCreateView.tsx`)
   - Full-screen form with multiple fields
   - Tab cycling through fields
   - Toggle fields (Authority, Auto-inject)
   - Text input fields

## Required Changes

### Part A: Modify CreateSessionDialog

**New Interface:**
```typescript
export interface CreateSessionDialogProps {
  onConfirm: (isolated: boolean) => void;  // Changed to pass isolation flag
  onCancel: () => void;
}
```

**UI Changes:**
1. Add toggle: `[ ] Isolated` (default OFF)
2. Toggle with Left/Right when focused
3. Tab to cycle between: Isolated toggle → Yes → No
4. Pass `isolated` boolean to `onConfirm`

### Part B: Update AgentView Session Creation

**Modify handleConfirmNewSession callback (~line 4800):**
```typescript
const handleConfirmNewSession = useCallback(async (isolated: boolean) => {
  // ...
  if (isolated) {
    const result = await createIsolatedSession({
      modelPath,
      project,
    });
    // ...
  } else {
    const result = await createSession({
      modelPath,
      project,
    });
  }
  // ...
}, [...]);
```

### Part C: Session Service Extensions

**New Function in `src/tui/services/sessionService.ts`:**
```typescript
export interface CreateIsolatedSessionResult extends CreateSessionResult {
  worktreePath: string;
  baseCommit: string;
}

export async function createIsolatedSession(
  options: CreateSessionOptions
): Promise<CreateIsolatedSessionResult> {
  const { modelPath, project, name } = options;
  const sessionName = name || `New Session ${new Date().toLocaleString()}`;

  // Create persisted session first
  const persistedSession = persistenceCreateSessionWithProvider(
    sessionName,
    project,
    modelPath
  );

  // Create isolated Rust background session with worktree
  const isolatedInfo = await sessionManagerCreateIsolated(
    persistedSession.id,
    modelPath,
    project,
    sessionName
  );

  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(persistedSession.id);

  return {
    sessionId: persistedSession.id,
    name: sessionName,
    provider: modelPath,
    worktreePath: isolatedInfo.worktreePath,
    baseCommit: isolatedInfo.baseCommit,
  };
}
```

### Part D: New SessionManagementPanel Component

**Location:** `src/tui/components/SessionManagementPanel.tsx`

Features:
- Accessible via command (e.g., `/sessions` or `/manage`)
- Lists completed isolated sessions
- Status badges with colors
- Merge/Discard/Prune actions

**UI Layout:**
```
┌─ Isolated Sessions ─────────────────────────────────┐
│                                                      │
│ Session                  Status         Files        │
│ ─────────────────────────────────────────────────── │
│ ► abc-123 (My Session)   [pending_merge]   5 files  │
│   def-456 (Other)        [clean]           0 files  │
│   ghi-789 (Orphaned)     [orphaned]        3 files  │
│                                                      │
│ [Merge] [Discard] [Inspect] [Prune Orphaned]        │
│                                                      │
│ ↑↓ Navigate | Enter Select | M Merge | D Discard   │
└──────────────────────────────────────────────────────┘
```

## NAPI Imports Required

**In `src/tui/services/sessionService.ts`:**
```typescript
import {
  sessionManagerCreateWithId,
  sessionManagerCreateIsolated,  // NEW - GIT-028
  sessionManagerList,
  // ...existing imports
} from '@sengac/codelet-napi';
```

**In `src/tui/components/SessionManagementPanel.tsx`:**
```typescript
import {
  listSessions,
  inspectSession,
  mergeSession,
  discardSession,
  pruneOrphaned,
  sessionManagerList,  // For getting active session IDs
} from '@sengac/codelet-napi';
```

## Files to Modify

| File | Changes |
|------|---------|
| `src/components/CreateSessionDialog.tsx` | Add isolated toggle, change onConfirm signature |
| `src/tui/components/AgentView.tsx` | Handle isolated flag in handleConfirmNewSession |
| `src/tui/services/sessionService.ts` | Add createIsolatedSession function |
| `src/tui/components/SplitSessionView.tsx` | Pass isolated flag through |
| `src/tui/components/BoardView.tsx` | Pass isolated flag through |

## Files to Create

| File | Purpose |
|------|---------|
| `src/tui/components/SessionManagementPanel.tsx` | Session management UI |
| `src/tui/__tests__/session-management-napi.test.ts` | NAPI binding tests for listSessions, mergeSession, etc. |
| `src/tui/__tests__/isolated-session-tui.test.tsx` | TUI integration tests for isolated toggle |

## Integration Tests Required

1. **listSessions NAPI binding**
   - Returns sessions with derived status
   - Filters by status work correctly

2. **inspectSession NAPI binding**
   - Returns diff without side effects
   - Worktree remains intact

3. **mergeSession NAPI binding**
   - Copies files to main worktree
   - Removes session worktree
   - Returns MergeResult

4. **discardSession NAPI binding**
   - Removes worktree
   - No files modified in main
   - Returns DiscardResult

5. **pruneOrphaned NAPI binding**
   - Removes orphaned worktrees
   - Returns PruneResult with count
