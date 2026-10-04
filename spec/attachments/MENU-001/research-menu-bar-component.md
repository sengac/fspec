# MENU-001 — Research: MenuBar component (v2 — two-zone GUI menu model)

Scope: the shared 1-row `MenuBar` widget + the anchored `MenuDropdown` panel,
as a new `components/menu_bar/` module. Pure snapshot structs + stateless
painters, unit-tested on `TestBackend`. No view wiring in this unit.

## Concept (agreed model, user-directed)

One single-line menu bar with TWO zones — an explicit departure from the
earlier three-zone proposal (no always-visible action row):

```text
│  ZONE A: menu bar (GUI-style)        │  ZONE B: view / chip switcher        │
  Actions  Help                          #1●  #2⠋  #3○          (non-mux: chips only)
                                         Board  #1  #2  Files  #3  Ckpts  (mux: pane order)
```

- **Zone A** is a GUI menu bar: top-level **menu items** (labels, e.g.
  `Actions`, `Help`). Selecting a menu item (Enter or click) shows a
  **dropdown dialog positioned directly underneath that item** — simulating a
  GUI menu list. The dropdown lists the category's actions (key hint + label
  + description). While a dropdown is open, `→` goes to the next menu item
  (opening ITS dropdown — classic menu tracking); when the menu items run
  out, focus flows into Zone B.
- **Zone B** is the view switcher: in **mux mode** it lists the panes in
  pane order with agent panes shown as status chips (e.g.
  `Board #1 #2 Files #3 Ckpts`); in **non-mux mode** it shows only the agent
  status chips (the session switcher). There are NO view items in non-mux
  mode (board/agent/files/checkpoints navigation keeps its existing keys).
- Selecting a **chip** (Enter) is direct — no dropdown: it opens/jumps to
  that session (or focuses that agent pane in mux).

## Unified focus model (the key simplification)

There is ONE ring focus on the bar, `MenuFocus`, and "dropdown open" is a
visual/state flag on top of it:

```rust
pub enum MenuFocus { Item(usize) /* index into MenuCategories */, Chip(usize) /* index into open sessions */ }
```

- The ring: `Item(0) → … → Item(N-1) → Chip(0) → … → Chip(M-1) → Item(0)` —
  wrap-around, driven by `←/→` (and on the board, after the kanban columns;
  see MENU-002).
- **Enter on `Item(i)`**: menu closed → open the dropdown under item `i`;
  menu already open → execute the highlighted dropdown row (+ close).
- **Enter on `Chip(j)`**: execute the chip (jump/focus that session) — no
  dropdown.
