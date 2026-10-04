# MENU-002 — AST Research: Board surface (ring + bar wiring)

Research of the existing code the continuous ring must integrate with.
Files read directly (AST pattern search in this workspace matches via
ripgrep below; the structural shapes are the listed `fn` signatures).

## 1. views/board.rs — BoardView + handle_event (265 lines)

- `pub struct BoardView { theme: Arc<Theme>, action_tx: Option<UnboundedSender<Action>>,
  last_viewport_height: Cell<u16>, last_content_area: Cell<Option<Rect>>,
  last_column_header_areas: Cell<Option<[Rect; 7]>>,
  last_column_content_areas: Cell<Option<[Rect; 7]>>,
  last_details_area: Cell<Option<Rect>>,
  recognizer: RefCell<SelectionRecognizer>,
  details_selection: RefCell<Option<Selection>>,
  selection_unit_id: RefCell<Option<String>>,
  details_press_active: Cell<bool>,
  clipboard: RefCell<Osc52Clipboard<Box<dyn Write + Send>>> }`
  → COPY-009 pattern: `RefCell<Option<Selection>>` interior mutability is the
  exact template for `open_menu: RefCell<Option<(usize, usize)>>`.
- `pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult`:
  - Esc with active strip selection → clear + consumed (COPY-009)
  - Shift+Right → `Action::OpenAgentView(selected_session(store))`
  - Enter → `Action::EnterWorkUnit(unit.id)` when a unit is selected
  - `is_selection_nav_key(key.code)` clears an active strip selection
  - `Left|h` → `FocusPrevColumn`; `Right|l` → `FocusNextColumn`
  - `Down|j` → `SelectNext`; `Up|k` → `SelectPrev`
  - `/` (no Ctrl) → `OpenWorkUnitSearch`; fallback `keys::handle_mode_view_key`
  → the ring must hook IN FRONT of the Left/Right arms and the Enter arm,
  and the Up/Down arms gain a `menu_focus` early branch.

## 2. store/board.rs — BoardStore (288 lines)

- `pub struct BoardStore { work_units, by_column: HashMap<String, Vec<usize>>,
  focused_column: usize, selected_index_per_column, scroll_offsets,
  session_attachments: HashMap<String, SessionId>, last_changed_id,
  checkpoint_counts }` — `#[derive(Debug, Default)]`
- `pub const COLUMN_ORDER: [&str; 7]` — the 7 ring column stops.
- `pub fn focused_column_index(&self) -> usize`
- `pub fn focus_next_column(&mut self)` → `(self.focused_column + 1) % 7`
- `pub fn focus_prev_column(&mut self)` → wrap to 6
- Additive-state precedent: `scroll_offsets` (RPC-016), `checkpoint_counts`
  (RPC-015) — both plain fields + public setter, no schema change.
  → `menu_focus: Option<MenuFocus>` + `menu_ring_size: usize` follow the same
  additive pattern with public mutators (RPC-012 field-shape rule is about
  the existing locked fields; additive fields have shipped 3× already).

## 3. views/board/render.rs — render_with_store (125 lines)

- Vertical split pinned by rpc014-source-shape:
  `[1, 4, 1, 5, 1, 1, 1, Min(0), 1, 1, 1]` (top border, 4-row header,
  separator, 5-row details, ├┬┤, column header, ├┼┤, content, ├┴┤, footer,
  bottom border).
- `header::paint(borders::inner_rect(split[1]), buf, store, &view.theme)`
  caches `last_column_header_areas`, `last_content_area`, `last_viewport_height`.
- `footer::render(borders::inner_rect(split[9]), buf, &view.theme)` — the
  footer hint text lives here (R11 updates it).
- The dropdown must paint AFTER this composition (anchored overlay), same
  layering as `paint_details_highlight`.

## 4. views/board/header.rs — paint (91 lines)

- `pub fn paint(area: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme)`
- Row 0: `checkpoint_status::render(row0, …)`; row 2: `─` divider loop;
  row 3: `keybinding_shortcuts::render(row3, buf, theme)` ← REPLACED by
  `menu_bar::paint_menu_bar(row3, buf, &snapshot, theme)` + cached rects.
- Right-column rect math: `right_start_x = padded.x + logo_w`, `right_w =
  padded_end - right_start_x` — the bar receives exactly `row3`'s rect.

## 5. views/board/keybinding_shortcuts.rs (50 lines)

- Single consumer of `board_shortcuts::CHORD_HINT`; whole module deleted by
  MENU-002 (the chord string survives only in the help dialog, MENU-005).
