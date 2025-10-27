# Interactive Kanban Board - Design Specification

**Work Unit**: BOARD-002
**Epic**: Interactive Kanban Board
**Created**: 2025-10-27
**Purpose**: Detailed design specification for the interactive Kanban board view, including UI/UX patterns, keyboard navigation, and component architecture

---

## Vision

Transform the static ASCII `fspec board` output into a fully interactive, keyboard-navigable Kanban board that allows users to:
- Navigate between columns and work units with arrow keys
- View work unit details inline
- Update status with keyboard shortcuts
- Filter and search work units in real-time
- Perform bulk operations
- See real-time updates from file changes

---

## Current State vs Desired State

### Current (`fspec board`)

```
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│BACKLOG   │SPECIFYING│TESTING   │IMPLEMENTI│VALIDATING│DONE      │BLOCKED   │
├──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┤
│RES-003   │GIT-004   │          │          │          │BUG-007 [3│          │
│RES-004   │RES-006   │          │          │          │BUG-008 [2│          │
│RES-005   │EST-002   │          │          │          │EXMAP-001 │          │
│RES-001   │LANG-001  │          │          │          │INIT-001  │          │
│... 22 mor│          │          │          │          │... 185 mo│          │
├────────────────────────────────────────────────────────────────────────────┤
│0 points in progress, 607 points completed                                  │
└────────────────────────────────────────────────────────────────────────────┘
```

**Problems**:
- Static output (no interaction)
- Can't see work unit details
- Can't update status directly
- No filtering or search
- Truncated lists ("... 22 more")
- No visual distinction between types (story/bug/task)

### Desired State (Interactive TUI)

