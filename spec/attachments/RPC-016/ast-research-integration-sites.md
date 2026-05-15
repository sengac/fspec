# RPC-016 AST research — viewport + last_state_change_at integration sites

Generated 2026-05-15 during the RPC-016 specifying phase to lock down
the exact integration points the implementation will touch.

## 1. `WorkUnitInfo` struct (codelet/rpc-types/src/lib.rs:37)

AST query:
```
pattern: 'pub struct WorkUnitInfo { $$$FIELDS }'
language: rust
```

Match: `codelet/rpc-types/src/lib.rs:37` — the additive
`pub last_state_change_at: Option<String>` field will be appended after
`attachments` (the existing RPC-014 additive field) so the wire-shape
diff is a single line.

## 2. JSON deserialization site (codelet/core/src/work_units.rs:52)

AST query:
```
pattern: 'struct WorkUnitRecord { $$$FIELDS }'
language: rust
```

Match: `codelet/core/src/work_units.rs:52` — the existing
`#[derive(Deserialize)] #[serde(rename_all = "camelCase")]` record will
gain a `#[serde(default)] state_history: Vec<StateHistoryEntry>` field
(default = empty Vec for legacy files), and the
`impl From<WorkUnitRecord> for WorkUnitInfo` mapper at line 71 will
pick `state_history.last().map(|e| e.timestamp.clone())` into
`last_state_change_at`.

The `StateHistoryEntry` struct is new in this crate and only used
internally for deserialization — it does NOT cross the RPC boundary.

## 3. `BoardStore` struct (codelet/fspec-tui/src/store/board.rs:40)

AST query:
```
pattern: 'pub struct BoardStore { $$$FIELDS }'
language: rust
```

Match: `codelet/fspec-tui/src/store/board.rs:40` — the new
`scroll_offsets: HashMap<String, usize>` field will be inserted
between `selected_index_per_column` and `session_attachments` to keep
selection-related state grouped. Four new pub methods are added
(scroll_offset_for / set_scroll_offset_for / move_selection /
scroll_focused_column / select_first_in_focused / select_last_in_focused)
mirroring the existing `selected_index_for` pattern.

## 4. `Action` enum (codelet/fspec-tui/src/components/mod.rs:86)

AST query:
```
pattern: 'pub enum Action { $$$VARIANTS }'
language: rust
```

Match: `codelet/fspec-tui/src/components/mod.rs:86` — the four new
variants (ScrollFocusedColumnUp(usize), ScrollFocusedColumnDown(usize),
SelectFirstInFocused, SelectLastInFocused) will be appended after
`CheckpointCountsLoaded` (the RPC-015 variant). Variant ordering is
not load-bearing so this is purely additive.

## 5. `BoardView` struct (codelet/fspec-tui/src/views/board.rs:52)

AST query:
```
pattern: 'pub struct BoardView { $$$FIELDS }'
language: rust
```

Match: `codelet/fspec-tui/src/views/board.rs:52` — the new
`last_viewport_height: Cell<u16>` field will be appended so the
`handle_event` method (taking `&self`) can read the most recently
computed viewport_height when emitting Action::ScrollFocusedColumnUp /
Down. `Cell<u16>` is `!Sync` but the Component trait only requires
`Send`, matching the existing constraint.

## 6. `paint_content_rows` extraction
   (codelet/fspec-tui/src/views/board/columns.rs:53)

AST query:
```
pattern: 'pub(crate) fn paint_content_rows($$$ARGS) { $$$BODY }'
language: rust
```

Match: `codelet/fspec-tui/src/views/board/columns.rs:53` — the
existing row-by-row painter will be replaced by a viewport-aware
implementation in a new sibling module `views/board/viewport.rs`.
The new painter reads `store.scroll_offset_for(column)` to choose
between `↑` (row 0 when offset > 0), the unit at
`scroll_offset + row_index` (otherwise), or `↓` (last viewport row
when scroll_offset + viewport_height < units.len()). It also reads
`store.last_state_change_at_max()` and `store.session_for(id)` to
build the `⏩ 🟢 {id} {points} ⏩` cell text.

`columns.rs` keeps `paint_column_headers` + `pad_to_width` and either
re-exports `paint_content_rows` from the new module OR is replaced
entirely (decision deferred to implementation — the file-size
invariant is the gate).

## 7. `App::dispatch` (codelet/fspec-tui/src/app/dispatch.rs:18)

Will gain four new match arms (one per new Action variant) that call
the matching BoardStore methods inline. Arrow-key SelectNext/SelectPrev
arms will switch from `set_selected_index_for` to
`move_selection(±1, viewport_height)` — the viewport_height value is
read from `navigator.board.last_viewport_height()`.

## 8. No new RPC methods

Confirmed: `FspecService` trait in `codelet/rpc/src/lib.rs:51` is
unchanged. `FspecBackend` trait in
`codelet/fspec-tui/src/transport/mod.rs:57` is unchanged. The data
needed for the indicators flows through the existing
`list_work_units` payload (now carrying `last_state_change_at`) and
the existing `BoardStore::session_attachments` map populated by
`Action::AttachSession`.

## 9. NAPI surface — no changes

Confirmed by inspecting `codelet/napi/src/types.rs` / `git.rs` /
`session_manager.rs`. The TS source reads `WorkUnitInfo.attachments`
and derives `lastChangedWorkUnit` from `stateHistory[last].timestamp`
locally; both behaviours continue working unchanged. The new
`last_state_change_at` field is invisible to TS callers because the
TS surface does not query it.
