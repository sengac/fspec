# RPC-017 — Priority reorder persistence (wire `[` / `]` to backend)

## TypeScript reference

### BoardView wiring
`src/tui/components/UnifiedBoardLayout.tsx:337-345`

```tsx
if (input === '[') {
  onMoveUp?.();
  return true;
}
if (input === ']') {
  onMoveDown?.();
  return true;
}
```

### BoardView implementation
`src/tui/components/BoardView.tsx` — search for `onMoveUp` / `onMoveDown`
handlers. They call into `useFspecStore.reorderWorkUnit(id, direction)`
which dispatches to NAPI.

### NAPI surface (search points)
- `codelet/napi/src/fspec.rs` — search for `prioritize_work_unit` /
  `move_work_unit` / `reorder_work_unit` exports.
- `codelet/napi/src/work_units_watcher.rs` — same.

The `prioritize-work-unit` fspec command exists in `src/commands/prioritize.ts`
with semantics:
- `position: 'top' | 'bottom' | number` → absolute position
- `before: <id>` / `after: <id>` → relative to another work unit

For BoardView `[` / `]` keys, the semantics are simpler: **move one position
up / down within the current sort order**, scoped to all work units (not
just the focused column).

## Current Rust state

`codelet/fspec-tui/src/app/dispatch.rs:152-155`:

```rust
Action::ReorderUp | Action::ReorderDown => {
    // RPC-012 architecture note [1]: persistence is out of scope
    // for this slice — placeholder no-op.
}
```

The actions are emitted by the BoardView on `[` / `]` (see
`codelet/fspec-tui/src/views/board.rs:95-102`) but App::dispatch
silently drops them.

## Target Rust behavior

### New FspecBackend methods

`codelet/fspec-tui/src/transport/mod.rs`:
```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-017: move a work unit one position up in the global priority order.
    /// No-op if the work unit is already at the top.
    async fn move_work_unit_up(&self, id: String) -> Result<()>;

    /// RPC-017: move a work unit one position down in the global priority order.
    /// No-op if the work unit is already at the bottom.
    async fn move_work_unit_down(&self, id: String) -> Result<()>;
}
```

### New RPC methods

`codelet/rpc/src/lib.rs`:
```rust
#[tarpc::service]
pub trait FspecService {
    // existing methods...

    async fn move_work_unit_up(id: String);
    async fn move_work_unit_down(id: String);
}
```

### Service impl

`FspecServiceImpl::move_work_unit_up` / `_down` delegate to a new helper
`codelet_core::work_units::move_work_unit(cwd, id, direction)` that
implements the same algorithm as `src/commands/prioritize.ts` for the
relative-position cases. The helper:
1. Loads the work-units store.
2. Finds the target's current index.
3. Swaps with the prev/next entry (no-op at boundaries).
4. Persists the new order.
5. Bumps the work-units watcher so subscribers re-render.

### Dispatch wiring

`codelet/fspec-tui/src/app/dispatch.rs`:
```rust
Action::ReorderUp => {
    if let Some(unit) = self.board_store.selected_work_unit() {
        let id = unit.id.clone();
        let backend = self.backend.clone();
        tokio::spawn(async move {
            let _ = backend.move_work_unit_up(id).await;
        });
    }
}
Action::ReorderDown => {
    // mirror
}
```

The work-units watcher will fire `Action::WorkUnitsLoaded(new_list)` after
the move persists, which `App::dispatch` already handles to re-seed
`BoardStore`. The auto-scroll math from RPC-016 keeps the moved unit
visible.

### NAPI surface preservation

The existing TS `fspec prioritize-work-unit` command and the
`useFspecStore.reorderWorkUnit` action route through their current code
paths. The new RPC methods are **additive** — they delegate to the same
`codelet_core::work_units::move_work_unit` helper that the existing
NAPI surface should also be migrated to use (eventually). For this card
only, add a NAPI export:

```rust
// codelet/napi/src/fspec.rs (new exports)
#[napi]
pub fn move_work_unit_up(cwd: String, id: String) -> napi::Result<()> {
    codelet_core::work_units::move_work_unit(&cwd, &id, Direction::Up)
        .map_err(to_napi_err)
}

#[napi]
pub fn move_work_unit_down(cwd: String, id: String) -> napi::Result<()> {
    codelet_core::work_units::move_work_unit(&cwd, &id, Direction::Down)
        .map_err(to_napi_err)
}
```

These are NEW exports — they do not replace the existing
`prioritize-work-unit` CLI surface.

## RPC/NAPI boundary contract

```
TS UI (BoardView.tsx) → useFspecStore.reorderWorkUnit(id, direction)
                     → fspec prioritize-work-unit ...  [TS pure path, today]
                     OR → napi.move_work_unit_up/down  [after this card, additive]

Rust TUI → FspecBackend::move_work_unit_up/down
       → FspecService::move_work_unit_up/down [tarpc]
       → codelet_core::work_units::move_work_unit(cwd, id, direction) [shared impl]
```

Both paths converge on the shared helper, so the on-disk state stays
consistent regardless of which UI moved the unit.

## Existing TypeScript behavior preserved

- `src/commands/prioritize.ts` — UNCHANGED.
- `src/tui/components/BoardView.tsx` — UNCHANGED.
- `src/tui/components/UnifiedBoardLayout.tsx` — UNCHANGED.
- `src/tui/store/fspecStore.ts` — UNCHANGED.

## Acceptance criteria sketch

- Pressing `[` on a selected work unit in the Rust BoardView moves it one
  position UP in the global priority order, persists to disk, and the
  BoardView re-renders with the unit in its new position.
- Pressing `]` mirrors with DOWN.
- `[` at the top is a no-op (no error, no log spam).
- `]` at the bottom is a no-op.
- After the move, the focused column's selection follows the moved unit
  (i.e. the unit stays selected, not the position).
- Two new NAPI exports `move_work_unit_up(cwd, id)` and `move_work_unit_down(cwd, id)`
  are visible in `codelet/napi/index.d.ts`.
- Two new RPC methods `FspecService::move_work_unit_up/down` exist and
  are tested against both `EmbeddedFspecBackend` and `WebSocketFspecBackend`.
- Existing TS `fspec prioritize-work-unit` command still works unchanged.
