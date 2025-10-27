# Interactive TUI Foundation - Architecture & Implementation Guide

**Work Unit**: FOUNDATION-001
**Epic**: Interactive TUI Foundation
**Created**: 2025-10-27
**Purpose**: Document the architecture, patterns, and implementation strategy for fspec's interactive terminal UI based on cage's proven patterns

---

## Executive Summary

This document outlines the architectural foundation for building an interactive Terminal User Interface (TUI) for fspec, based on patterns and components from the [cage](https://github.com/sengac/cage) project. The goal is to transform fspec from a pure CLI tool into a full-featured interactive application that allows users to manage all aspects of ACDD workflow without typing commands.

**Key Technologies**:
- **Ink** - React for terminal interfaces
- **@inkjs/ui** - Pre-built UI components
- **Zustand** - State management (lightweight, no boilerplate)
- **Commander** - CLI argument parsing (already in use)
- **TypeScript** - 100% type safety with ES modules

**Architecture Pattern**: View-based navigation with shared components, modal input modes (vim-style), and reactive state management.

---

## Cage Architecture Analysis

### What is Cage?

**Cage** (Code Alignment Guard Engine) is a developer productivity tool that observes and analyzes Claude Code interactions in real-time through a sophisticated TUI built with Ink.

**Key Stats**:
- **Monorepo structure**: cli, backend, hooks, shared packages
- **39+ Gherkin features** with 327+ scenarios
- **100% TypeScript** with strict type safety
- **Vitest** for testing (including TUI components)
- **Uses fspec** for ACDD workflow (dogfooding)

### Reusable Architectural Patterns

#### 1. ViewManager Pattern (Navigation System)

**Location**: `cage/packages/cli/src/core/navigation/ViewManager.tsx`

**How it works**:
```typescript
interface ViewDefinition {
  id: string;
  component: React.FC<ViewProps>;
  metadata: {
    title: string;
    subtitle?: string;
    footer?: string;
    showDefaultFooter?: boolean;
    showServerStatus?: boolean;
  };
}

const viewDefinitions: Record<string, ViewDefinition> = {
  main: { id: 'main', component: MainMenuView, metadata: { ... } },
  board: { id: 'board', component: BoardView, metadata: { ... } },
  // ... etc
};
```

**Navigation mechanism**:
- Maintains `history` array (like browser history)
- `navigate(viewId)` pushes to history stack
- `goBack()` pops from history (or exits if at root)
- Context provides: `currentView`, `navigate`, `goBack`, `updateMetadata`

**Benefits**:
- Centralized view registry
- Automatic back button handling
- Consistent layout wrapping
- Metadata-driven footers and titles

#### 2. VirtualList Component (Performance-Optimized Lists)

**Location**: `cage/packages/cli/src/shared/components/ui/VirtualList.tsx`

**Features**:
```typescript
interface VirtualListProps<T> {
  items: T[];
  height: number; // Number of visible rows
  renderItem: (item: T, index: number, isSelected: boolean) => React.ReactNode;
  onSelect?: (item: T, index: number) => void;
  onFocus?: (item: T, index: number) => void;
  keyExtractor?: (item: T, index: number) => string;
  emptyMessage?: string;
  showScrollbar?: boolean;
  enableWrapAround?: boolean;
}
```

**Key implementation details**:
- **Windowing**: Only renders visible items (scrollOffset + height)
- **Keyboard navigation**:
  - `↑↓` / `jk` - Move selection
  - `g` / `G` - Jump to top/bottom
  - `PageUp` / `PageDown` - Jump by page
  - `Enter` - Select item
- **Scrollbar visualization**: Uses `figures` library (█ ▓ ▒ ░)
- **Wrap-around**: Optional circular navigation
- **Auto-scroll**: Keeps selected item visible

**Performance**:
- Handles 1000+ items smoothly
- No lag with large datasets
- Memory-efficient (only visible DOM)

#### 3. TaskList Component (Rich List Display)

**Location**: `cage/packages/cli/src/shared/components/ui/TaskList.tsx`

**Layouts**:
- **Minimal**: Single line, truncated text
- **Compact**: One line with icons + progress bar
- **Detailed**: Multi-line with timestamps, dependencies, tags

**Features**:
```typescript
interface Task {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed' | 'blocked';
  activeForm: string; // Present continuous form (e.g., "Running tests")
  priority: 'high' | 'medium' | 'low';
  progress: number; // 0-100
  duration: number | null;
  dependencies?: string[];
  assignee?: string;
  tags?: string[];
}
```

**Visual elements**:
- Status icons: ⏳ (pending), 🔄 (in_progress), ✅ (completed), 🚫 (blocked)
- Priority icons: 🔴 (high), 🟡 (medium), 🟢 (low)
- Progress bars: `█████████▓▓▓▓▓▓▓▓▓▓▓` (20 chars)
- Animated spinners: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏

**Filtering & Sorting**:
- Filter by: status, priority
- Sort by: priority, status, progress, created, updated

**Adaptation for fspec**:
- TaskList → **WorkUnitList**
- Task → **WorkUnit**
- Keep same visual language (icons, progress bars)
- Add fspec-specific fields (estimate, epic, Example Mapping state)

#### 4. FullScreenLayout (Consistent Layout System)

**Location**: `cage/packages/cli/src/shared/components/layout/FullScreenLayout.tsx`

**Structure**:
```
┌────────────────────────────────────────┐
│ Title                     [Server: ●]  │ ← Title bar
├────────────────────────────────────────┤
│                                        │
│                                        │
│          Content Area                  │ ← Flexible content
│          (flexGrow: 1)                 │
│                                        │
│                                        │
├────────────────────────────────────────┤
│ ↑↓ Navigate | ↵ Select | ESC Back     │ ← Footer (shortcuts)
└────────────────────────────────────────┘
```

**Features**:
- Auto-sizing (uses `useStdoutDimensions`)
- Title/subtitle support
- Custom or default footer
- Server status indicator (optional)
- Back button integration (ESC key)
- Breadcrumb navigation in title

#### 5. Input Mode System (Modal Editing)

**Location**: `cage/packages/cli/src/shared/contexts/InputContext.tsx`

**Modes** (like vim):
- **normal**: Navigation, selection (default)
- **insert**: Text input (forms, search boxes)
- **command**: Command palette (`:` prefix)

**useSafeInput hook**:
```typescript
useSafeInput(
  (input, key) => {
    if (key.upArrow) { /* handle */ }
    if (key.return) { /* handle */ }
  },
  {
    isActive: true,
    activeModes: ['normal'], // Only active in normal mode
    respectFocus: false,     // Ignore focus manager
  }
);
```

**Benefits**:
- Prevents input conflicts between components
- Modal editing prevents accidental actions
- Clear mental model (vim users already know it)

#### 6. Zustand State Management

**Location**: `cage/packages/cli/src/shared/stores/appStore.ts`

**Pattern**:
```typescript
interface AppState {
  // State
  events: Event[];
  serverStatus: 'running' | 'stopped' | 'unknown';
  hooksStatus: HooksStatus;

  // Actions
  fetchLatestEvents: () => Promise<void>;
  addEvent: (event: Event) => void;
  clearEvents: () => void;
  refreshHooksStatus: () => Promise<void>;
}

export const useAppStore = create<AppState>()(
  immer((set, get) => ({
    events: [],
    serverStatus: 'unknown',

    fetchLatestEvents: async () => {
      const events = await api.getLatestEvents();
      set(state => { state.events = events; });
    },

    addEvent: (event) => set(state => {
      state.events.push(event);
    }),

    clearEvents: () => set(state => {
      state.events = [];
    }),
  }))
);
```

**Usage in components**:
```typescript
const events = useAppStore(state => state.events);
const addEvent = useAppStore(state => state.addEvent);
```

**Benefits**:
- No boilerplate (unlike Redux)
- Immer for immutable updates
- Simple selector pattern
- TypeScript-friendly

#### 7. Singleton Services Pattern

**Location**: `cage/packages/cli/src/features/events/services/stream-service.ts`

**Pattern**:
```typescript
class StreamService {
  private static instance: StreamService;

  static getInstance() {
    if (!this.instance) {
      this.instance = new StreamService();
    }
    return this.instance;
  }

  start() {
    // Connect to SSE, start polling, etc.
    // Update store directly
    useAppStore.getState().addEvent(event);
  }

  stop() {
    // Cleanup
  }
}
```

**Usage**:
```typescript
useEffect(() => {
  const service = StreamService.getInstance();
  service.start();
  return () => service.stop();
}, []);
```

**Benefits**:
- Separation of concerns (services ≠ components)
- Components never call services directly
- Services update store, components read reactively
- No polling in components

---

## Proposed fspec TUI Architecture

### Project Structure

```
fspec/
├── src/
│   ├── tui/                          # NEW: All TUI code
│   │   ├── app/
│   │   │   └── App.tsx               # Main TUI app
│   │   ├── core/
│   │   │   ├── navigation/
│   │   │   │   ├── ViewManager.tsx
│   │   │   │   ├── viewDefinitions.tsx
│   │   │   │   └── types.ts
│   │   │   └── theme/
│   │   │       └── useTheme.tsx
│   │   ├── views/
│   │   │   ├── MainMenuView.tsx
│   │   │   ├── BoardView.tsx
│   │   │   ├── WorkUnitDetailView.tsx
│   │   │   ├── WorkUnitEditorView.tsx
│   │   │   ├── ExampleMappingView.tsx
│   │   │   └── ... (more views)
│   │   ├── components/
│   │   │   ├── KanbanBoard.tsx
│   │   │   ├── WorkUnitCard.tsx
│   │   │   ├── WorkUnitList.tsx
│   │   │   └── ... (more components)
│   │   ├── shared/
│   │   │   ├── components/
│   │   │   │   ├── VirtualList.tsx      # From cage
│   │   │   │   ├── FullScreenLayout.tsx # From cage
│   │   │   │   └── ... (more from cage)
│   │   │   ├── hooks/
│   │   │   │   └── useSafeInput.tsx     # From cage
│   │   │   └── contexts/
│   │   │       └── InputContext.tsx     # From cage
│   │   └── stores/
│   │       └── fspecStore.ts            # Zustand store
│   ├── commands/                     # EXISTING: CLI commands
│   │   └── tui.ts                    # NEW: Launch TUI command
│   ├── utils/                        # EXISTING
│   ├── types/                        # EXISTING
│   └── index.ts                      # EXISTING
├── package.json
└── tsconfig.json
```

### Technology Stack

**Required dependencies** (to be added):
```json
{
  "dependencies": {
    "ink": "^6.3.0",
    "@inkjs/ui": "latest",
    "zustand": "latest",
    "immer": "latest",
    "figures": "latest",
    "cli-spinners": "latest",
    "ink-spinner": "^5.0.0",
    "ink-table": "latest",
    "ink-use-stdout-dimensions": "latest",
    "react": "^19.1.1"
  },
  "devDependencies": {
    "@types/react": "^19.1.13",
    "ink-testing-library": "^4.0.0"
  }
}
```

**Already have** (from existing fspec):
- `commander` ✓
- `chalk` ✓
- `typescript` ✓
- `vitest` ✓

### Core Components to Copy from Cage

#### 1. **VirtualList.tsx** (Verbatim copy)
- Already battle-tested
- No changes needed
- Just import and use

#### 2. **FullScreenLayout.tsx** (Minor adaptation)
- Remove "server status" indicator (or repurpose for "fspec daemon")
- Keep title/footer pattern
- Keep breadcrumb navigation

#### 3. **InputContext.tsx** (Verbatim copy)
- Modal editing is universally useful
- No changes needed

#### 4. **useSafeInput.tsx** (Verbatim copy)
- Hook for keyboard input
- No changes needed

#### 5. **ViewManager.tsx** (Verbatim copy)
- Navigation pattern is identical
- No changes needed

#### 6. **TaskList.tsx** (Adapt → WorkUnitList.tsx)
- Change Task → WorkUnit interface
- Update icons for work unit types (story/bug/task)
- Add fspec-specific fields (estimate, epic, tags)
- Keep visual style (progress bars, colors, layout)

### Zustand Store Schema

```typescript
interface FspecStore {
  // State
  workUnits: WorkUnit[];
  epics: Epic[];
  features: FeatureFile[];
  tags: Tag[];
  config: FspecConfig;

  // UI State
  selectedWorkUnitId: string | null;
  selectedView: string;
  filterStatus: WorkUnitStatus | 'all';
  searchQuery: string;

  // Actions
  loadWorkUnits: () => Promise<void>;
  loadEpics: () => Promise<void>;
  loadFeatures: () => Promise<void>;

  updateWorkUnit: (id: string, updates: Partial<WorkUnit>) => Promise<void>;
  updateWorkUnitStatus: (id: string, status: WorkUnitStatus) => Promise<void>;
  createWorkUnit: (data: CreateWorkUnitData) => Promise<string>;
  deleteWorkUnit: (id: string) => Promise<void>;

  addRule: (workUnitId: string, rule: string) => Promise<void>;
  addExample: (workUnitId: string, example: string) => Promise<void>;
  addQuestion: (workUnitId: string, question: string) => Promise<void>;

  // UI Actions
  selectWorkUnit: (id: string | null) => void;
  setFilterStatus: (status: WorkUnitStatus | 'all') => void;
  setSearchQuery: (query: string) => void;
}
```

### View Definitions Registry

```typescript
export const viewDefinitions: Record<string, ViewDefinition> = {
  main: {
    id: 'main',
    component: MainMenuView,
    metadata: {
      title: 'fspec - ACDD Project Management',
      showDefaultFooter: true,
    },
  },

  board: {
    id: 'board',
    component: BoardView,
    metadata: {
      title: 'Kanban Board',
      subtitle: 'Acceptance Criteria Driven Development',
      footer: '←→ Columns | ↑↓ Work Units | ↵ Details | n New | s Status | ESC Back',
      showDefaultFooter: false,
    },
  },

  workUnitDetail: {
    id: 'workUnitDetail',
    component: WorkUnitDetailView,
    metadata: {
      title: 'Work Unit Details',
      footer: 'Tab Switch | e Edit | s Status | f Feature | t Tests | ESC Back',
      showDefaultFooter: false,
    },
  },

  // ... more views
};
```

---

## Implementation Strategy

### Phase 1: Scaffold (FOUNDATION-001)

**Goal**: Get basic TUI working with one view

**Steps**:
1. **Install dependencies**:
   ```bash
   npm install ink @inkjs/ui zustand immer figures cli-spinners ink-spinner react
   npm install -D @types/react ink-testing-library
   ```

2. **Copy shared components from cage**:
   - `src/tui/shared/components/VirtualList.tsx`
   - `src/tui/shared/components/FullScreenLayout.tsx`
   - `src/tui/shared/contexts/InputContext.tsx`
   - `src/tui/shared/hooks/useSafeInput.tsx`

3. **Create core navigation**:
   - `src/tui/core/navigation/ViewManager.tsx`
   - `src/tui/core/navigation/viewDefinitions.tsx`
   - `src/tui/core/navigation/types.ts`

4. **Create Zustand store**:
   - `src/tui/stores/fspecStore.ts`
   - Integrate with existing file operations (workUnits.json, epics.json, etc.)

5. **Create App.tsx**:
   - Wrap in InputModeProvider
   - Use ViewManager with main menu view
   - Handle exit (process.exit)

6. **Add TUI command**:
   ```typescript
   // src/commands/tui.ts
   export function registerTuiCommand(program: Command) {
     program
       .command('tui')
       .description('Launch interactive TUI')
       .action(async () => {
         const { render } = await import('ink');
         const React = await import('react');
         const { App } = await import('../tui/app/App');

         render(React.createElement(App));
       });
   }
   ```

7. **Add default command** (no args = TUI):
   ```typescript
   // src/index.ts
   if (process.argv.length === 2) {
     // No args, launch TUI
     await import('./commands/tui');
     registerTuiCommand(program);
     program.parse(['', '', 'tui']);
   } else {
     // Args provided, parse normally
     program.parse(process.argv);
   }
   ```

8. **Test**:
   ```bash
   npm run build
   ./dist/index.js  # Should launch TUI
   ```

**Acceptance Criteria**:
- [x] `fspec` (no args) launches TUI
- [x] Main menu displays with title
- [x] ESC exits gracefully
- [x] Zustand store loads work units from JSON
- [x] No errors or warnings

**Estimate**: 5 points (1-2 days)

### Phase 2: Add More Views (Post-Foundation)

Once foundation is solid:
- Add BoardView (BOARD-004)
- Add WorkUnitDetailView (BOARD-005)
- Add WorkUnitEditorView (WORK-001)
- etc.

Each view follows the same pattern:
1. Create view component (inherits ViewProps)
2. Register in viewDefinitions
3. Implement keyboard navigation
4. Connect to Zustand store
5. Test with ink-testing-library

---

## Testing Strategy

### Unit Testing TUI Components

**Use ink-testing-library**:
```typescript
import { render } from 'ink-testing-library';
import React from 'react';
import { MainMenuView } from './MainMenuView';

describe('MainMenuView', () => {
  it('should display menu options', () => {
    const { lastFrame } = render(<MainMenuView onNavigate={jest.fn()} />);

    expect(lastFrame()).toContain('Kanban Board');
    expect(lastFrame()).toContain('Work Units');
    expect(lastFrame()).toContain('Features');
  });

  it('should navigate on selection', () => {
    const onNavigate = jest.fn();
    const { stdin } = render(<MainMenuView onNavigate={onNavigate} />);

    stdin.write('\r'); // Press Enter

    expect(onNavigate).toHaveBeenCalledWith('board');
  });
});
```

### Integration Testing

**Test full workflows**:
```typescript
describe('Kanban workflow', () => {
  it('should navigate from board to work unit detail', () => {
    const { lastFrame, stdin } = render(<App />);

    // Navigate to board
    stdin.write('j'); // Move down to "Kanban Board"
    stdin.write('\r'); // Select

    expect(lastFrame()).toContain('Kanban Board');

    // Select work unit
    stdin.write('\r');

    expect(lastFrame()).toContain('Work Unit Details');
  });
});
```

---

## File Watching & Real-Time Updates

**Option 1: Polling** (Simple)
```typescript
useEffect(() => {
  const interval = setInterval(() => {
    useAppStore.getState().loadWorkUnits();
  }, 2000);

  return () => clearInterval(interval);
}, []);
```

**Option 2: File Watching** (Better)
```typescript
import chokidar from 'chokidar';

class FileWatcherService {
  static instance: FileWatcherService;
  watcher: chokidar.FSWatcher | null = null;

  start() {
    this.watcher = chokidar.watch('spec/*.json', {
      persistent: true,
    });

    this.watcher.on('change', (path) => {
      if (path.includes('work-units.json')) {
        useAppStore.getState().loadWorkUnits();
      } else if (path.includes('epics.json')) {
        useAppStore.getState().loadEpics();
      }
    });
  }

  stop() {
    this.watcher?.close();
  }
}
```

---

## Common Pitfalls & Solutions

### Pitfall 1: Input conflicts between components

**Problem**: Multiple components listen to keyboard, causing duplicate actions.

**Solution**: Use InputContext and useSafeInput with proper modes.

### Pitfall 2: Re-rendering entire list on scroll

**Problem**: Scrolling is laggy with large lists.

**Solution**: Use VirtualList (only renders visible items).

### Pitfall 3: Stale state in async actions

**Problem**: Zustand state updates before async operation completes.

**Solution**: Use `get()` inside async functions:
```typescript
updateWorkUnit: async (id, updates) => {
  await api.updateWorkUnit(id, updates);
  const currentUnits = get().workUnits;
  set({ workUnits: currentUnits.map(u => u.id === id ? {...u, ...updates} : u) });
}
```

### Pitfall 4: Layout breaking on terminal resize

**Problem**: Fixed-height components don't resize.

**Solution**: Use `useStdoutDimensions()` hook from `ink-use-stdout-dimensions`.

---

## Performance Considerations

### 1. **Virtual Scrolling**
- Use VirtualList for any list > 50 items
- Only renders visible rows
- Handles 1000+ items smoothly

### 2. **Memoization**
- Memoize expensive computations with `useMemo`
- Memoize callbacks with `useCallback`
- Prevent unnecessary re-renders

### 3. **Selective Store Updates**
- Use granular selectors:
  ```typescript
  // Bad (re-renders on any store change)
  const state = useAppStore();

  // Good (only re-renders when workUnits change)
  const workUnits = useAppStore(state => state.workUnits);
  ```

### 4. **Debouncing**
- Debounce search input
- Debounce file save operations
- Use lodash.debounce or custom hook

---

## Next Steps

1. **Implement FOUNDATION-001**:
   - Install dependencies
   - Copy shared components
   - Create basic App.tsx
   - Add TUI command
   - Test basic navigation

2. **Create BOARD-004** (Interactive Kanban):
   - Design KanbanBoard component
   - Implement column navigation
   - Add WorkUnitCard component
   - Test keyboard shortcuts

3. **Iterate**:
   - Add views incrementally
   - Test continuously
   - Gather user feedback
   - Refine UX

---

## References

- **Cage Repository**: https://github.com/sengac/cage
- **Ink Documentation**: https://github.com/vadimdemedes/ink
- **Zustand Documentation**: https://github.com/pmndrs/zustand
- **ink-testing-library**: https://github.com/vadimdemedes/ink-testing-library
- **fspec Repository**: https://github.com/sengac/fspec

---

**Document Version**: 1.0
**Last Updated**: 2025-10-27
**Author**: AI Analysis (Claude Code)
