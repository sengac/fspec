# MENU-002 — Research: Board surface (v2 — two-zone GUI menu model)

Scope: wire the MENU-001 `MenuBar` + `MenuDropdown` into the single Board
view. The board header's row-3 `u Actions` chord is replaced by the live bar,
and the existing kanban navigation (`←/→`/h/l, wheel) becomes the front half
of a continuous ring that runs through the menu items and the session chips.

## What the board looks like

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│ ┏┓┏┓┏┓┏┓┏┓   Checkpoints: 3 Manual, 12 Auto                                     │
│ ┣ ┗┓┃┃┣ ┃                                                                           │
│ ┻ ┗┛┣┛┗┛┗┛                                                                           │
│ v0.10.8  Actions  Help │ #1●  #2⠋  #3○    ← row 3 of the header (was "u Actions")│
├──────────────────────────────────────────────────────────────────────────────────┤
│ 5-row details strip (unchanged)                                                   │
├┬──────────┬──────────┬─...─┤
│ BACKLOG   │ SPECIFYING│ ... │
```

- Row 3 of the header strip (`views/board/header.rs`, currently
  `keybinding_shortcuts::render(row3, …)` painting `CHORD_HINT`) now renders
  `menu_bar::render_menu_bar` into the SAME right-column rect (the rect that
  today holds `checkpoint_status` (row 0), the `─` divider (row 2), and the
  chord (row 3)).
- The `─` divider on row 2: red card — keep (cheap, separates the bar from
  the checkpoint row) vs. drop. Recommendation: keep; the bar's bg is only
  applied when the header has ≥ 1 row of width and the divider already
  exists.
- In non-mux mode the bar's Zone B is chips only (the user's explicit model:
  "when it's not in mux mode it just shows the agent status chips" — no
  Board/Agent/Files/Ckpts view labels).

## The continuous ring (the core of this unit)

Today (`views/board.rs::handle_event` + `store/board.rs`):

- `←/h` → `Action::FocusPrevColumn`; `→/l` → `Action::FocusNextColumn`
  (wraps at the ends, `focus_prev/next_column` in `store/board.rs` L157-168)
- `↑/k` → `SelectPrev`; `↓/j` → `SelectNext`
- Wheel: `ScrollLeft/ScrollRight` inside `last_content_area` →
  `FocusPrev/NextColumn` (`views/board/mouse.rs` L97-106); `ScrollUp/Down` →
  `SelectPrev/Next`
- Mouse click on a column header / cell → `SetFocusedColumn(idx)`
  (`views/board/mouse.rs::handle_left_click`)

The new ring (one focus position, walked by `←/→`/`h/l`/wheel, wrap-around):

```text
columns (7)  →  menu items (Actions, Help, …)  →  session chips (#1, #2, …)  →  columns …
```

- `→` on the LAST column (BLOCKED) lands on `Item(0)` (first menu item,
  dropdown CLOSED, inverse-video highlight on the item).
- `→` through the items: `Item(i)` → `Item(i+1)`; at the last item, `→` goes
  to `Chip(0)` ("when it runs out of menu items, it goes to the view
  switcher / agent chip switcher").
- `→` through the chips: wraps back to the FIRST column.
- `←` mirrors all of it (from column 0 → last chip; `Chip(0)` → last item —
  and the last item's dropdown OPENS per the v2 focus model; red card).
- **`↑/↓` behavior by focus row** (RED CARD — the one genuinely open
  semantic):
  - columns: unchanged (`SelectPrev/Next`).
  - menu items / chips: drop focus back INTO the board (focus returns to the
    focused column; selection unchanged; bar highlight clears). Rationale:
    the bar sits visually just above the details strip, so ↓/↑ read as
    "go back into the grid". The alternative (↑ opens the dropdown) breaks
    the mental split between "ring" and "dropdown" — rejected, red card.
- **`Enter` by focus row**:
  - columns: unchanged (`EnterWorkUnit` / mux `MuxEnterWorkUnit`).
  - menu item: open its dropdown (if closed) / execute highlighted row (if
    open).
  - chip: jump to / open that session — emits
    `Action::OpenAgentView(Some(sid))` (single view: flips to AgentView via
    `Navigator::apply_action`; the `.` New Agent / Shift+Right path already
    does exactly this) — red card: "jump to existing OR start new" vs.
    "always jump" — recommendation: chip exists ⟺ session exists, so always
    jump; a session-less "new" affordance is the `+`/`.` menu entry.
- **Wheel** rides the ring: `ScrollRight` on the last column focuses
  `Item(0)`; `ScrollLeft` on `Item(0)` focuses the last column — identical
  math to the keys (single `MenuRing::advance(±1)` helper; no duplicated
  logic, per the "Two Front Doors, One Source of Truth" rule).
- **Mouse click on a bar item**: click = move focus there (+ open dropdown
  for items) — focus+execute is NOT applied to items (Enter still
  distinguishes "open" from "execute"), but click on a CHIP = execute
  (jump/focus), mirroring the board's click-to-select-cards. Red card:
  click-on-item = focus-only vs focus+open-dropdown — recommendation:
  focus+open (matches Enter, one gesture).

## State placement

- **`BoardStore`** gains `menu_focus: Option<MenuFocus>` — additive field
  (RPC-012's field shape is "locked by rule [1]/[2]" but the pattern in this
  codebase for additive state is a new public field + public mutator; see
  `scroll_offsets` added for RPC-016, `checkpoint_counts` for RPC-015).
  The store already owns the focused column + per-column selection +
  scroll offsets, so the ring position belongs next to it:
  `menu_focus_prev/next()`, `menu_focus_from_columns()/to_columns()`
  (wrap logic lives on the store, mirroring `focus_next_column` — keeps
  `handle_event` thin).
  - Ring size: columns (7) + items (`MenuCategories::len()`, static) +
    chips (`session_attachments.len()` — the store already tracks
    `session_attachments: HashMap<String, SessionId>`; chip ORDER must be
    session-open order, which the store does NOT currently know — RED CARD:
    the store learns the open-session order (App feeds it, or the ring
    counts chips from the AgentViewStore passed at dispatch time).
    Recommendation: App dispatch computes the chip count from
    `agent_view_store.open_sessions().len()` and passes it to
    `board_store.set_menu_ring_size(7 + items + n)` — no new store
    dependency, the board store stays session-order-agnostic.
- **Dropdown open state**: `BoardView.open_menu: RefCell<Option<(usize,
  usize)>>` (category, cursor) — view-local interior mutability, the exact
  pattern of `details_selection: RefCell<Option<Selection>>` (COPY-009) and
  `last_details_area: Cell<Option<Rect>>`. Render caches the per-item bar
  rects in a `Cell<Option<Vec<Rect>>>` for click hit-testing (same caching
  pattern as `last_column_header_areas`).
- **`Action` enum** (in `components/mod.rs`) gains:
  - `MenuMove(i32)` — ring step (±1); dispatch walks `board_store.menu_focus`
    (or the mux/agent equivalents per MENU-003/004).
  - `MenuFocusToColumns` — `↑/↓` from the bar back into the focused column
    (clears `menu_focus`).
  - `MenuOpenDropdown(usize)` / `MenuCloseDropdown` /
    `MenuDropdownCursor(i32)` — the dropdown open/close/cursor.
  - `MenuExecuteItem { category, row }` — resolve through the registry to
    the existing `Action` (the dispatch does `MenuCategory.entries[row].action`
    + close; DRY with the bare-key arms — the bare keys in
    `views/board/keys.rs` keep emitting their specific actions, so the
    registry's `action` field stays the single source for WHAT executes).
  - `MenuChipActivate(usize)` — dispatch maps chip index → session id
    (App has both stores) → `OpenAgentView(Some(sid))`.
  - All are consumed by `App::dispatch` (the single mutation surface — the
    board view only EMITS; see `views/board.rs` "BoardView holds NO
    work-units state … Keyboard + mouse handlers emit Actions onto the bus
    that App::dispatch consumes").

## Render changes

- `BoardView::render_with_store(area, buf, store)` signature gains an
  `AgentViewStore` (or a pre-built `MenuSnapshot` — red card: passing the
  whole `&AgentViewStore` mirrors `Navigator::render_with_stores` which
  already passes both stores to the mux render; passing a snapshot keeps the
  board render store-pure and testable). Recommendation: **pass
  `&AgentViewStore`** — the Navigator call site
  (`navigator.rs::render_with_stores`, L252-253) already holds both, and the
  chips need `open_sessions()` + per-session slots which are `AgentViewStore`
  getters; a hand-rolled snapshot struct would duplicate the getter chain.
  The `menu_bar` painter itself stays pure (it takes `MenuSnapshot`), the
  board's render helper builds the snapshot.
- `views/board/header.rs::paint` passes the right-column row-3 rect to
  `menu_bar::render_menu_bar` instead of `keybinding_shortcuts::render`;
  the `keybinding_shortcuts.rs` module is deleted (or emptied to a shim).
- After painting the bar, cache the item/chip rects
  (`Cell<Option<Vec<Rect>>>`) for `mouse.rs` hit-testing.
- The dropdown is painted by the board render AFTER the box (same layering
  as the board details highlight overlay in `render.rs` L82-84), using
  `menu_bar::render_menu_dropdown` with the cached item rect of the open
  item. It must paint within the terminal (clamped) and is NOT a Compositor
  component (anchored, view-local — the turn-modal precedent).

## Key routing changes

- `views/board.rs::handle_event`:
  - the `Left/h` / `Right/l` arms and the wheel arms
    (`views/board/mouse.rs`) now call ONE helper
    `menu_ring::advance(self, store, +1/-1)` that either emits
    `FocusNextColumn` (still on a column) or `MenuMove` (on the bar) — the
    store decides the boundary.
  - new arms: `↑/↓` when `store.menu_focus.is_some()` → `MenuFocusToColumns`;
    `Enter` when on a menu item → open/execute; on a chip →
    `MenuChipActivate`.
  - `Escape`: unchanged (no bar state to clear — Esc on the board still
    reaches the App-level exit cascade; the bar has no "Esc clears focus"
    behavior when the dropdown is closed, since Esc there means exit —
    red card: should Esc on a FOCUSED (but closed) bar item just clear the
    highlight? Recommendation: no — keep Esc as exit; `↑/↓` leaves the bar).
- `views/board/keys.rs` mode-view arms (`a/c/f/d/m/./ /u`) are UNCHANGED —
  bare keys still execute directly; the dropdown/registry just makes the
  same actions discoverable. `u` still opens the help overlay (MENU-005
  repurposes it).
- `views/board/details_select.rs::is_selection_nav_key` must be extended to
  cover the new bar-navigation keys so an active strip text-selection is
  cleared when the ring moves (COPY-009 rule: "any selection-changing
  navigation key clears an active strip selection").

## Mouse changes

- `views/board/mouse.rs::handle_mouse` gains a FIRST hit-test (before the
  details-strip test? red card — the bar is above the details strip, so
  there is no geometric overlap; order: bar rects → details strip → content):
  - click on an item rect → `MenuMoveTo(item)` + open dropdown
    (`MenuOpenDropdown`)
  - click on a chip rect → `MenuChipActivate`
  - click ANYWHERE else (content, details, footer) →
    `MenuCloseDropdown` (GUI click-outside) when one is open — emitted
    alongside the normal click handling.
- Wheel on the bar row: red card — wheel over the bar with a dropdown open
  drives the dropdown cursor (like `↑/↓`); wheel over the bar closed =
  ignored. Recommendation: yes (the board search dialog already treats
  wheel as cursor movement).

## Tests that move / new tests

- Existing: `tests/board_period_new_agent_rpc395.rs`,
  `tests/board_search_dialog_board022.rs`, `mux009_board_m_key.rs`,
  `scrollback_scroll_rpc094.rs`, `view_board_unit_rpc015.rs`,
  `view_board_version_under_logo_board021.rs`,
  `tests/source_shape_board_search_board022.rs` — all assert the `u Actions`
  chord or the header rows; they get the new bar assertions (the two header
  snapshots regenerate in MENU-005).
- `rpc014-source-shape` (board split): the board vertical split is
  UNCHANGED (1+4+1+5+1+1+1+Min+1+1+1) — the bar replaces row 3 OF the
  existing 4-row header, so the split stays pinned; only the header's
  internal row-3 content changes.
- New integration suite (19-scenario parity with the BOARD-023 suite
  `board_actions_dialog_board023.rs`): ring walk both directions with
  wrap; `↑/↓` back into columns; Enter-open/Enter-execute; chip activation
  flips to AgentView; wheel parity with keys; click behaviors; dropdown
  true-modal key blocking (j/k/h/l swallowed while open); click-outside
  close; no second layer (re-click open item toggles closed — red card:
  click on the open item closes it, GUI parity).

## Open red cards

1. `↑/↓` on bar items — drop into the grid (my recommendation) vs. ↑ opens
   the dropdown.
2. Chip order source for the ring count (store-fed vs. dispatch-fed count —
   recommendation: dispatch-fed `set_menu_ring_size`).
3. Esc on a focused-but-closed bar item — no-op (my recommendation) vs.
   clears the highlight.
4. Click on an item = focus + open dropdown (my recommendation) vs.
   focus-only.
5. Row-2 divider kept (my recommendation).
6. Render signature: pass `&AgentViewStore` (my recommendation) vs. a
   pre-built `MenuSnapshot`.
