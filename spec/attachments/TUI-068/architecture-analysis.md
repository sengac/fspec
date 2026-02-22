# TUI-068: Session Lifecycle Service Architecture

## Problem Statement

Session lifecycle management is currently scattered across multiple components:
- **AgentView.tsx** (8000+ lines) directly calls Rust NAPI, stores, and services
- **BoardView.tsx** IPC handler directly manipulates stores
- **globalStreamListener.ts** directly updates stores
- **Duplicate state**: `currentWorkUnitId` exists in both `fspecStore` and `sessionStore`

This violates SOLID/DRY principles and causes bugs like stale work unit auto-attachment.

## Solution: Service Facade Pattern

### Design Principles

1. **Single Source of Truth**: `sessionStore.currentWorkUnitId` is THE authority for current work unit
2. **Facade Pattern**: `sessionService.ts` is the ONLY entry point for session lifecycle operations
3. **Thin Components**: AgentView/BoardView only import sessionService, not stores or NAPI
4. **Orchestration**: Service coordinates all stores, NAPI, and context updates atomically

## New Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         COMPONENTS (Thin)                           │
├─────────────────────────────────────────────────────────────────────┤
│  AgentView.tsx         BoardView.tsx         globalStreamListener   │
│       │                     │                        │              │
│       └─────────────────────┼────────────────────────┘              │
│                             │                                       │
│                             ▼                                       │
│              ┌──────────────────────────────┐                       │
│              │     sessionService.ts        │  ← FACADE             │
│              │  (Single Entry Point)        │                       │
│              └──────────────────────────────┘                       │
│                             │                                       │
│         ┌───────────────────┼───────────────────┐                   │
│         ▼                   ▼                   ▼                   │
│  ┌─────────────┐   ┌─────────────────┐   ┌──────────────┐          │
│  │ fspecStore  │   │  sessionStore   │   │  Rust NAPI   │          │
│  │ (internal)  │   │   (internal)    │   │  (internal)  │          │
│  └─────────────┘   └─────────────────┘   └──────────────┘          │
└─────────────────────────────────────────────────────────────────────┘
```

## Service API

### sessionService.ts - New Functions

```typescript
// Create session (existing - enhanced)
export async function createSession(options: CreateSessionOptions): Promise<CreateSessionResult>

// Create isolated session (existing)
export async function createIsolatedSession(options: CreateSessionOptions): Promise<CreateIsolatedSessionResult>

// NEW: Destroy session - orchestrates ALL cleanup
export async function destroySession(sessionId: string): Promise<void>
// Internally:
//   1. sessionManagerDestroy(sessionId) - Rust NAPI
//   2. fspecStore.detachSession(workUnitId) - store cleanup
//   3. sessionStore.setCurrentWorkUnit(null, null) - context clear
//   4. GlobalSessionStreamManager.unsubscribe(sessionId) - stream cleanup

// NEW: Attach session to work unit
export function attachToWorkUnit(sessionId: string, workUnitId: string): void
// Internally:
//   1. fspecStore.attachSession(workUnitId, sessionId)
//   2. sessionStore.setCurrentWorkUnit(workUnitId, status)
//   3. workUnitContextService.setWorkUnitContext(sessionId, context)

// NEW: Detach session from work unit
export function detachFromWorkUnit(sessionId: string): void
// Internally:
//   1. fspecStore.detachSession(workUnitId)
//   2. sessionStore.setCurrentWorkUnit(null, null)
//   3. workUnitContextService.setWorkUnitContext(sessionId, null)

// Existing - for isolated sessions (UI prompts user to choose)
export function mergeSessionChanges(repoPath: string, sessionId: string): MergeResultJs
export function discardSessionChanges(repoPath: string, sessionId: string): DiscardResultJs
```

## Store Changes

### fspecStore.ts - Removals

```typescript
// REMOVE these (duplicate of sessionStore):
- currentWorkUnitId: string | null
- setCurrentWorkUnitId: (workUnitId: string | null) => void
- getCurrentWorkUnitId: () => string | null