```
┌─ fspec Kanban Board ────────────────────────────────────────────────────────┐
│                                                                              │
│  BACKLOG (26)    SPECIFYING (4)  TESTING (0)    IMPLEMENTING (0)            │
│  ┌──────────┐   ┌──────────┐    ┌──────────┐   ┌──────────┐                │
│  │📖 RES-003│   │📖 GIT-004│    │          │   │          │                │
│❯ │5pt 🟡    │   │8pt 🔴    │    │          │   │          │                │
│  │Perplexity│   │Interactive│   │          │   │          │                │
│  │research  │   │checkpoint│   │          │   │          │                │
│  └──────────┘   └──────────┘    └──────────┘   └──────────┘                │
│  ┌──────────┐   ┌──────────┐                                                │
│  │📖 RES-004│   │📖 RES-006│                                                │
│  │3pt 🟢    │   │5pt 🟡    │                                                │
│  │JIRA tool │   │Stakeholder│                                               │
│  │          │   │comm      │                                                │
│  └──────────┘   └──────────┘                                                │
│  ... 22 more (scroll ↓)                                                     │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ ← → Columns | ↑↓ jk Work Units | ↵ Details | / Search | f Filter | n New   │
│ s Status | e Edit | d Delete | ESC Back              0pts in progress       │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Improvements**:
- ✓ Interactive navigation
- ✓ Work unit cards show: type icon, estimate, priority, truncated title
- ✓ Selected item highlighted
- ✓ Keyboard shortcuts visible in footer
- ✓ Real-time point counts
- ✓ Scrollable columns (virtual scrolling)

---

## UI Component Architecture

### Component Hierarchy

```
BoardView
├── FilterBar (search, status, epic, tags)
├── KanbanBoard
│   ├── KanbanColumn (×7: backlog → done + blocked)
│   │   ├── ColumnHeader (title, count, points)
│   │   └── VirtualList<WorkUnit>
│   │       └── WorkUnitCard (×N visible items)
│   │           ├── TypeIcon (📖/🐛/⚙️)
│   │           ├── WorkUnitId
│   │           ├── EstimateBadge
│   │           ├── PriorityIndicator
│   │           └── TruncatedTitle
└── Footer (keyboard shortcuts, summary)
```

### Component Specifications

#### 1. BoardView

**Responsibilities**:
- Manage column focus (which column is active)
- Handle global keyboard shortcuts
- Orchestrate data loading from Zustand store
- Handle navigation to detail view

**State**:
```typescript
interface BoardViewState {
  focusedColumnIndex: number; // 0-6 (backlog → blocked)
  selectedWorkUnitId: string | null;
  searchQuery: string;
  filterStatus: WorkUnitStatus | 'all';
  filterEpic: string | null;
  filterTags: string[];
}
```

**Keyboard Shortcuts**:
- `←` / `h` - Previous column
- `→` / `l` - Next column
- `↑` / `k` - Previous work unit in column
- `↓` / `j` - Next work unit in column
- `Enter` - View work unit details
- `/` - Focus search
- `f` - Open filter menu
- `n` - New work unit
- `s` - Update status (opens status wizard)
- `e` - Edit work unit
- `d` - Delete work unit (with confirmation)
- `ESC` - Go back / clear filters

#### 2. KanbanColumn

**Props**:
```typescript
interface KanbanColumnProps {
  status: WorkUnitStatus;
  workUnits: WorkUnit[];
  isFocused: boolean;
  selectedId: string | null;
  onSelectWorkUnit: (id: string) => void;
  onWorkUnitAction: (action: string, id: string) => void;
}
```

**Rendering**:
```
┌─ BACKLOG (26) ─ 78pts ─┐
│                         │
│  [WorkUnitCard]         │
│  [WorkUnitCard]         │
│  [WorkUnitCard]         │  ← VirtualList (only visible)
│  ...                    │
│  ... 20 more (scroll ↓) │
│                         │
└─────────────────────────┘
```

**Visual States**:
- **Focused**: Border color = cyan (active column)
- **Unfocused**: Border color = gray (inactive column)
- **Empty**: Show "No work units" message

#### 3. WorkUnitCard

**Compact Layout** (1 card = 4 rows):
```
┌──────────────────────────┐
│ 📖 RES-003      5pt 🟡  │ ← Row 1: Icon, ID, Estimate, Priority
│ Perplexity research      │ ← Row 2: Title (truncated)
│ integration tool         │ ← Row 3: Title cont. (optional)
│ @research-tools          │ ← Row 4: Epic tag (optional)
└──────────────────────────┘
```

**Visual Elements**:
- **Type Icons**:
  - 📖 Story
  - 🐛 Bug
  - ⚙️ Task
- **Priority Indicators**:
  - 🔴 High (if estimate > 8 or tagged @critical)
  - 🟡 Medium (estimate 3-8)
  - 🟢 Low (estimate 1-2)
- **Estimate Badge**: `5pt`, `13pt`, `--` (no estimate)
- **Epic Tag**: `@epic-name` (dimmed, optional)

**States**:
- **Selected**: Background = cyan, bold text
- **Unselected**: Background = default, normal text
- **Blocked**: Red border + 🚫 icon

**Props**:
```typescript
interface WorkUnitCardProps {
  workUnit: WorkUnit;
  isSelected: boolean;
  compact?: boolean; // If false, show detailed view
}
```

#### 4. FilterBar

**Layout**:
```
┌─ Filters ──────────────────────────────────────────────────────────────┐
│ Search: [perplexity___________]  Status: [all ▼]  Epic: [research ▼]  │
│ Tags: [@high @cli @discovery] ×                                        │
└────────────────────────────────────────────────────────────────────────┘
```

**Features**:
- **Search**: Fuzzy search in title, description, Example Mapping
- **Status dropdown**: all, backlog, specifying, testing, implementing, validating, done, blocked
- **Epic dropdown**: List of all epics
- **Tag picker**: Multi-select with × to clear
- **Active filters badge**: Shows count of active filters

**Keyboard**:
- `/` - Focus search input (enter insert mode)
- `Tab` - Move between filter fields
- `ESC` - Exit filter (return to normal mode)
- `Ctrl+C` - Clear all filters

#### 5. ColumnHeader

**Layout**:
```
┌─ BACKLOG (26) ─ 78pts ─┐
```

**Display**:
- Status name (uppercase)
- Work unit count
- Total story points in column

**Visual**:
- Focused: Bold + cyan color
- Unfocused: Dim

---

## Keyboard Navigation Flow

### Column Navigation

```
User presses →
  ├─ focusedColumnIndex++
  ├─ If at end (blocked), wrap to start (backlog)
  ├─ selectedWorkUnitId = first work unit in new column
  └─ Re-render with new focus
```

### Work Unit Navigation (within column)

```
User presses ↓
  ├─ Get current column's work units
  ├─ Find index of selectedWorkUnitId
  ├─ Increment index (or wrap if enableWrapAround)
  ├─ Update selectedWorkUnitId to next work unit
  └─ VirtualList auto-scrolls to keep selected visible
```

### Selection (Enter key)

```
User presses Enter
  ├─ Get selectedWorkUnitId
  ├─ navigate('workUnitDetail', { id: selectedWorkUnitId })
  └─ ViewManager pushes to history
```

---

## Data Flow

### Loading Work Units

```
App.tsx (useEffect)
  ↓
fspecStore.loadWorkUnits()
  ↓
Read spec/work-units.json
  ↓
