# AST Research: Session Attachment for Work Units

## Summary

Research conducted to understand current architecture for implementing session attachment to work units.

## Key Findings

### 1. fspecStore.ts (src/tui/store/fspecStore.ts)

**Current State Structure:**
```typescript
interface FspecState {
  workUnits: WorkUnit[];
  epics: Epic[];
  stashes: unknown[];
  stagedFiles: FileStatusWithChangeType[];
  unstagedFiles: FileStatusWithChangeType[];
  checkpointCounts: { manual: number; auto: number };
  isLoaded: boolean;
  error: string | null;
  cwd: string;
}
```

**Required Additions:**
- `sessionAttachments: Map<string, string>` - Maps workUnitId → sessionId
- `currentWorkUnitId: string | null` - Tracks which work unit user entered
- `attachSession(workUnitId: string, sessionId: string): void`
- `detachSession(workUnitId: string): void`
- `getAttachedSession(workUnitId: string): string | undefined`

### 2. BoardView.tsx (src/tui/components/BoardView.tsx)

**Current Flow (lines 431-439):**
```typescript
onEnter={() => {
  if (focusedPanel === 'board') {
    const currentColumn = groupedWorkUnits[focusedColumnIndex];
    if (currentColumn.units.length > 0) {
      const workUnit = currentColumn.units[selectedWorkUnitIndex];
      setSelectedWorkUnit(workUnit);  // Line 437: Stores work unit
      setViewMode('agent');           // Line 438: Switches to agent view
    }
  }
}}
```

**Current AgentView rendering (lines 342-344):**
```typescript
<AgentView
  onExit={() => setViewMode('board')}
/>
```

**Required Changes:**
- Pass `workUnitId={selectedWorkUnit?.id}` prop to AgentView

### 3. UnifiedBoardLayout.tsx

**Work unit rendering location:** Lines 451-455 (based on previous exploration)
- Currently uses `⏩` emoji for last changed work unit
- Need to add `🟢` emoji prefix when `sessionAttachments.has(workUnit.id)`

### 4. AgentView.tsx

**Session creation:** Uses `persistenceCreateSessionWithProvider()` (NAPI)
**Session resume:** Uses `persistenceGetSessionMessages()` (NAPI)

**Required Changes:**
- Accept `workUnitId?: string` prop
- On mount: Check `store.getAttachedSession(workUnitId)`
- If session found: Call `persistenceGetSessionMessages()` to restore
- On first message (new session): Call `store.attachSession(workUnitId, newSessionId)`
- /detach command: Call `store.detachSession(workUnitId)`, then clear conversation
- /resume command: After selection, call `store.attachSession(workUnitId, selectedSessionId)`

## Integration Points

1. **Store → UnifiedBoardLayout**: Read `sessionAttachments` for visual indicator
2. **BoardView → AgentView**: Pass `workUnitId` prop
3. **AgentView → Store**: Call `attachSession`/`detachSession` actions
4. **Existing NAPI**: Reuse `persistenceCreateSessionWithProvider`, `persistenceGetSessionMessages`, `persistenceListSessions`

## Files to Modify

1. `src/tui/store/fspecStore.ts` - Add session attachment state and actions
2. `src/tui/components/BoardView.tsx` - Pass workUnitId to AgentView
3. `src/tui/components/UnifiedBoardLayout.tsx` - Render 🟢 indicator
4. `src/tui/components/AgentView.tsx` - Handle session attachment logic