// KEEP these (different purpose - multi-session tracking):
+ sessionAttachments: Map<string, string>  // workUnitId → sessionId
+ attachSession(workUnitId, sessionId)
+ detachSession(workUnitId)
+ getAttachedSession(workUnitId)
+ hasAttachedSession(workUnitId)
+ getWorkUnitBySession(sessionId)
+ clearAllSessionAttachments()
```

### sessionStore.ts - Single Source of Truth

```typescript
// KEEP - this is THE source of truth:
currentWorkUnitId: string | null
currentWorkUnitStatus: string | null
setCurrentWorkUnit(workUnitId, status)
```

## Component Changes

### AgentView.tsx

**Before:**
```typescript
import { sessionManagerDestroy } from '@sengac/codelet-napi';
import { useFspecStore } from '../store/fspecStore';
import { useSessionStore } from '../store/sessionStore';

// Direct calls scattered throughout 8000+ lines
sessionManagerDestroy(sessionId);
attachSessionToWorkUnit(workUnitId, sessionId);
detachSessionFromWorkUnit(workUnitId);
setCurrentWorkUnit(workUnitId, status);
```

**After:**
```typescript
import { 
  createSession,
  destroySession,
  attachToWorkUnit,
  detachFromWorkUnit 
} from '../services/sessionService';

// Clean single calls
await destroySession(sessionId);
attachToWorkUnit(sessionId, workUnitId);
detachFromWorkUnit(sessionId);
```

### BoardView.tsx IPC Handler

**Before:**
```typescript
if (message.type === 'work-unit-changed') {
  useFspecStore.getState().attachSession(workUnitId, sessionId);  // Direct store call
}
```

**After:**
```typescript
if (message.type === 'work-unit-changed') {
  attachToWorkUnit(sessionId, workUnitId);  // Service call
}
```

### globalStreamListener.ts

**Before:**
```typescript
useSessionStore.getState().setCurrentWorkUnit(currentWorkUnitId, status);
sessionSetWorkUnitContext(activeSessionId, workUnitId, title, status);
```

**After:**
```typescript
// Use service for context sync
syncWorkUnitContext(sessionId, workUnitId);  // New service function
```

## Isolated Session Workflow

For isolated sessions (with git worktrees), the workflow is:

```
User clicks "Close Session"
         │
         ▼
┌─────────────────────────────────┐
│  UI checks: isIsolated?         │
└─────────────────────────────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
  YES        NO
    │         │
    ▼         │
┌───────────┐ │
│ UI Prompt │ │
│ Merge or  │ │
│ Discard?  │ │
└───────────┘ │
    │         │
┌───┴───┐     │
▼       ▼     │
Merge  Discard│
│       │     │
▼       ▼     │
mergeSessionChanges()    │
   or                    │
discardSessionChanges()  │
│       │     │
└───┬───┘     │
    │         │
    ▼         ▼
┌─────────────────────────────────┐
│     destroySession(sessionId)   │
│  (handles all cleanup)          │
└─────────────────────────────────┘
```

## Data Flow: Create Session with Work Unit

```
1. User selects work unit "TOOL-014" on board
2. User presses Enter

BoardView:
   └── sessionStore.setCurrentWorkUnit("TOOL-014", "specifying")
   └── setViewMode('agent')

AgentView mounts:
   └── createSession({ modelPath, project })
       └── [internal] persistenceCreateSessionWithProvider()
       └── [internal] GlobalSessionStreamManager.subscribeToSession()
       └── [internal] sessionManagerCreateWithId()
   └── attachToWorkUnit(sessionId, "TOOL-014")
       └── [internal] fspecStore.attachSession("TOOL-014", sessionId)
       └── [internal] sessionStore.setCurrentWorkUnit("TOOL-014", "specifying")
       └── [internal] workUnitContextService.setWorkUnitContext(sessionId, context)
```

## Data Flow: Close Session

```
1. User clicks "Close Session" in AgentView