- `pub mod keybinding_shortcuts;` in views/board.rs removed; the
  `use super::keybinding_shortcuts;` in header.rs removed.

## 6. views/board/mouse.rs — handle_mouse (wheel + click routing)

- `pub(super) fn handle_mouse(view: &BoardView, event: &Event, store: &BoardStore) -> EventResult`
- Wheel: `ScrollLeft/ScrollRight` inside `last_content_area` →
  `FocusPrev/NextColumn`; `ScrollUp/Down` → `SelectPrev/Next`.
  → bar-row hit-test (cached item/chip rects) intercepts ScrollLeft/Right
  (ring) and ScrollUp/Down (dropdown cursor) BEFORE the content-area test.
- `fn handle_left_click(view, column, row, store) -> EventResult` (line 112)
  — column-header/cell click → `SetFocusedColumn`. Bar rect hit-test runs
  FIRST (bar is geometrically above the details strip; no overlap).

## 7. MENU-001 component contract (already on disk)

- `components/menu_bar/mod.rs`: `MenuFocus::{Item(usize), ZoneB(usize)}`,
  `ZoneBCell::{View{label, active}, Chip(usize)}`,
  `MenuSnapshot { focus, open_menu: Option<(usize, usize)>, zone_b, chips,
  clock_ms }` with `advance(focus, delta) -> MenuFocus` (ring math over
  items + Zone B cells, `rem_euclid` wrap), `dropdown_cursor(cat, cursor)`,
  `chip_for_cell(cell) -> Option<usize>`.
- `components/menu_bar/items.rs`: `CATEGORIES: [MenuCategory; 2]`
  (Actions: 7 entries, Help: 2 entries), `MenuAction::to_action(
  new_agent_target: Option<SessionId>) -> Action` — the dropdown execute
  path resolves through this (R4/R8 single source of truth).
- `components/menu_bar/chips.rs`: `ChipInput`, `build_chips(&[ChipInput],
  clock_ms) -> Vec<MenuChip>`; `index_prefix(n, total)` paints `#n` when
  n >= 1 (single session shows no `#`).
- `components/menu_bar/layout.rs`: `menu_bar_layout(&MenuSnapshot) ->
  MenuLayout` — the geometry pass whose `DisplayCell` rects the board
  caches for hit-testing.
- `components/menu_bar/paint.rs`: `paint_menu_bar(area, buf, &snapshot,
  theme)` (stateless; zero-area early return).
- `components/menu_bar/dropdown_paint.rs`: `render_menu_dropdown(rect, buf,
  &snapshot, theme)` (rounded border, cursor inverse-video, left-clamp,
  bottom-clip ellipsis).
- `views/board/board_shortcuts.rs` — re-export shim over the registry
  (BOARD-023 keeps compiling unchanged).

## 8. views/navigator.rs — the dual-store call site

- `pub fn render_with_stores(&self, area, buf, board_store: &BoardStore,
  agent_store: &mut AgentViewStore)` (line 244)
- `ViewMode::Board` arm (line 253): `self.board.render_with_store(area, buf, board_store)`
  → gains `agent_store` (immutable read via `open_sessions()`).
- `AgentViewStore::open_sessions(&self) -> &[SessionContext]`
  (store/agent_view.rs:124) — the chip source; `SessionStatus` from
  codelet_rpc_types.
- Mux arm (line 279) calls `mux_render::render_with_stores` — mux bar wiring
  is MENU-004, out of scope here (the board render signature change is
  still what mux reuses).

## 9. Action bus (components/mod.rs)

- Existing variants used by the ring: `FocusPrevColumn`, `FocusNextColumn`,
  `SelectPrev`, `SelectNext`, `EnterWorkUnit`, `SetFocusedColumn(usize)`,
  `OpenAgentView(Option<SessionId>)`, `OpenBoardHelp`,
  `OpenBoardExitConfirmation`, `OpenCheckpointsView`,
  `OpenChangedFilesView`, `OpenFoundation`, `OpenMuxConfigDialog`,
  `OpenAttachmentPicker`, `OpenWorkUnitSearch`.
- `App::dispatch` (app/) is the single mutation surface; BoardView only
  emits. New variants (architecture note): `MenuMove(i32)`,
  `MenuFocusToColumns`, `MenuOpenDropdown(usize)`, `MenuCloseDropdown`,
  `MenuDropdownCursor(i32)`, `MenuExecuteItem { category, row }`,
  `MenuChipActivate(usize)`, `MenuMoveToItem(usize)`.
