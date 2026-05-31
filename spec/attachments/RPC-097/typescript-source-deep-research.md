# RPC-097 — DeepSearch: Shift+Right → CreateSessionDialog in the TypeScript Ink Frontend

Research date: 2026-05-28
Scope: `src/tui/`, `src/store/`, `src/components/`

---

## 1. Where Shift+Right Is Detected

**File:** `src/tui/components/AgentView.tsx`
**Lines:** 4708–4729

```tsx
// VIEWNV-001: Shift+Left/Right for unified session navigation
// Uses sessionNavigation hook which determines correct target based on position in tree
// Check escape sequences first, then Ink key detection
{
  const isShiftLeft =
    input.includes('[1;2D') ||
    input.includes('\x1b[1;2D') ||
    (key.shift && key.leftArrow);
  const isShiftRight =
    input.includes('[1;2C') ||
    input.includes('\x1b[1;2C') ||
    (key.shift && key.rightArrow);

  if (isShiftLeft) {
    sessionNavigation.handleShiftLeft();
    return true;
  }
  if (isShiftRight) {
    sessionNavigation.handleShiftRight();
    return true;
  }
}
```

`src/tui/components/MultiLineInput.tsx` (lines 201–214) explicitly **does not** consume
Shift+Left/Right — it returns `false` so the keys propagate up to AgentView:

```tsx
// TUI-049: Shift+Left/Right for session switching - let it propagate to view level
if (input.includes('[1;2D') || input.includes('\x1b[1;2D')) {
  return false;
}
if (input.includes('[1;2C') || input.includes('\x1b[1;2C')) {
  return false;
}
if (key.shift && key.leftArrow) { return false; }
if (key.shift && key.rightArrow) { return false; }
```

## 2. What Shift+Right Triggers

Shift+Right calls `sessionNavigation.handleShiftRight` from the
`useSessionNavigation` hook.

**File:** `src/tui/hooks/useSessionNavigation.ts`, lines 48–62:

```ts
const handleShiftRight = useCallback(() => {
  const result = navigateRight();

  switch (result.type) {
    case 'session':
      onNavigate(result.sessionId);
      break;
    case 'create-dialog':
      openCreateSessionDialog();
      break;
    case 'board':
      // Shouldn't happen on right navigation
      break;
  }
}, [onNavigate, openCreateSessionDialog]);
```

**File:** `src/tui/utils/sessionNavigation.ts`, lines 32–40:

```ts
export function navigateRight(): NavigationResult {
  const next = sessionGetNext();   // Rust SessionManager via @sengac/codelet-napi

  if (next) {
    return { type: 'session', sessionId: next };
  } else {
    return { type: 'create-dialog' };
  }
}
```

## 3. The Confirmation Dialog

A **CreateSessionDialog** modal is shown. Rendered by `AgentView.tsx` lines 5561–5567:

```tsx
{/* VIEWNV-001: Create session dialog (shown when navigating past right edge) */}
{showCreateSessionDialog && (
  <CreateSessionDialog
    onConfirm={handleCreateSessionConfirm}
    onCancel={closeCreateSessionDialog}
  />
)}
```

Also rendered by `BoardView.tsx` line 619 when no session is attached.

## 4. The CreateSessionDialog Component (EXACT TS REFERENCE)

**File:** `src/components/CreateSessionDialog.tsx`

### Props (lines 41–48)

```ts
export interface CreateSessionDialogProps {
  onConfirm: (isolated: boolean) => void;
  onCancel: () => void;
  workUnit?: WorkUnitInfo;       // TUI-067: optional work unit
}
```

### Options (lines 27–31) — exact ordering

```ts
type DialogOption = 'yes' | 'yes-isolated' | 'cancel';
const OPTIONS: DialogOption[] = ['yes', 'yes-isolated', 'cancel'];
```

### Title + Description (lines 67–71) — context-aware

```ts
const title = workUnit ? `Work on ${workUnit.id}?` : 'Start New Agent?';
const description = workUnit
  ? 'Start an AI session for this task'
  : 'Begin a fresh AI conversation, not linked to any task.';
```

When triggered by Shift+Right past the last session, `workUnit` is **not** passed,
so the user sees:

* Title: `Start New Agent?`
* Description: `Begin a fresh AI conversation, not linked to any task.`

### Rendered tree (lines 118–134)

```tsx
return (
  <Dialog onClose={onCancel} borderColor="cyan">
    <Text bold>{title}</Text>
    <Text dimColor>{description}</Text>

    {/* TUI-090: Three flat options */}
    <Box marginTop={1} justifyContent="center">
      {renderOption('yes', 'Yes')}
      {renderOption('yes-isolated', 'Yes - Isolated')}
      {renderOption('cancel', 'Cancel')}
    </Box>

    <Box marginTop={1} justifyContent="center">
      <Text dimColor>← → Select | Enter Confirm | Esc Cancel</Text>
    </Box>
  </Dialog>
);
```

### Option button rendering (lines 100–115)

```tsx
const renderOption = (option: DialogOption, label: string) => {
  const isSelected = OPTIONS[selectedIndex] === option;
  return (
    <Box marginX={1} key={option}>
      <Text
        backgroundColor={isSelected ? 'blue' : undefined}
        color={isSelected ? 'white' : 'gray'}
        bold={isSelected}
      >
        {` ${label} `}
      </Text>
    </Box>
  );
};
```

### Key handling (lines 73–98)