AgentView:
   └── destroySession(sessionId)
       └── [internal] Get attached work unit from fspecStore
       └── [internal] sessionManagerDestroy(sessionId)
       └── [internal] fspecStore.detachSession(workUnitId)
       └── [internal] sessionStore.setCurrentWorkUnit(null, null)
       └── [internal] GlobalSessionStreamManager.unsubscribeFromSession(sessionId)
   └── onExit() → returns to BoardView

BoardView:
   └── selectedWorkUnit stays for UI highlight (different concern)
   └── User presses "/" → NO auto-attach (sessionStore.currentWorkUnitId is null)
```

## Data Flow: AI Changes Work Unit via IPC

```
1. AI runs `fspec update-work-unit-status AUTH-001 implementing`
2. fspec CLI sends IPC message: { type: 'work-unit-changed', payload: { workUnitId: 'AUTH-001', sessionId } }

BoardView IPC handler:
   └── attachToWorkUnit(sessionId, "AUTH-001")
       └── [internal] fspecStore.attachSession("AUTH-001", sessionId)
       └── [internal] sessionStore.setCurrentWorkUnit("AUTH-001", "implementing")
       └── [internal] workUnitContextService.setWorkUnitContext(sessionId, context)
   └── loadData() → refresh board display
```

## Testing Strategy

### Unit Tests for sessionService

```typescript
describe('sessionService', () => {
  beforeEach(() => {
    // Reset stores
    useFspecStore.setState({ sessionAttachments: new Map() });
    useSessionStore.getState().reset();
  });

  describe('attachToWorkUnit', () => {
    it('should update all stores atomically', () => {
      attachToWorkUnit('session-123', 'TOOL-014');
      
      expect(useFspecStore.getState().getAttachedSession('TOOL-014')).toBe('session-123');
      expect(useSessionStore.getState().currentWorkUnitId).toBe('TOOL-014');
    });
  });

  describe('detachFromWorkUnit', () => {
    it('should clear all stores atomically', () => {
      attachToWorkUnit('session-123', 'TOOL-014');
      detachFromWorkUnit('session-123');
      
      expect(useFspecStore.getState().hasAttachedSession('TOOL-014')).toBe(false);
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  describe('destroySession', () => {
    it('should orchestrate full cleanup', async () => {
      // Setup
      attachToWorkUnit('session-123', 'TOOL-014');
      
      // Act
      await destroySession('session-123');
      
      // Assert all cleanup happened
      expect(useFspecStore.getState().hasAttachedSession('TOOL-014')).toBe(false);
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
      // Verify NAPI was called (mock)
    });
  });
});
```

## Implementation Phases

### Phase 1: Add New Service Functions
1. Add `destroySession()` to sessionService.ts
2. Add `attachToWorkUnit()` to sessionService.ts
3. Add `detachFromWorkUnit()` to sessionService.ts
4. Add unit tests for new functions

### Phase 2: Update Callers
1. Update AgentView.tsx to use service functions
2. Update BoardView.tsx IPC handler to use service functions
3. Update globalStreamListener.ts to use service functions

### Phase 3: Remove Duplicate State
1. Remove `currentWorkUnitId` from fspecStore
2. Remove `setCurrentWorkUnitId()` from fspecStore
3. Remove `getCurrentWorkUnitId()` from fspecStore
4. Update any remaining direct usages

### Phase 4: Cleanup & Verification
1. Remove unused imports from AgentView/BoardView
2. Run full test suite
3. Manual testing of all session workflows

## Files to Modify

- `src/tui/services/sessionService.ts` - Add new lifecycle functions
- `src/tui/store/fspecStore.ts` - Remove duplicate state
- `src/tui/components/AgentView.tsx` - Use service, remove direct store/NAPI calls
- `src/tui/components/BoardView.tsx` - Update IPC handler
- `src/tui/store/globalStreamListener.ts` - Use service for context sync
- `src/tui/services/__tests__/sessionService.test.ts` - Add tests

## Benefits

1. **Single Responsibility**: sessionService owns session lifecycle
2. **DRY**: No duplicate currentWorkUnitId state
3. **Testable**: Mock stores, test service in isolation
4. **Maintainable**: One place to change session logic
5. **Bug Fixed**: New sessions won't auto-attach to stale work units