Parse JSON → WorkUnit[]
  ↓
set({ workUnits })
  ↓
BoardView re-renders with new data
```

### Updating Status

```
User presses 's'
  ↓
BoardView calls fspecStore.updateWorkUnitStatus(id, newStatus)
  ↓
Store calls underlying CLI logic (updateWorkUnitStatus from commands/)
  ↓
Update spec/work-units.json
  ↓
set({ workUnits: updated })
  ↓
BoardView re-renders
```

### Real-Time Updates (File Watcher)

```
FileWatcherService.start()
  ↓
chokidar watches spec/work-units.json
  ↓
File changes detected
  ↓
Trigger fspecStore.loadWorkUnits()
  ↓
Components re-render with fresh data
```

---

## Filtering & Search

### Search Algorithm

**Fuzzy match** on:
- Work unit title
- Work unit description
- Example Mapping rules
- Example Mapping examples
- Example Mapping questions
- Epic name
- Tags

**Implementation**:
```typescript
function matchesSearch(workUnit: WorkUnit, query: string): boolean {
  const lowerQuery = query.toLowerCase();

  return (
    workUnit.title.toLowerCase().includes(lowerQuery) ||
    workUnit.description?.toLowerCase().includes(lowerQuery) ||
    workUnit.rules?.some(r => r.toLowerCase().includes(lowerQuery)) ||
    workUnit.examples?.some(e => e.toLowerCase().includes(lowerQuery)) ||
    workUnit.epic?.toLowerCase().includes(lowerQuery) ||
    workUnit.tags?.some(t => t.toLowerCase().includes(lowerQuery))
  );
}
```

### Filter Combinations

**Filters are AND-ed**:
- Search: "research" AND
- Status: "backlog" AND
- Epic: "research-tools" AND
- Tags: ["@high", "@cli"]

**Result**: Only work units matching ALL filters

---

## Performance Optimizations

### 1. Virtual Scrolling (VirtualList)

**Problem**: Rendering 1000+ work units at once is slow.

**Solution**: Only render visible items (e.g., 10-20 cards).

**Implementation**:
```typescript
<VirtualList
  items={filteredWorkUnits}
  height={20} // 20 visible cards
  renderItem={(wu, index, isSelected) => (
    <WorkUnitCard workUnit={wu} isSelected={isSelected} />
  )}
/>
```

### 2. Memoization

**Memoize filtered work units**:
```typescript
const filteredWorkUnits = useMemo(() => {
  return workUnits
    .filter(wu => matchesSearch(wu, searchQuery))
    .filter(wu => filterStatus === 'all' || wu.status === filterStatus)
    .filter(wu => !filterEpic || wu.epic === filterEpic);
}, [workUnits, searchQuery, filterStatus, filterEpic]);
```

### 3. Selective Re-renders

**Use Zustand selectors**:
```typescript
// Bad (re-renders on ANY store change)
const state = useAppStore();