```ts
useInputCompat({
  id: 'create-session-dialog-nav',
  priority: InputPriority.CRITICAL,
  isActive: true,
  handler: (_input, key) => {
    if (key.rightArrow) {
      setSelectedIndex(prev => (prev + 1) % OPTIONS.length);
      return true;
    } else if (key.leftArrow) {
      setSelectedIndex(prev => (prev - 1 + OPTIONS.length) % OPTIONS.length);
      return true;
    } else if (key.return) {
      const selected = OPTIONS[selectedIndex];
      if (selected === 'yes') {
        onConfirm(false);
      } else if (selected === 'yes-isolated') {
        onConfirm(true);
      } else {
        onCancel();
      }
      return true;
    }
    return false;
  },
});
```

ESC is captured by the base `Dialog` component (`src/components/Dialog.tsx`),
which calls `onClose` → `onCancel`.

### EXACT visual contract (selected vs unselected)

| Attribute            | Selected            | Unselected      |
|----------------------|---------------------|-----------------|
| `backgroundColor`    | `blue`              | (none)          |
| `color`              | `white`             | `gray`          |
| `bold`               | `true`              | `false`         |
| Label padding        | `' Yes '` (single space each side) | same |
| Container            | `<Box marginX={1}>` | `<Box marginX={1}>` |

There are **NO ▸ / ○ markers** in the TS source. Visual selection is conveyed
solely via the **blue background + white bold text** on the selected button.

### EXACT footer

```
← → Select | Enter Confirm | Esc Cancel
```

* Separator is **ASCII pipe `|`** (U+007C), not the box-drawing
  `│` (U+2502).
* Wrapped in `<Text dimColor>` → all-grey.

### Border

`borderColor="cyan"` on the wrapping `<Dialog>`.

## 5. Symmetric Shift+Left Binding

**File:** `src/tui/hooks/useSessionNavigation.ts` lines 64–79:

```ts
const handleShiftLeft = useCallback(() => {
  const result = navigateLeft();

  switch (result.type) {
    case 'session':
      onNavigate(result.sessionId);
      break;
    case 'board':
      clearActiveSession();
      onNavigateToBoard();
      break;
    case 'create-dialog':
      // Shouldn't happen on left navigation
      break;
  }
}, [onNavigate, onNavigateToBoard]);
```

`navigateLeft()` in `sessionNavigation.ts`:

* From a session → previous session.
* From first session → board view (asymmetric: left never creates a dialog).

## 6. Full Code Path: Keypress → Session Creation

```
1. User presses Shift+Right
      │
2. MultiLineInput.tsx:206-213  →  returns false (propagate)
      │
3. AgentView.tsx:4716-4727  →  detects isShiftRight,
                                calls sessionNavigation.handleShiftRight()
      │
4. useSessionNavigation.ts:48-62  →  calls navigateRight()
      │
5. utils/sessionNavigation.ts:32-40  →  sessionGetNext() (Rust)
      │
      ├─ If next session exists → { type: 'session', sessionId }
      │     →  onNavigate(sessionId) — switches session,
      │        snapshots pending input via sessionSetPendingInput,
      │        calls resumeSessionById.
      │
      └─ If no next session → { type: 'create-dialog' }
            →  openCreateSessionDialog() store action.
                  │
6. sessionStore.ts:242-247  →  showCreateSessionDialog = true
      │
7. AgentView.tsx:5561-5567  →  renders <CreateSessionDialog />
      │     Title: "Start New Agent?"
      │     Description: "Begin a fresh AI conversation, not linked to any task."
      │     Options: [Yes] [Yes - Isolated] [Cancel]
      │
8. User picks:
      ├─ Yes              → onConfirm(false)
      ├─ Yes - Isolated   → onConfirm(true)
      └─ Cancel / Esc     → onCancel()
      │
9. handleCreateSessionConfirm in AgentView.tsx:3658-3791:
      │  - Validates modelsInitialized
      │  - cleanupCurrentSessionHandler()
      │  - configureProfileEnvironment()
      │  - buildModelString()
      │  - isolated === true  → createIsolatedSession({...})  (line 3702)
      │  - isolated === false → createSession({...})          (line 3716)
      │  - activateSession(result.sessionId)
      │  - applyPendingIsolationState() / applyPendingDebugState()
      │  - SESS-001: auto-attach work unit only if !wasInSession
      │  - clear conversation + inputValue
      │  - closeCreateSessionDialog()
```

## 7. State Machine / Store Actions

### Session Store: `src/tui/store/sessionStore.ts`

Fields:

* `currentSessionId: string | null`
* `showCreateSessionDialog: boolean`
* `navigationTargetSessionId: string | null`
* `isReadyForNewSession: boolean`
* `shouldAutoCreateSession: boolean`
* `pendingIsolatedSession: boolean`
* `isIsolated: boolean`, `worktreePath: string | null`

Actions:

* **`openCreateSessionDialog`** (lines 242–247) — sets `showCreateSessionDialog = true`.
* **`closeCreateSessionDialog`** (lines 249–254) — sets `showCreateSessionDialog = false`.
* **`activateSession(sessionId)`** — called after Rust returns a new SessionId.
* **`prepareForNewSession`** — fallback if creation fails.

### Rust-side State (source of truth via `@sengac/codelet-napi`)

* `sessionGetNext()` — next session from SessionManager IndexMap.
* `sessionGetPrev()`.
* `sessionGetFirst()`.
* `sessionClearActive()` — when returning to board.

### Input Priority

`CreateSessionDialog` and base `Dialog` use `InputPriority.CRITICAL` via
`useInputCompat` (lines 75 and 45) so the modal captures keys before any other
handler. AgentView's main input handler is disabled while the dialog is open
via `isActive: !showCreateSessionDialog` at lines 4553 and 5451.
