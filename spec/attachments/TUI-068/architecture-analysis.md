# TUI-068: Session-Work Unit State Management Architecture Analysis

## Problem Statement

When a user clicks "Close Session", the session is destroyed correctly, but when they press "/" to create a new session, it immediately re-attaches to the same work unit. This reveals a fundamental architectural issue with how work unit context is tracked across the application.

## Current Architecture (Problematic)

### Three Places Tracking Work Unit Context

#### 1. BoardView (Local React State)
```typescript
const [selectedWorkUnit, setSelectedWorkUnit] = useState<any>(null);
```
- **Purpose**: UI navigation + passed as prop to AgentView
- **Lifecycle**: Persists while BoardView is mounted, NOT cleared on AgentView exit

#### 2. fspecStore (Zustand)
```typescript
// State
currentWorkUnitId: string | null;
sessionAttachments: Map<string, string>; // workUnitId → sessionId

// Actions
setCurrentWorkUnitId: (workUnitId: string | null) => void;
attachSession: (workUnitId: string, sessionId: string) => void;
detachSession: (workUnitId: string) => void;

// Selectors
getCurrentWorkUnitId: () => string | null;
getAttachedSession: (workUnitId: string) => string | undefined;
getWorkUnitBySession: (sessionId: string) => string | undefined;
```

#### 3. sessionStore (Zustand)
```typescript
// State
currentWorkUnitId: string | null;
currentWorkUnitStatus: string | null;

// Actions
setCurrentWorkUnit: (workUnitId: string | null, status: string | null) => void;
```

### Problems

1. **Duplicate State**: `currentWorkUnitId` exists in BOTH `fspecStore` and `sessionStore`
2. **Unclear Ownership**: No single source of truth for "what work unit is the user working on"
3. **Prop Drilling**: `workUnitId` passed from BoardView → AgentView as prop
4. **Exit Behavior Unclear**: Which state needs clearing on exit? Detach vs Close?
5. **IPC Updates Fragmented**: IPC goes to `fspecStore.attachSession()` but `sessionStore` also has work unit state
6. **Bug Manifestation**: `BoardView.selectedWorkUnit` persists across AgentView exit, causing auto-attach on next session

## Root Cause of Bug

```
1. User on board, selectedWorkUnit = TOOL-014
2. User presses "/" → AgentView opens with workUnitId prop = TOOL-014
3. Session created, attached to TOOL-014
4. User clicks "Close Session"
   └── Session destroyed ✅
   └── fspecStore.sessionAttachments cleared ✅
   └── BoardView.selectedWorkUnit = TOOL-014 (NOT cleared) ❌
5. User presses "/" again
   └── AgentView opens with workUnitId prop = TOOL-014 (from unchanged selectedWorkUnit)
   └── New session auto-attaches to TOOL-014 ← BUG
```

## Proposed Architecture

### Design Principles

1. **Single Source of Truth**: One store owns the "current work unit context"
2. **Separation of Concerns**: UI state (navigation) vs Domain state (session context)
3. **Clear Lifecycle**: Explicit rules for when state is set/cleared
4. **No Prop Drilling**: Components read from store, not props

### New State Ownership

```
sessionStore (SESSION CONTEXT - Single Source of Truth)
├── currentSessionId: string | null
├── currentWorkUnitId: string | null      ← THE work unit we're working on
└── currentWorkUnitStatus: string | null

fspecStore (PERSISTENCE & MULTI-SESSION TRACKING)
└── sessionAttachments: Map<workUnitId, sessionId>  ← For IPC, background sessions

BoardView (UI STATE ONLY)
└── selectedWorkUnit: local state for keyboard navigation highlight
    (NOT used for session context)
```

### Removals from fspecStore

- `currentWorkUnitId` (duplicate of sessionStore)
- `setCurrentWorkUnitId()`
- `getCurrentWorkUnitId()`

### Removals from AgentViewProps

- `workUnitId` prop (read from sessionStore instead)

