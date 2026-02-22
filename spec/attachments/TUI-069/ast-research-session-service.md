# AST Research: Session Service Code Smells

## Research Target
- `src/tui/services/sessionService.ts`
- `src/tui/store/fspecStore.ts`
- `src/tui/components/AgentView.tsx`

## Code Smells Identified

### 1. Missing Error Handling in sessionService.ts
**Location:** `attachToWorkUnit` and `detachFromWorkUnit` functions
**Issue:** No try/catch blocks, no rollback on failure
**Fix:** Added error handling with rollback

### 2. Console Statements in fspecStore.ts
**Location:** `loadData` function error handler
**Issue:** Using `console.error` and `console.warn` instead of logger
**Fix:** Replaced with `logger.error` and `logger.warn`

### 3. Incomplete Facade Pattern
**Location:** sessionService.ts exports
**Issue:** Missing `getAttachedWorkUnit` function - forcing components to access store directly
**Fix:** Added `getAttachedWorkUnit` export function

### 4. Direct Store Access in AgentView.tsx
**Location:** Line 1103-1106 (now removed)
**Issue:** Direct `useFspecStore(state => state.getWorkUnitBySession)` selector
**Fix:** Replaced with `getAttachedWorkUnit` import from sessionService

### 5. Hardcoded Title in attachToWorkUnit
**Location:** Line 514-518
**Issue:** Using `workUnitId` as title placeholder
**Fix:** Added optional `title` parameter, callers now pass actual title

## Functions Modified

### sessionService.ts
- `attachToWorkUnit(sessionId, workUnitId, status, title?)` - Added error handling with rollback, added optional title
- `detachFromWorkUnit(sessionId)` - Added error handling with rollback
- `getAttachedWorkUnit(sessionId)` - NEW: Facade function for read access

### fspecStore.ts
- `loadData()` - Replaced console.* with logger.*

### AgentView.tsx
- Removed `getWorkUnitBySession` store selector
- Added `getAttachedWorkUnit` import from sessionService
- Updated all `attachToWorkUnit` calls to pass title parameter
