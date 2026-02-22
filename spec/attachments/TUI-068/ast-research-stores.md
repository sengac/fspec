# AST Research: Store State Management

## Summary

Analyzed the state management stores for TUI-068 refactoring.

## fspecStore.ts

### State with currentWorkUnitId (TO BE REMOVED)
```typescript
// Line 74: currentWorkUnitId state
currentWorkUnitId: string | null;

// Line 103: setCurrentWorkUnitId action
setCurrentWorkUnitId: (workUnitId: string | null) => void;

// Line 113: getCurrentWorkUnitId selector
getCurrentWorkUnitId: () => string | null;

// Line 131: Initial state
currentWorkUnitId: null,

// Lines 448-452: setCurrentWorkUnitId implementation
setCurrentWorkUnitId: (workUnitId: string | null) => {
  set(state => {
    state.currentWorkUnitId = workUnitId;
  });
},

// Lines 477-479: getCurrentWorkUnitId implementation
getCurrentWorkUnitId: () => {
  return get().currentWorkUnitId;
},
```

### State to KEEP
- `sessionAttachments: Map<string, string>` - for multi-session tracking
- `attachSession`, `detachSession`, `clearAllSessionAttachments` actions
- `getAttachedSession`, `hasAttachedSession`, `getWorkUnitBySession` selectors

## sessionStore.ts

### State (ALREADY EXISTS - Single Source of Truth)
```typescript
// Line 54: currentWorkUnitId
currentWorkUnitId: string | null;

// Line 57: currentWorkUnitStatus
currentWorkUnitStatus: string | null;

// Line 111-114: setCurrentWorkUnit action
setCurrentWorkUnit: (
  workUnitId: string | null,
  workUnitStatus: string | null
) => void;

// Lines 236-246: setCurrentWorkUnit implementation
setCurrentWorkUnit: (
  workUnitId: string | null,
  workUnitStatus: string | null
) => {
  set(state => {
    state.currentWorkUnitId = workUnitId;
    state.currentWorkUnitStatus = workUnitStatus;
  });
},
```

### Hooks (ALREADY EXISTS)
```typescript
// Lines 375-376
export const useCurrentWorkUnitId = () =>
  useSessionStore(state => state.currentWorkUnitId);

// Lines 378-379
export const useCurrentWorkUnitStatus = () =>
  useSessionStore(state => state.currentWorkUnitStatus);
```

## globalStreamListener.ts

Uses `currentWorkUnitId` from somewhere (lines 68-80) - needs investigation for IPC updates.

## Findings

1. **fspecStore.ts**: Has duplicate `currentWorkUnitId` state that needs REMOVAL
2. **sessionStore.ts**: Has `currentWorkUnitId` that is the SINGLE SOURCE OF TRUTH
3. **sessionStore.ts**: Already has `setCurrentWorkUnit` action
4. **sessionStore.ts**: Already has `useCurrentWorkUnitId` hook

## Changes Required

### Remove from fspecStore.ts:
- `currentWorkUnitId: string | null` from interface (line 74)
- `setCurrentWorkUnitId: (workUnitId: string | null) => void` from interface (line 103)
- `getCurrentWorkUnitId: () => string | null` from interface (line 113)
- `currentWorkUnitId: null` from initial state (line 131)
- `setCurrentWorkUnitId` implementation (lines 448-452)
- `getCurrentWorkUnitId` implementation (lines 477-479)

### Keep in fspecStore.ts:
- All `sessionAttachments` related code (for multi-session/IPC tracking)

### Update AgentView.tsx:
- Remove `workUnitId` prop
- Use `useCurrentWorkUnitId()` from sessionStore

### Update BoardView.tsx:
- Call `sessionStore.setCurrentWorkUnit()` before entering agent mode

### Update AgentView.tsx exit handler:
- Always call `sessionStore.setCurrentWorkUnit(null, null)` on exit
