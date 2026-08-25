# AST Research — BOARD-022 Board '/' Search Dialog

Research date: 2026-08-24. Tool: AstGrep (rust) + direct source reads.
Scope: `rust/fspec-tui` (BoardView, BoardStore, file-search popup, compositor,
dialog theme, dispatch).

## 1. BoardView key handling — insertion point for the `/` arm

`rust/fspec-tui/src/views/board.rs`

- `impl BoardView` (line 100) contains `pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult` (line 147).
- The keyboard match arms (lines 189–274) already contain modifier-free
  single-char shortcuts with the `!CONTROL` guard pattern:
  - `KeyCode::Char('f') | KeyCode::Char('F')` → `Action::OpenChangedFilesView` (line 233)
  - `KeyCode::Char('c') | KeyCode::Char('C')` → `Action::OpenCheckpointsView` (line 238)
  - `KeyCode::Char('d') | KeyCode::Char('D') if !CONTROL` → `Action::OpenFoundation` (line 245)
  - `KeyCode::Char('a') | KeyCode::Char('A') if !CONTROL` → `Action::OpenAttachmentPicker` (line 255)
  - `KeyCode::Char('.') if !CONTROL` → `Action::OpenAgentView` (line 268)
- **No `KeyCode::Char('/')` arm exists** — the key is free on the board.
- The new arm: `KeyCode::Char('/') if !CONTROL → emit(Action::OpenWorkUnitSearch); consumed`.
- BoardView holds no work-unit state; it only emits Actions (RPC-009 pattern).

## 2. BoardStore — data source for client-side filtering

`rust/fspec-tui/src/store/board.rs`

- `pub struct BoardStore` (line 40): `work_units: Vec<WorkUnitInfo>`,
  `by_column: HashMap<String, Vec<usize>>`, `focused_column: usize`,
  `selected_index_per_column`, `scroll_offsets`, `session_attachments`,
  `checkpoint_counts`.
- `WorkUnitInfo` (rpc-types/src/lib.rs:37) carries `id`, `title`,
  `description: Option<String>`, `status` — everything the three search
  modes need.
- Existing accessors: `column_units(column)`, `selected_work_unit()`,
  `set_focused_column(&str)`, `set_selected_index_for(&str, usize)`.
- **Missing (to add):** `pub fn work_units(&self) -> &[WorkUnitInfo]`
  (seed the dialog) and `pub fn find(&self, id: &str) -> Option<&WorkUnitInfo>`.

`rust/fspec-tui/src/store/board_viewport.rs`

- `pub fn select_index_in_focused(&mut self, index: usize, viewport_height: usize)` (line 104)
  is the viewport-aware row selector used by mouse click (RPC-023).
- **New helper (to add):** `pub fn select_work_unit(&mut self, id: &str, viewport_height: usize)`
  — resolve the unit's column + index, set focused column, set selection
  index, and set the column scroll offset so the unit is visible
  (reuses the same clamp/ensure-visible math as `select_index_in_focused`).

## 3. File-search popup — the reference widget

`rust/fspec-tui/src/views/agent/file_search_popup.rs`

- State: `filter: String`, `matches: Vec<String>`, `selected_index: usize`,
  `scroll_offset: usize`, `last_visible_rows: Cell<usize>`, `wheel: WheelVelocity`,
  `scrollbar_drag: ScrollbarDrag` (TUI-103).
- `set_filter` (line 129): resets selection + scroll + drag state on change.
- `set_matches` (line 140): clamps selection, `ensure_visible`.
- `handle_key` (line 260): Esc→Dismiss, Up/Down/PageUp/PageDown/Home/End
  navigate, Enter→SelectedEnter, Tab→SelectedTab; SHIFT/CONTROL modifiers
  → Ignored.
- `render` (line 305): `FspecDialog { accent: Accent::Cyan, title: "File Search",
  rows, footer: "↑↓ Navigate │ Tab/Enter Select │ Esc Close", min_width: 45 }`
  via `render_dialog`; visible rows = `(area.height - 8).clamp(1, 20)`.
