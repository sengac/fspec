# RPC-014 — Rich box-drawing Kanban grid + work-unit details strip

## TypeScript reference

### File: `src/tui/components/UnifiedBoardLayout.tsx`

Key structural elements (lines 354-516):

1. **Top border** (line 357): `'┌' + '─'.repeat(totalWidth) + '┐'`
2. **Header row** (lines 360-380): 4-row tall — owned by RPC-015, not this card.
3. **Header→Details separator** (line 383): plain `├ ─ ┤`.
4. **Work-unit details strip** (lines 386-425): 5 rows tall, contains:
   - `WorkUnitTitle` — `src/tui/components/WorkUnitTitle.tsx`
   - `WorkUnitDescription` — `src/tui/components/WorkUnitDescription.tsx`
   - `WorkUnitAttachments` — `src/tui/components/WorkUnitAttachments.tsx`
   - `WorkUnitMetadata` — `src/tui/components/WorkUnitMetadata.tsx`
5. **Details→Columns separator** (line 428): top junctions `├ ┬ ┤`.
6. **Column header row** (lines 431-439): uppercase status names per column.
7. **Column header separator** (line 442): cross junctions `├ ┼ ┤`.
8. **Column content rows** (lines 445-499): VIEWPORT_HEIGHT rows, each row is `│ <cell> │ <cell> │ ... │`.
9. **Footer separator** (line 502): bottom junctions `├ ┴ ┤`.
10. **Footer** (lines 504-511): owned by RPC-013.
11. **Bottom border** (line 514): `'└' + '─'.repeat(totalWidth) + '┘'`.

### Column width math (lines 64-76)

```ts
const calculateColumnWidths = (terminalWidth: number) => {
  const borders = 2;
  const separators = STATES.length - 1;
  const availableWidth = terminalWidth - borders - separators;
  const baseWidth = Math.floor(availableWidth / STATES.length);
  const remainder = availableWidth % STATES.length;
  return { baseWidth: Math.max(8, baseWidth), remainder: baseWidth >= 8 ? remainder : 0 };
};
```

The first `remainder` columns get `baseWidth + 1`, the rest get `baseWidth`.
Column widths are computed once and shared by the separator builder and the
content rows so the box-drawing characters always line up.

### Details strip content (lines 386-425)

```tsx
{selectedWorkUnit ? (
  <>
    <WorkUnitTitle id={selectedWorkUnit.id} title={selectedWorkUnit.title} />
    <WorkUnitDescription description={selectedWorkUnit.description || ''} width={terminalWidth} />
    <WorkUnitAttachments attachments={selectedWorkUnit.attachments} width={terminalWidth} />
    <WorkUnitMetadata epic={selectedWorkUnit.epic} estimate={selectedWorkUnit.estimate} status={selectedWorkUnit.status} />
  </>
) : (
  <Box flexGrow={1} justifyContent="center" alignItems="center">
    <Text>No work unit selected</Text>
  </Box>
)}
```

`WorkUnitTitle` (`src/tui/components/WorkUnitTitle.tsx`) shows:
```
AGENT-002: Test Cline agent compatibility
```
in cyan/bold.

`WorkUnitDescription` (`src/tui/components/WorkUnitDescription.tsx`) shows the
first line of the description, wrapped to terminal width.

`WorkUnitAttachments` (`src/tui/components/WorkUnitAttachments.tsx`) shows:
```
Attachments (use the "A" key to view): ast-research-napi-session.json
```

`WorkUnitMetadata` (`src/tui/components/WorkUnitMetadata.tsx`) shows:
```
Epic: agent-testing | Status: backlog
```

## Current Rust state

`codelet/fspec-tui/src/views/board.rs:115-147` renders a single outer
`Block::default().borders(Borders::ALL)` and seven `Paragraph`s for the
columns. No box-drawing junctions between columns, no work-unit details
strip, no per-column width math.

The `BoardStore` already exposes `selected_work_unit() -> Option<&WorkUnitInfo>`
(`codelet/fspec-tui/src/store/board.rs:149-155`) — the data path for the
details strip is wired; only the render is missing.

## Target Rust behavior

### New files (all under 300 LoC)

1. `codelet/fspec-tui/src/views/board/grid.rs` — pure-function helpers:
   - `calculate_column_widths(terminal_width: u16) -> ColumnWidths`
   - `build_border_row(widths, left, mid, right, separator: SeparatorType) -> String`
   - `column_width_at(idx, widths) -> u16`