- **`→` from `Item(N-1)`** (last menu item): focus moves to `Chip(0)` and
  the dropdown CLOSES (per user: "when it runs out of menu items, it goes to
  the view switcher / agent chip switcher").
- **`←` from `Chip(0)`**: focus moves to `Item(N-1)` and its dropdown OPENS
  (symmetric tracking — red card on the auto-open).
- **`↑/↓`** while a dropdown is open: move the highlighted row (wrap,
  BOARD-023 R4 parity). `j/k` map to `↓/↑` inside the open dropdown (vim
  parity; the board's j/k are ring/column keys otherwise).
- **Esc (two-stage)**: dropdown open → close it (focus stays on the item);
  dropdown closed → leave the bar (focus returns to columns / input / pane).
- While a dropdown is open, all other keys are CONSUMED as no-ops
  (true-modal, BUG-161 / BOARD-0023-R7 parity) — the surface behind stays
  frozen. Mouse wheel drives the highlighted row like `↑/↓`.

## Where it lives + why `components/`

- `rust/fspec-tui/src/components/` is the home for cross-view widgets
  (`dialog_theme.rs`, `board_keybinding_dialog.rs`, `spinner.rs`). The bar +
  dropdown are painted by three different surfaces (board, agent, mux) — a
  `views::agent` import from `views/board` is forbidden (sibling view modules
  must not import each other; TUI-106 note in `views/agent/spinner.rs`), so
  `components/` is the only correct dependency direction.
- Proposed file map (each < 300 LoC, workspace ceiling):
  - `menu_bar/mod.rs` — `MenuSnapshot` + `MenuFocus` + re-exports + entry
    fns `render_menu_bar(area, buf, &snapshot, theme)` and
    `render_menu_dropdown(area, buf, &snapshot, &dropdown, theme)`.
  - `menu_bar/items.rs` — the `MenuCategories` registry (evolved from
    `views/board/board_shortcuts.rs`).
  - `menu_bar/chips.rs` — pure fn: session snapshot → `Vec<MenuChip>`.
  - `menu_bar/dropdown.rs` — dropdown row builder + pure geometry
    (`dropdown_rect(item_rect, entries, available_height)`).
  - `menu_bar/paint.rs` — the 1-row bar painter + truncation ladder.

- `MenuSnapshot` is an OWNED struct built fresh per frame by each caller
  (same pattern as `SessionHeader<'a>` in `views/agent/header.rs` — "Built
  fresh per frame … so the widget itself stays free of cloned data on the
  painter's hot path"). No `action_tx`, no `&mut` — activation is decided by
  the caller; the widget only paints.

## The registry: evolving `board_shortcuts.rs`

- `views/board/board_shortcuts.rs` (120 LoC) is today's single source of
  truth for the six board shortcuts (`SHORTCUTS: [BoardShortcut; 6]` +
  `CHORD_HINT: "u Actions"`), consumed by the BOARD-023 dialog row builder
  and documented against the key arms in `views/board/keys.rs` (R11).
- It becomes the `MenuCategories` registry:

  ```rust
  pub struct MenuCategory {
      pub id: &'static str,      // stable slug ("actions", "help")
      pub label: &'static str,   // "Actions", "Help"
      pub entries: &'static [MenuEntry],
  }
  pub struct MenuEntry {
      pub key: &'static str,     // key hint painted in the dropdown (".", "/", "C", …)
      pub label: &'static str,   // "New Agent", "Search", …
      pub description: &'static str,
      pub action: MenuAction,    // the existing Action emitted on execute
  }
  ```

- **Default categories (RED CARD — the main open design question)**:
  - `Actions`: `.` New Agent · `/` Search · `C` Checkpoints · `F` Changed
    Files · `D` FOUNDATION.md · `M` Mux · `A` Attachments (the `a` key arm
    exists in `views/board/keys.rs` but was never in the SHORTCUTS table —
    include or not is a red card).
  - `Help`: `?` Help · `Esc` Exit.
  - Alternative splits (red card): a single `Actions` menu with all 8
    entries; or `File` (New Agent, Exit) / `Actions` / `Help`.
- `CHORD_HINT` dies (the live bar replaces the hint).
  `board_shortcuts.rs` either moves into `components/menu_bar/items.rs` or
  becomes a re-export shim (the TUI-106 precedent: `views/agent/spinner.rs`
  is a re-export shim over `components/spinner.rs` — "keeps every existing
  call site compiling unchanged with byte-identical behavior"). The BOARD-023
  dialog (MENU-005) migrates to the registry so bar, dropdown, and help
  dialog can never drift.
- `MenuAction` reuses the EXISTING `Action`s
  (`OpenCheckpointsView`, `OpenChangedFilesView`, `OpenFoundation`,
  `OpenAgentView(Option<SessionId>)`, `OpenWorkUnitSearch`,
  `OpenMuxConfigDialog`, `OpenBoardHelp`, `OpenBoardExitAttachment`-style
  picker for `A`, plus `OpenBoardHelp` / `OpenBoardExitConfirmation`) — NO
  new `Action` variants in MENU-001. (The ring-navigation variants land with
  MENU-002/003.)

## The MenuDropdown (anchored panel, NOT a centered modal)

- **Geometry**: anchored directly under the owning menu item:
  `Rect { x: item_x, y: bar_y + 1, width: max(24, widest_row + 4), height:
  entries + 2 }` (border top/bottom, no footer — GUI menus have none; red
  card: optional 1-row footer `↑↓ Navigate │ Enter Execute │ Esc Close` for
  discoverability).
- **Clamping**: if `x + width` overflows the terminal, shift left so it fits
  (the dropdown never paints off-screen); if `y + height` overflows the
  bottom, clip the visible rows and paint a dim `⋯` indicator row (red card:
  clip vs. flip upward — the bar is always at/near the top of every surface,
  so flipping is never needed in practice; clip is the simple answer).
- **Rows**: `dim key hint (4-cell column) │ label (fg) │ description (dim,
  truncated with `…`)`; cursor row inverse-video (bg cyan / fg black —
  recommendation, red card; consistent with the board's focused-column
  cyan+bold and selection bg-Green/fg-Black lifted to a highlight).
- **Border**: 1-cell box border in the accent (reuse the
  `dialog_theme::render_dialog` building blocks — `FspecDialog`/rows — but
  WITHOUT the centering; i.e. extract the frame+row painter from
  `dialog_theme` so the dropdown and the centered dialogs share one frame
  implementation; red card: share vs. hand-roll).
- **State**: the dropdown is VIEW-LOCAL (not a Compositor Foreground
  component — those are centered; the turn-modal / pause-prompt pattern in
  `views/agent/pane_render.rs` is the precedent: the view paints its own
  anchored overlay after its main content). Per-surface state:
  - Board: `BoardView.open_menu: RefCell<Option<(usize category, usize cursor)>>`
    (BoardView already uses `Cell`/`RefCell` interior mutability for
    `details_selection` etc. — same pattern).
  - Agent: plain `AgentView.open_menu: Option<(usize, usize)>` (AgentView is
    already `&mut` through its render/dispatch path).
  - Mux: `MultiplexLayout.open_menu` (the bar is mux-level — MENU-004).
- **Keys** (handled by the owning surface, using the widget's pure helpers):
  `↑/↓`/`j/k`/wheel: move cursor (wrap); `←/→`: ring move (open adjacent
  item's dropdown / close and move to Zone B per the focus model above);
  `Enter`: execute highlighted + close; `Esc`: close; everything else
  consumed (true-modal).
- **Mouse**: click on a row → execute + close; click on another menu item →
  move + open its dropdown; click anywhere else on the surface → close
  (GUI click-outside; red card: also close on any board grid click —
  recommendation: yes, any click outside the dropdown rect closes it).

## Zone B: session chips + (mux) view items

- Non-mux: chips only. One chip per open session: `#n` + status glyph, WU id
  appended when width allows (`#1 AUTH-002 ●`).
  - Running → animated braille `⠋⠙⠹…` (magenta, reuses
    `components/spinner::current_frame_glyph` + `DOTS_FRAMES`, 80 ms cadence)
  - Compacting → `↻` cyan · Paused → `!` yellow · Idle → `●` green-dim ·
    Interrupted → `✕` red · Cleared → chip omitted (red card)
- Mux: pane-order items — view panes as labels (`Board` / `Files` / `Ckpts`)
  interleaved with agent-pane chips (the window session of that agent slot;
  the BUG-163 window math in `views/multiplex/render.rs` L59-101 already
  computes `window_sessions` in pane order — same source).
- Data sources (all exist, per-session since BUG-194):
  - `store.open_sessions()` — order + `#n` (`(n, total)` via
    `session_index_for`, mirroring `chrome_paint.rs` L63-67/L139-142 where
    the `#N` prefix only paints when `index.0 >= 1`)
  - `store.session_status_for(sid)` → `SessionStatus` (rpc-types L233:
    Idle/Running/Interrupted/Paused/Compacting/Cleared + `as_str()`)
  - `store.work_unit_context_for(sid)` → WU id (same source the
    SessionHeader's `(ID: status)` segment uses, `chrome_paint.rs` L45-51)

  ```rust
  pub struct MenuChip {
      pub index: (usize, usize),   // (n, total)
      pub status: SessionStatus,
      pub wu_id: Option<String>,
  }
  ```

## Styling + geometry

- Row background `#333333` (same as `views/agent/header.rs::HEADER_BG` and
  `footer.rs::FOOTER_BG`); the painter takes a `Color` bg param (callers
  pass `HEADER_BG`) to stay theme-agnostic.
- 1-cell horizontal padding (mirror `horizontal_pad(area, 1)`,
  `views/agent/chrome.rs`, SessionHeader RPC-029).
- Zone separator: dim `│` between Zone A and Zone B, only when Zone B has
  content (no sessions → no Zone B, no separator; the whole bar can then be
  just `Actions Help`).
- Focused bar item: inverse-video (bg cyan / fg black) — recommendation,
  red card (alt: bold-cyan text like the focused column in
  `views/board/columns.rs`).
- Truncation ladder (narrow widths), pure + deterministic:
  1. drop WU ids from chips → `#1⠋` minimum form,
  2. fold chips beyond the 3rd into `+n`,
  3. (mux) drop view labels, keep the focused pane's label,
  4. absolute minimum: `Actions Help │ #1⠋`.
- Guards: no rendering when `area.width == 0 || area.height == 0`
  (SessionHeader::render parity).

## Test plan (unit, `TestBackend`)

- Registry: every entry has non-empty key/label/description + a valid
  action (mirror `slash_commands.rs::names_round_trip`); the six BOARD-023
  actions preserved verbatim; category slugs stable.
- Dropdown geometry: `dropdown_rect` anchored under item x; left-clamp at
  terminal edge; height clip + `⋯` indicator at a fixed short area.
- Dropdown rendering: row layout (key column fixed width, dim description
  truncation), cursor inverse cell, border, no footer (or footer — per red
  card resolution).
- Bar painter: fixed-width snapshots (80/100/120/200), each truncation
  ladder step pinned at a boundary width, focused-item inverse cell,
  `#333333` bg on every cell, 1-cell padding, chip glyph-per-status table,
  `#n` only when `n >= 1`.
- Prop test (workspace requirement for serialization/parsing-adjacent
  logic): painted row width never exceeds `area.width` for arbitrary
  (width, session count, statuses, focus, open-category) inputs.

## Open red cards

1. **Category split** — `Actions | Help` (my recommendation: all 7 board
   actions in `Actions`, `? Help` + `Esc Exit` in `Help`) vs. a single
   `Actions` menu vs. `File | Actions | Help`.
2. **Include the `A` Attachments entry** in `Actions`? (Key arm exists in
   `keys.rs` but was never in the SHORTCUTS table.)
3. **`←` from `Chip(0)` auto-opens** the last item's dropdown (symmetric
   tracking, my recommendation) vs. focuses it closed.
4. **Dropdown footer** — none (GUI purity, my recommendation) vs. 1-row
   hint (discoverability).
5. **Click-outside closes** the dropdown (my recommendation) — confirm.
6. **Cleared sessions** — omit chip (my recommendation) vs. dim.
7. **Share the `dialog_theme` frame painter** with the dropdown (my
   recommendation: extract, DRY) vs. hand-roll the box in `dropdown.rs`.
