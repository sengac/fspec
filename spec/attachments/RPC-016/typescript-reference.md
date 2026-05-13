# RPC-016 — Per-column scroll viewport + indicators + keyboard nav

## TypeScript reference

### Per-column scroll state
`src/tui/components/UnifiedBoardLayout.tsx:149-158`

```tsx
const [scrollOffsets, setScrollOffsets] = useState<Record<string, number>>({
  backlog: 0, specifying: 0, testing: 0, implementing: 0,
  validating: 0, done: 0, blocked: 0,
});
```

Each of the seven columns owns its own scroll offset. The DONE column will
typically be heavily scrolled while BACKLOG stays at offset 0.

### Auto-scroll-to-selected
`src/tui/components/UnifiedBoardLayout.tsx:185-234`

When `selectedWorkUnitIndex` moves outside the visible viewport in the
focused column, the scroll offset is adjusted so the selected item is
near the top (if scrolling up) or near the bottom (if scrolling down),
accounting for the up/down arrow rows that consume one viewport row each.

### Viewport height
`src/tui/components/UnifiedBoardLayout.tsx:95-103`

```ts
const calculateViewportHeight = (terminalHeight: number): number => {
  // Fixed rows: 17 (border + 4 header + separator + 5 details +
  // separator + col-header + separator + footer separator + footer + bottom border)
  const fixedRows = 17;
  return Math.max(5, terminalHeight - fixedRows);
};
```

### Up/down arrow rendering
`src/tui/components/UnifiedBoardLayout.tsx:452-458`

```tsx
if (rowIndex === 0 && scrollOffset > 0 && column.units.length > 0) {
  return fitToWidth('↑', currentColWidth);
}
if (rowIndex === VIEWPORT_HEIGHT - 1 && scrollOffset + VIEWPORT_HEIGHT < column.units.length) {
  return fitToWidth('↓', currentColWidth);
}
```

When a column is scrolled, the first row of the column displays `↑`; when
there are more items below the viewport, the last row displays `↓`.

### Last-changed indicator (`⏩`)
`src/tui/components/UnifiedBoardLayout.tsx:169-183, 468-475`

```tsx
const lastChangedWorkUnit = useMemo(() => {
  return workUnits.reduce((latest, current) => {
    const latestStateTimestamp = latest.stateHistory?.length
      ? new Date(latest.stateHistory[latest.stateHistory.length - 1].timestamp).getTime()
      : 0;
    const currentStateTimestamp = current.stateHistory?.length
      ? new Date(current.stateHistory[current.stateHistory.length - 1].timestamp).getTime()
      : 0;
    return currentStateTimestamp > latestStateTimestamp ? current : latest;
  });
}, [workUnits]);

// ...
const isLastChanged = lastChangedWorkUnit?.id === wu.id;
const text = isLastChanged
  ? `⏩ ${sessionIndicator}${wu.id}${storyPointsText} ⏩`
  : `${sessionIndicator}${wu.id}${storyPointsText}`;
```

### Session-attached indicator (`🟢`)
`src/tui/components/UnifiedBoardLayout.tsx:131-132, 470-475`

```tsx
const hasAttachedSession = useFspecStore(state => state.hasAttachedSession);
// ...
const hasSession = hasAttachedSession(wu.id);
const sessionIndicator = hasSession ? '🟢 ' : '';
```

### Keyboard navigation
`src/tui/components/UnifiedBoardLayout.tsx:287-352`

- PageUp/PageDown: scroll by VIEWPORT_HEIGHT (line 301-309).
- Home/End: jump to start/end of focused column's units (line 251-279,
  parsed from raw stdin since Ink filters these keys).
- Arrow up/down: move selection by 1 (line 328-331).
- Arrow left/right: change focused column (line 324-327).

## Current Rust state

`codelet/fspec-tui/src/views/board.rs:149-194` renders every unit in the
column, no viewport math, no arrows.

`BoardStore::last_changed_id` and `BoardStore::session_for(id)` already
exist (`codelet/fspec-tui/src/store/board.rs:181-198`); only the render
is missing.

Keyboard navigation handles arrow keys and h/j/k/l + `[`/`]` already
(`codelet/fspec-tui/src/views/board.rs:78-104`) but NOT PageUp / PageDown
/ Home / End.

## Target Rust behavior

### New BoardStore fields

```rust
pub struct BoardStore {
    // existing fields...
    scroll_offsets: HashMap<String, usize>,
}

impl BoardStore {
    pub fn scroll_offset_for(&self, column: &str) -> usize { ... }
    pub fn set_scroll_offset_for(&mut self, column: &str, offset: usize) { ... }
    /// Move the focused column's selection by `delta` and auto-scroll the
    /// viewport so the selection stays visible. `viewport_height` is the
    /// number of rows available for the column content (computed from
    /// terminal_height - fixed_rows).
    pub fn move_selection(&mut self, delta: isize, viewport_height: usize) { ... }
    /// Scroll the focused column by `delta` rows. Used by PageUp / PageDown.
    pub fn scroll_focused_column(&mut self, delta: isize, viewport_height: usize) { ... }
    /// Jump the focused column's selection to its first / last unit.
    pub fn select_first_in_focused(&mut self) { ... }
    pub fn select_last_in_focused(&mut self) { ... }
}
```