- Rows built by `file_search_popup_rows::build_rows` (separate file, pure fn,
  empty-state literals `"(type to search files)"` / `"(no files match \"<filter>\")"`).

**BOARD-022 delta:** the popup is an inline agent-view widget; the board
dialog is a `Component` on the `Compositor` (like `AttachmentPickerDialog`)
because the board has no input buffer to splice into. Key routing and
rendering are otherwise identical; Tab semantics differ (mode toggle vs
select-without-space).

## 4. Compositor modal pattern — AttachmentPickerDialog (RPC-374)

`rust/fspec-tui/src/components/attachment_picker_dialog.rs`

- `Component` impl: `Priority::Foreground`, `id() = "attachment-picker-dialog"`.
- `handle_event`: Esc → `EventResult::Consumed(Some(remove_callback))`;
  Up/Down move (clamped, no wrap); Enter → emit action + remove callback.
- `remove_callback` boxes a closure calling `compositor.remove(id)`.
- `with_action_tx(tx)` builder for the App's `UnboundedSender<Action>`.

`rust/fspec-tui/src/app/dispatch_viewer.rs`

- `handle_open_attachment_picker()`: idempotent guard
  `compositor.contains(ATTACHMENT_PICKER_DIALOG_ID)`, seeds dialog from
  `board_store.selected_work_unit()`, `compositor.push(Box::new(dialog))`.
- `try_dispatch_viewer(&mut self, action) -> bool` routes the action
  variants from the catch-all arm of `App::dispatch`.

**BOARD-022 mirrors this exactly** with `dispatch_work_unit_search.rs`
+ `Action::OpenWorkUnitSearch` / `Action::SelectWorkUnit(String)`.

## 5. Event dispatch stage order

`rust/fspec-tui/src/app/events.rs`

- Stage 1: DisconnectDialog (Critical). Stage 2: **Compositor** (modals get
  first crack — a pushed search dialog will intercept `/`, Tab, Enter, Esc
  before BoardView sees them). Stage 3: Navigator (BoardView/AgentView).
  Stage 4: App shortcuts (`?`, board Esc, Ctrl+D).
- Paste: `compositor.handle_paste` first, then Navigator.
- TUI-110 central Press-only filter at the top of `handle_event`.

## 6. Docs to update

- `views/board/keybinding_shortcuts.rs` line 32: chord string
  `"C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent"`
  gains `◆ / Search`.
- `components/help_content.rs` `board_help_lines()` (line 22): gains
  `"/             Search work units"`.
- Snapshots affected: `tests/snapshots/app_with_mock_backend__help_dialog_*.snap`
  (board help content), `src/components/snapshots/...help_dialog...snap`,
  and any board render snapshots asserting the chord line
  (e.g. `view_board_unit_rpc015.rs`-style assertions).

## 7. RPC surface — confirmed unchanged

- `rust/rpc/src/lib.rs`: `FspecService` has `list_work_units()` (line 67) +
  `search_files(prefix, limit)` (line 196). No work-unit search method.
- `rust/fspec-tui/src/transport/mod.rs`: `FspecBackend` trait mirrors the
  service 1:1; `list_work_units()` + `work_units_rx()` (lines 67, 85).
- `embedded.rs` / `websocket.rs` delegate to the tarpc client.
- **Conclusion:** work-unit data is already in the TUI (BoardStore, kept
  fresh via the RPC-006 broadcast). No new RPC method; a source-shape test
  pins the absence of `search_work_units` in the four files above.

## 8. Test infrastructure

- `rust/fspec-tui/tests/common/mod.rs` provides a mock `FspecBackend`
  (implements every trait method incl. `search_files` at line 2642).
- Existing board integration tests to model after:
  `tests/board_period_new_agent_rpc395.rs` (drives `BoardView::handle_event`
  with `Event::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::empty()))`
  and asserts emitted `Action`s), `tests/board_open_attachment_rpc374.rs`
  (compositor dialog push/pop assertions).
- `BoardView::new(Arc::new(Theme::default()), tx)` + `unbounded_channel()`
  is the standard fresh-view fixture; `store.replace_work_units(vec![...])`
  seeds the store.
