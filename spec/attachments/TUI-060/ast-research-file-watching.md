# AST Research: File Watching Implementation

## Overview
Analysis of current file watching implementation in TUI components to identify code for refactoring into a reusable hook.

## Current Implementation Location

### BoardView.tsx - Inline Chokidar Usage

**File:** `src/tui/components/BoardView.tsx`

Two instances of inline chokidar file watching:

1. **Work Units Watcher** (line ~144):
```typescript
const watcher = chokidar.watch(workUnitsPath, {
  ignoreInitial: true,
  persistent: false,
});
watcher.on('change', () => {
  void loadData();
});
```

2. **Git Stash Watcher** (line ~176):
```typescript
const watcher = chokidar.watch(stashPath, {
  ignoreInitial: true,
  persistent: false,
});
watcher.on('change', () => {
  void loadCheckpointCounts();
});
```

## Components Needing the Hook

1. **BoardView** - Already has the implementation (refactor to use hook)
2. **AgentView** - Needs to add the hook for realtime work unit status updates
3. **SessionHeader** - Consumes work unit data from props (no direct hook usage)

## Zustand Store Integration

**File:** `src/tui/store/fspecStore.ts`

- `loadData()` function reloads work units from JSON file
- `workUnits` state contains current work unit data including status
- `getWorkUnitBySession()` retrieves work unit ID for a session

## Existing Hooks Directory

**Location:** `src/tui/hooks/`

Existing hooks:
- `useWorkUnitContext.ts` - Manages work unit context for sessions

## Proposed Hook Structure

```typescript
// src/tui/hooks/useWorkUnitsWatcher.ts
export function useWorkUnitsWatcher(options?: {
  enabled?: boolean;
}) {
  // Uses chokidar to watch spec/work-units.json
  // Calls loadData() on file change
  // Cleans up watcher on unmount
}
```

## Test Files to Update

- `src/tui/__tests__/BoardView-file-watchers.test.tsx`
- `src/tui/__tests__/BoardView-git-watcher-fix.test.tsx`