// Good (only re-renders when workUnits change)
const workUnits = useAppStore(state => state.workUnits);
```

---

## Accessibility

### Screen Reader Support

**Ink doesn't support screen readers**, but we can:
- Use semantic labels in text
- Provide descriptive feedback
- Support keyboard-only navigation

### Color Blindness

**Don't rely on color alone**:
- Priority: Use icons + color (🔴 🟡 🟢)
- Status: Use text + color
- Selection: Use background + border

### Keyboard-Only Navigation

**All actions accessible via keyboard**:
- No mouse required
- Vim-style shortcuts (hjkl)
- Standard arrow keys
- Enter/Escape for common actions

---

## Edge Cases

### Empty Columns

**Display**:
```
┌─ TESTING (0) ─ 0pts ─┐
│                       │
│   No work units       │
│                       │
└───────────────────────┘
```

**Navigation**: Skip empty columns when using ←→ keys.

### Single Column Navigation

**Behavior**: If only one column has work units, ←→ does nothing (or wraps).

### Long Titles

**Truncation**:
```
┌──────────────────────────┐
│ 📖 RES-003      5pt 🟡  │
│ Interactive research co… │ ← Truncated with "…"
└──────────────────────────┘
```

**Full title in detail view** (on Enter).

### Large Estimates

**Display**:
```
13pt  → Yellow (acceptable upper limit)
21pt  → Red + ⚠️ (too large, needs breakdown)
```

**Tooltip/hint**: "Story too large - consider breaking down"

---

## Testing Strategy

### Unit Tests (ink-testing-library)

**Test KanbanColumn rendering**:
```typescript
it('should render work unit cards', () => {
  const { lastFrame } = render(
    <KanbanColumn
      status="backlog"
      workUnits={mockWorkUnits}
      isFocused={true}
      selectedId={null}
      onSelectWorkUnit={jest.fn()}
      onWorkUnitAction={jest.fn()}
    />
  );

  expect(lastFrame()).toContain('BACKLOG');
  expect(lastFrame()).toContain('RES-003');
  expect(lastFrame()).toContain('5pt');
});
```

**Test keyboard navigation**:
```typescript
it('should navigate to next column on → key', () => {
  const { stdin, lastFrame } = render(<BoardView />);

  stdin.write('\x1B[C'); // Right arrow

  expect(lastFrame()).toContain('SPECIFYING'); // Column title highlighted
});
```

### Integration Tests

**Test end-to-end workflow**:
```typescript
it('should update work unit status', async () => {
  const { stdin, lastFrame } = render(<BoardView />);

  // Select work unit
  stdin.write('j'); // Move down
  stdin.write('\r'); // Enter (view details)

  // Update status
  stdin.write('s'); // Status wizard
  stdin.write('\r'); // Confirm transition

  await waitFor(() => {
    expect(lastFrame()).toContain('specifying'); // New status
  });
});
```

---

## Implementation Checklist

### Phase 1: Basic Board (BOARD-004)

- [ ] Create BoardView.tsx
- [ ] Create KanbanBoard.tsx
- [ ] Create KanbanColumn.tsx
- [ ] Create WorkUnitCard.tsx
- [ ] Implement column navigation (←→)
- [ ] Implement work unit navigation (↑↓)
- [ ] Implement selection (Enter → detail view)
- [ ] Add footer with keyboard shortcuts
- [ ] Test with ink-testing-library

**Estimate**: 8 points

### Phase 2: Filtering & Search (BOARD-004 cont.)

- [ ] Create FilterBar.tsx
- [ ] Implement search (fuzzy match)
- [ ] Implement status filter dropdown
- [ ] Implement epic filter dropdown
- [ ] Implement tag picker
- [ ] Add filter clear button
- [ ] Test filter combinations

**Estimate**: 5 points

### Phase 3: Actions (BOARD-006)

- [ ] Status update wizard (s key)
- [ ] Edit work unit (e key)
- [ ] Delete work unit (d key with confirmation)
- [ ] New work unit (n key)
- [ ] Bulk operations (multi-select with Space)

**Estimate**: 8 points

### Phase 4: Real-Time Updates (BOARD-003)

- [ ] Add FileWatcherService
- [ ] Watch spec/work-units.json
- [ ] Auto-reload on file changes
- [ ] Show git stash indicator
- [ ] Show changed files list
- [ ] Inspect file diffs (d key)

**Estimate**: 8 points

---

## Future Enhancements

### Drag & Drop (ASCII)

**Visual feedback** for moving work units:
```
┌─ BACKLOG ─┐   ┌─ SPECIFYING ─┐
│            │   │               │
│ RES-003 ───┼───►              │  ← Dragging indicator
│            │   │               │
└────────────┘   └───────────────┘
```

**Implementation**: Multi-step wizard:
1. Press 'm' on work unit
2. Navigate to target column with ←→
3. Press Enter to move

### Swimlanes

**Group by epic**:
```
┌─ Epic: Research Tools ─────────────────────────┐
│ BACKLOG    SPECIFYING  TESTING  IMPLEMENTING   │
│ RES-003    RES-006                             │
│ RES-004                                        │
└────────────────────────────────────────────────┘
┌─ Epic: Interactive CLI ────────────────────────┐
│ BACKLOG    SPECIFYING  TESTING  IMPLEMENTING   │
│ BOARD-002  BOARD-003                           │
└────────────────────────────────────────────────┘
```

### Velocity Chart

**Show points completed per week** (ASCII art):
```
┌─ Velocity (last 4 weeks) ─┐
│                            │
│ 50│       ██               │
│ 40│    ██ ██               │
│ 30│ ██ ██ ██ ██           │
│ 20│ ██ ██ ██ ██           │
│ 10│ ██ ██ ██ ██           │
│  0├────────────────────    │
│    W1  W2  W3  W4          │
└────────────────────────────┘
```

---

## References

- **Cage BoardView**: N/A (cage doesn't have Kanban board, but has event inspector list)
- **Ink Documentation**: https://github.com/vadimdemedes/ink
- **VirtualList Pattern**: See `tui-foundation-architecture.md`
- **fspec board command**: `src/commands/board.ts`

---

**Document Version**: 1.0
**Last Updated**: 2025-10-27
**Author**: AI Analysis (Claude Code)