### New Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. USER SELECTS WORK UNIT ON BOARD                                  │
├─────────────────────────────────────────────────────────────────────┤
│ BoardView:                                                          │
│   setSelectedWorkUnit(workUnit)  // UI highlight only               │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 2. USER PRESSES ENTER/SLASH TO START SESSION                        │
├─────────────────────────────────────────────────────────────────────┤
│ BoardView:                                                          │
│   sessionStore.setCurrentWorkUnit(workUnit.id, workUnit.status)     │
│   setViewMode('agent')                                              │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 3. AGENTVIEW MOUNTS                                                  │
├─────────────────────────────────────────────────────────────────────┤
│ AgentView:                                                          │
│   const workUnitId = useSessionStore(s => s.currentWorkUnitId)      │
│   // Creates session                                                │
│   fspecStore.attachSession(workUnitId, sessionId)                   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 4. AI CHANGES WORK UNIT VIA IPC                                      │
├─────────────────────────────────────────────────────────────────────┤
│ IPC Handler (BoardView):                                            │
│   fspecStore.attachSession(newWorkUnitId, sessionId)                │
│   sessionStore.setCurrentWorkUnit(newWorkUnitId, status)            │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 5. USER EXITS AGENTVIEW (Detach OR Close)                            │
├─────────────────────────────────────────────────────────────────────┤
│ ALWAYS:                                                             │
│   sessionStore.setCurrentWorkUnit(null, null)  // Clear context     │
│                                                                     │
│ IF CLOSE:                                                           │
│   sessionManagerDestroy(sessionId)                                  │
│   fspecStore.detachSession(workUnitId)                              │
│                                                                     │
│ RETURN TO BOARD:                                                    │
│   onExit()  // No special handling needed                           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 6. USER PRESSES "/" FOR NEW SESSION                                  │
├─────────────────────────────────────────────────────────────────────┤
│ BoardView:                                                          │
│   // Only set work unit context if user explicitly selected one     │
│   if (selectedWorkUnit) {                                           │
│     sessionStore.setCurrentWorkUnit(selectedWorkUnit.id, status)    │
│   }                                                                 │
│   setViewMode('agent')                                              │
│                                                                     │
│ AgentView:                                                          │
│   const workUnitId = useSessionStore(s => s.currentWorkUnitId)      │
│   // workUnitId is null if no work unit selected → no auto-attach   │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Tasks

### Phase 1: Remove Duplicates from fspecStore

1. Remove `currentWorkUnitId` state
2. Remove `setCurrentWorkUnitId()` action
3. Remove `getCurrentWorkUnitId()` selector
4. Keep `sessionAttachments` map (needed for multi-session tracking)

### Phase 2: Update AgentView

1. Remove `workUnitId` from `AgentViewProps`
2. Read `currentWorkUnitId` from `sessionStore` instead
3. Update all usages of the prop to use the store selector

### Phase 3: Update BoardView

1. Call `sessionStore.setCurrentWorkUnit()` when entering agent mode
2. Only set work unit if user explicitly selected one (Enter on work unit)
3. For "/" without selection, leave currentWorkUnitId as null

### Phase 4: Update Exit Handlers

1. AgentView `handleExitChoice`: ALWAYS call `sessionStore.setCurrentWorkUnit(null, null)`
2. This applies to both Detach and Close
3. Close additionally destroys session and detaches from fspecStore

### Phase 5: Update Tests

1. Update all tests that use `workUnitId` prop
2. Update tests that use `fspecStore.currentWorkUnitId`
3. Add new tests for the clear-on-exit behavior

## Files to Modify

- `src/tui/store/fspecStore.ts` - Remove duplicate state
- `src/tui/store/sessionStore.ts` - Already has the state, may need updates
- `src/tui/components/AgentView.tsx` - Remove prop, use store
- `src/tui/components/BoardView.tsx` - Set store state before entering agent mode
- `src/tui/components/__tests__/*.test.tsx` - Update tests
- `src/tui/store/__tests__/*.test.ts` - Update tests

## Benefits

1. **Single Source of Truth**: `sessionStore.currentWorkUnitId` is THE authority
2. **Clear Lifecycle**: Always cleared on AgentView exit
3. **No Prop Drilling**: AgentView reads from store
4. **Consistent Behavior**: Detach and Close both clear context
5. **Bug Fixed**: New session won't auto-attach to stale work unit

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing tests | Run full test suite after each phase |
| IPC handler inconsistency | Update IPC handler to use sessionStore |
| Background sessions | fspecStore.sessionAttachments still tracks these |
| Performance (store reads) | Zustand selectors are memoized, no concern |
