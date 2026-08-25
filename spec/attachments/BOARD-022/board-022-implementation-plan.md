# BOARD-022 — Board `/` Search Dialog (Implementation Plan)

## Goal

Pressing `/` on the Rust TUI Kanban board opens a centered search dialog —
visually identical in contract to the AgentView `@` file-search popup
(RPC-020/RPC-027, `dialog_theme` cyan accent) — that finds work units by
**ID**, **title**, or **description**. `Tab` toggles the active search mode
(`id → title → description → id`). Selecting a match focuses that card on
the board (column + row, viewport-aware) and closes the dialog.

## UX contract

| Key | Behaviour |
|-----|-----------|
| `/` (board, modifier-free) | Open dialog, consume key |
| printable / Backspace | Edit query, re-filter live |
| `Tab` | Cycle mode id → title → description → id; re-filter, selection → first match |
| `↑`/`↓`/PageUp/PageDown/Home/End | Navigate matches (wrap-around, scroll window) |
| `Enter` | Select highlighted match: board focuses its column + row, dialog closes |
| `Esc` | Close, no selection change |
| `/` while open | Ignored (no re-open) |

Dialog title row shows the active mode, e.g. `Search Work Units [id]`.
Empty-filter state: `(no work units match "<query>")` non-selectable row
(mirrors `file_search_popup_rows.rs`); empty board: `(board is empty)`.
Footer: `↑↓ Navigate │ Tab Mode │ Enter Select │ Esc Close`.

## Why no new RPC surface (RPC architecture)

The `@` file popup needs `search_files` (RPC-020) because file data is NOT
in the TUI. Work units ARE in the TUI: `BoardStore` holds the full
`Vec<WorkUnitInfo>` snapshot, kept fresh by the existing `list_work_units`
RPC + `work_units_rx` broadcast (RPC-006). So:

- **No new `FspecService` method** in `rust/rpc/src/lib.rs`.
- **No new `FspecBackend` trait method** in `fspec-tui/src/transport/mod.rs`
  (and therefore none in `embedded.rs` / `websocket.rs`).
- Filtering is a pure client-side function over the store snapshot.
- Cross-transport parity (RPC-020 convention) is preserved automatically:
  both transports serve the identical work-units snapshot.
- A **source-shape test** pins the invariant: no `search_work_units`
  substring in `rpc/src/lib.rs`, `transport/mod.rs`, `transport/embedded.rs`,
  `transport/websocket.rs`.

## Component / data flow

```
BoardView::handle_event  (views/board.rs)
  Char('/') modifier-free → emit Action::OpenWorkUnitSearch (consumed)
        │
App::dispatch (app/dispatch_work_unit_search.rs — new, mirrors dispatch_viewer.rs)
  handle_open_work_unit_search():
    idempotent on WORK_UNIT_SEARCH_DIALOG_ID
    seed = board_store.work_units() snapshot
    compositor.push(WorkUnitSearchDialog::new(seed).with_action_tx(tx))
        │
WorkUnitSearchDialog (components/work_unit_search_dialog.rs — new)
  Component, Priority::Foreground, id "work-unit-search-dialog"
  state: query: String, mode: SearchMode {Id, Title, Description},
         matches: Vec<String> (unit ids), selected: usize, scroll_offset
  handle_event:
    Char(c)   → query.push, re-filter
    Backspace → query.pop, re-filter
    Tab       → mode = mode.next(), re-filter, selected = 0
    Up/Down/… → navigate (scroll_viewport::wrap_index/ensure_visible)
    Enter     → emit Action::SelectWorkUnit(id) + remove self (callback)
    Esc       → remove self (callback)
  render: dialog_theme FspecDialog { accent: Cyan,
    title: "Search Work Units [<mode>]", rows via build_rows, footer }
        │
App::dispatch  Action::SelectWorkUnit(id)
  unit = board_store.find(id)
  board_store.set_focused_column(&unit.status)
  board_store.select_work_unit(&id, viewport_height)   ← new helper
```

## New / changed files (all < 300 LoC)