2. `codelet/fspec-tui/src/views/board/details_strip.rs` — renders the 5-row
   work-unit details into a given `Rect` using the store's `selected_work_unit`.
3. `codelet/fspec-tui/src/views/board.rs` — orchestrator, refactored to
   compose grid + details_strip + column content rows.

### Rendering contract

When `BoardStore::selected_work_unit()` returns `Some(unit)`:
- Row 1: `{unit.id}: {unit.title}` (cyan, bold)
- Row 2: first line of `unit.description` truncated to `terminal_width - 2`
- Row 3 (if any attachments): `Attachments (use the "A" key to view): <comma-joined>`
- Row 4: `Epic: {unit.epic} | Status: {unit.status}`
- Row 5: blank (padding) OR the `[estimate]` value if non-None

When `selected_work_unit()` returns `None`: centered `No work unit selected`.

### Column header row

Each column header `{column.uppercase()}` padded to the column's width.
Focused column is cyan + bold (matching the TS `chalk.cyan` call); others are
gray (matching `chalk.gray`).

### Cell rendering

Format: `{id}` or `{id} [{points}]` if `estimate` is `Some`. Selected cell is
`bg=green fg=black bold`. Bugs are red; tasks are blue; everything else is
default fg. **`⏩` and `🟢` indicators land in RPC-016, not this card.**

### Terminal-width responsiveness

`Compositor::render` already receives the framebuffer area. The grid module
re-computes column widths every frame from `area.width`. **Important:** when
`area.width < 64` (8 columns × min 8 width minus padding), fall back to a
single-column "narrow mode" that just lists the focused column — matching the
TS `Math.max(8, baseWidth)` floor.

## RPC/NAPI boundary

**No new RPC methods required for this card.** The `WorkUnitInfo` payload
already carries `id, title, work_type, status, description, estimate, epic`
(`codelet/rpc-types/src/lib.rs:35-46`) which covers everything in the details
strip EXCEPT `attachments`.

### Attachment data (open question for specifying phase)

The TS code reads `selectedWorkUnit.attachments: string[]` from the work-unit
struct returned by `loadWorkUnits` (`src/tui/store/fspecStore.ts`). The
underlying NAPI call is `getWorkUnits()` in `codelet/napi/src/work_units_watcher.rs`
which currently returns `WorkUnitInfo` WITHOUT an `attachments` field.

**Decision needed:** Either
- (a) extend `WorkUnitInfo` in `codelet/rpc-types/src/lib.rs` to add
  `attachments: Vec<String>` (gated on the `napi` feature to keep the
  TS shape compatible), and add `attachments` to the NAPI Rust impl. The
  TS code already uses this field, so the NAPI surface must already be
  providing it somewhere — confirm in specifying phase.
- (b) add a separate `FspecBackend::get_work_unit_attachments(id) -> Vec<String>`
  RPC method that fetches them lazily on selection.

Option (a) is strongly preferred — single payload, matches TS exactly, and
the NAPI surface likely already provides this since the TS code reads it as
an array field.

## Existing TypeScript behavior preserved

This card touches ONLY:
- `codelet/fspec-tui/src/views/board*.rs` (new files + refactor)
- Possibly `codelet/rpc-types/src/lib.rs` to add `attachments: Vec<String>`
  to `WorkUnitInfo` (only if not already there — gated on `napi` feature).

The TS `UnifiedBoardLayout.tsx`, `WorkUnitTitle.tsx`, `WorkUnitDescription.tsx`,
`WorkUnitAttachments.tsx`, `WorkUnitMetadata.tsx` are all unchanged.

## Acceptance criteria sketch

- Box-drawing characters appear between columns (`│`, `┬`, `┼`, `┴`)
  and at the four corners (`┌`, `┐`, `└`, `┘`).
- Column widths sum exactly to `terminal_width - 2` (no off-by-one).
- Selected work unit's title, description, attachments (if any), epic,
  and status are visible above the column headers, in a 5-row strip.
- When no work unit is selected, the strip shows `No work unit selected`
  centered.
- Focused column header is rendered cyan + bold; other column headers are
  gray.
- Bug work units render red; task work units render blue; story work units
  render default fg.
- Selected cell in the focused column is `bg=green fg=black bold`.
- Total grid width adapts to terminal resize (verified by rendering at
  120w × 24h, 200w × 50h, and 80w × 20h).