### New BoardView render logic

In `codelet/fspec-tui/src/views/board.rs::render_column`:

1. Compute `viewport_height = area.height - 2` (account for column header + separator).
2. Read `scroll_offset = store.scroll_offset_for(column)`.
3. For `row_index in 0..viewport_height`:
   - If `row_index == 0 && scroll_offset > 0 && units.len() > 0`: render `↑` centered.
   - Else if `row_index == viewport_height - 1 && scroll_offset + viewport_height < units.len()`: render `↓` centered.
   - Else: compute `item_index = scroll_offset + row_index`; render the unit at that index (or blank if out of range).

### Indicators

In the cell render path:
- Compute `is_last_changed = store.last_changed() == Some(unit.id.as_str())`.
- Compute `has_session = store.session_for(&unit.id).is_some()`.
- Build the cell text:
  ```rust
  let session_indicator = if has_session { "🟢 " } else { "" };
  let points = unit.estimate.map(|p| format!(" [{p}]")).unwrap_or_default();
  let text = if is_last_changed {
      format!("⏩ {session_indicator}{}{points} ⏩", unit.id)
  } else {
      format!("{session_indicator}{}{points}", unit.id)
  };
  ```

### Last-changed derivation

The TS code derives `lastChangedWorkUnit` from `stateHistory[last].timestamp`.
The Rust `BoardStore` currently has `last_changed_id` as a manually-set
`Option<String>`. **Decision needed in specifying phase:**

- (a) Pass `state_history: Vec<StateHistoryEntry>` over the RPC and derive
  `last_changed_id` inside `replace_work_units` — matches TS exactly but
  bloats the work-units payload.
- (b) Add a `last_state_change_at: Option<DateTime<Utc>>` field to
  `WorkUnitInfo` carrying only the latest timestamp — minimal payload,
  same render outcome.

Option (b) is preferred. Requires adding the field to
`codelet/rpc-types/src/lib.rs::WorkUnitInfo` (gated on the `napi` feature)
and wiring `codelet_core::work_units::WorkUnit` to populate it.

### Keyboard nav

Extend `codelet/fspec-tui/src/views/board.rs::handle_event`:
- `KeyCode::PageUp` → `Action::ScrollFocusedColumnUp(viewport_height)`
- `KeyCode::PageDown` → `Action::ScrollFocusedColumnDown(viewport_height)`
- `KeyCode::Home` → `Action::SelectFirstInFocused`
- `KeyCode::End` → `Action::SelectLastInFocused`

(`viewport_height` is computed at render time and stashed in the view; the
keyboard handler reads it from a `last_viewport_height: Cell<u16>` field.)

### Auto-scroll integration

`App::dispatch` for `Action::SelectNext` / `Action::SelectPrev` now calls
`board_store.move_selection(±1, viewport_height)` instead of the raw
`set_selected_index_for`. The `move_selection` method clamps to column
length AND adjusts `scroll_offsets[focused_column]` so the new selection
stays visible (mirroring the TS auto-scroll algorithm at line 185-234).

## RPC/NAPI boundary

### Option (b) — `WorkUnitInfo` extension

```rust
// codelet/rpc-types/src/lib.rs
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
    // RPC-016: timestamp of the latest state-history entry, for
    // `lastChangedWorkUnit` derivation in the board view. None means
    // the work unit has never transitioned.
    pub last_state_change_at: Option<String>, // ISO-8601 UTC
}
```

The TS code reads `wu.stateHistory[last].timestamp` today; after this card
it should ALSO read `wu.lastStateChangeAt` if present (with the
`stateHistory` derivation as fallback so the TS BoardView keeps working
unchanged with older payloads).

### No new RPC methods

The session-attached indicator reuses the existing `session_attachments`
map already wired in RPC-012. The last-changed indicator reuses the
extended `WorkUnitInfo` payload.

## Existing TypeScript behavior preserved

- `src/tui/components/UnifiedBoardLayout.tsx` — UNCHANGED. Its existing
  `stateHistory`-based derivation still works because `WorkUnitInfo` is
  additive.
- `src/tui/store/fspecStore.ts::hasAttachedSession` — UNCHANGED.

## Acceptance criteria sketch

- Each column displays AT MOST `viewport_height` rows of work units.
- `↑` appears centered at row 0 of a column when `scroll_offset > 0`.
- `↓` appears centered at the last viewport row when there are more units
  below the viewport.
- Moving the selection past the top/bottom of the visible viewport
  auto-scrolls the column so the selection remains visible.
- PageUp/PageDown scroll the focused column by `viewport_height` rows.
- Home/End jump the focused column's selection to the first/last unit.
- The most-recently-changed work unit (derived from
  `last_state_change_at`) displays `⏩ <id> ⏩` in every column it appears.
- Work units with an attached session (`store.session_for(id).is_some()`)
  display the `🟢 ` prefix.
- `codelet/rpc-types/src/lib.rs::WorkUnitInfo` gains `last_state_change_at: Option<String>`.
- All existing RPC-009/011/012 tests pass; new tests cover auto-scroll
  math, PageUp/Down, Home/End, and arrow rendering.