| File | Change |
|------|--------|
| `fspec-tui/src/components/work_unit_search_dialog.rs` | **NEW** dialog component (~200 LoC) + inline insta snapshot test |
| `fspec-tui/src/components/work_unit_search_rows.rs` | **NEW** `build_rows` helper (extracted, mirrors `file_search_popup_rows.rs`) |
| `fspec-tui/src/components/mod.rs` | `pub mod` + 2 new `Action` variants: `OpenWorkUnitSearch`, `SelectWorkUnit(String)` |
| `fspec-tui/src/views/board.rs` | `Char('/')` arm in `handle_event` (modifier-free guard like `d`/`a`) |
| `fspec-tui/src/views/board/keybinding_shortcuts.rs` | chord string gains `◆ / Search` |
| `fspec-tui/src/components/help_content.rs` | board list gains `"/             Search work units"` |
| `fspec-tui/src/app/dispatch_work_unit_search.rs` | **NEW** `handle_open_work_unit_search` + `Action::SelectWorkUnit` routing (mirrors `dispatch_viewer.rs`) |
| `fspec-tui/src/app/dispatch.rs` | route the 2 new actions to the new helper file |
| `fspec-tui/src/store/board.rs` | `pub fn work_units(&self) -> &[WorkUnitInfo]` + `pub fn find(&self, id) -> Option<&WorkUnitInfo>` accessors |
| `fspec-tui/src/store/board_viewport.rs` | `pub fn select_work_unit(&mut self, id, viewport_height)` — sets focused column's selection index + scroll offset so the unit is visible |
| `fspec-tui/tests/board_search_dialog.rs` | **NEW** integration tests (1:1 with scenarios) |
| `fspec-tui/tests/source_shape_board_search.rs` | **NEW** pins: dialog id const, `Action` variants, `/` arm in board.rs, absence of `search_work_units` in the 4 RPC/transport files |

## Pure filtering function (unit-tested, proptest)

```rust
pub enum SearchMode { Id, Title, Description }

pub fn filter_work_units(units: &[WorkUnitInfo], mode: SearchMode, query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    units.iter()
        .filter(|u| match mode {
            SearchMode::Id => u.id.to_lowercase().contains(&q),
            SearchMode::Title => u.title.to_lowercase().contains(&q),
            SearchMode::Description => u.description
                .as_deref()
                .is_some_and(|d| d.to_lowercase().contains(&q)),
        })
        .map(|u| u.id.clone())
        .collect()
}
```

- Case-insensitive substring; empty query → all units (Id/Title modes) or
  only units *with* a description (Description mode — empty query in
  Description mode lists description-bearing units; see rule 3).
- Ordering = board order (work-units vec order), so matches read top-to-bottom
  as the user sees them.

## Test plan (ACDD — tests before code)

Feature: `spec/features/board-search-dialog.feature` (tag `@BOARD-022`).
Integration test file `tests/board_search_dialog.rs` with `@step` comments,
driving `BoardView::handle_event` + `App::dispatch` with a mock backend
(`tests/common/mod.rs` already implements `FspecBackend` — unchanged):

1. `/` opens the dialog (compositor contains the dialog id; key consumed)
2. `/` while open is ignored (still exactly one dialog layer)
3. typing filters live (Id mode: "auth" → only AUTH-* ids)
4. Tab cycles modes and re-filters (id → title → description → id)
5. Description mode never matches units without a description
6. Enter selects: focused column == unit status, selected unit id == match,
   dialog removed from compositor
7. Esc closes without changing selection
8. zero matches → non-selectable empty-state row; Enter is a no-op
9. header chord contains "/ Search"; board help contains the `/` row
10. proptest: `filter_work_units` round-trip invariants (every returned id
    exists in input; empty query ⊆ full list; case-insensitivity)
11. source-shape: no `search_work_units` in rpc/transport files; dialog id
    const + Action variants present

## Out of scope

- Mouse support in the dialog (keyboard-only, like `AttachmentPickerDialog`)
- Fuzzy/regex matching (substring only)
- Persisting the last query or mode across open/close
- Any change to the TypeScript Ink board (Rust TUI only)
