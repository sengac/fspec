# MENU-004 — Research: Mux surface (v2 — two-zone GUI menu model)

Scope: wire the MENU-001 `MenuBar` + `MenuDropdown` into mux mode as ONE
full-width bar at the top of the mux view (above all panes), suppress the
per-pane bars, and keep the per-pane focus-flash + divider + footer intact.

## What the mux view looks like

```text
Actions Help │ Board  #1●  #2⠋  #3○  Files  #3  Ckpts        ← ONE full-width bar, top of the mux view
┌───────────────────────────────────────┬───────────────────────────────────────┐
│ (board pane)                          │ #1 (AUTH-002: testing): model ...      │
│  row 3 of the board pane is BLANK     │ <scrollback ...>                       │
│  (no per-pane bar — top row is it)    │ > Type a message...                    │
│  BACKLOG SPEC ... │ cards ...         │                                        │
└───────────────────────────────────────┴───────────────────────────────────────┘
 MUX 3 panes [Board|Agent|Agent]  ●pane 2  /mux config · Shift+←/→ focus · drag divider   ← existing footer, unchanged
```

- The bar is painted by `views/multiplex/render.rs::render_with_stores` —
  after the focus flash, before dividers/footer (full width, top row).
- Zone B in mux = pane order with agent chips (the agreed model):
  `Board  #1●  #2⠋  Files  #3  Ckpts`. The existing mux footer already
  joins pane kinds as `Board|Agent|Agent` (`render.rs::paint_footer`
  L184-217) — the bar reuses the same `effective_panes()` +
  `window_session_ids()` source (BUG-163, `render.rs` L59-101) so the chips
  are the window sessions in pane order.
- Per-pane bars are suppressed (see below), so there is exactly ONE bar on
  screen.

## Render change

`render_with_stores` currently reserves the bottom row for the footer
(`body = area.height - 1`). It now reserves the TOP row for the bar too:

```rust
let bar = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
let body = Rect { x: area.x, y: area.y + 1, width: area.width,
                  height: area.height.saturating_sub(2) }; // reserve top bar + bottom footer
```

- `layout.body_area` (used by the divider/drag math in `mux_drag_axis`,
  `navigator_mux_events.rs` L146-174) must use the NEW body rect so the
  dividers still span only the body (they already paint `body.y..body.y+
  body.height` — they will automatically exclude the bar row once
  `body_area` is the shrunk rect; verify in the divider tests).
- Min height rises 3 → 4 (`if area.height < 4 { return }` at
  `render.rs` L42) — the bar needs 1 row.
- After painting panes, paint the bar into `bar`. The bar's snapshot is
  built from BOTH stores (`board_store` + `agent_store`), which
  `render_with_stores` already holds — no signature change needed (it takes
  both stores today).
- The per-pane bars are suppressed:
  - **Board pane**: the board's `header.rs` row 3 must paint BLANK in mux.
    Add a `menu_suppressed: bool` (or `menu_row: None`) to the board's
    render path — `BoardView::render_with_store` gains a flag (mirrors the
    MENU-003 `PaneSession.menu_row` flag). The 4-row header still renders,
    row 3 just stays empty.
  - **Agent panes**: `PaneSession.menu_row = false` (MENU-003 flag), so the
    5-constraint chrome is unchanged and no per-agent bar paints.
  - **Files / Checkpoints panes**: they have no header bar concept —
    unaffected (they never had a row-3 chord).

## Focus model in mux (the "trap" preserved)

The mux "trap" (`views/multiplex/keys.rs` — "unfocused panes receive NO
keyboard events") is preserved. The bar belongs to the MUX layer, not a
pane, so it is entered from the FOCUSED pane's edge rule:

- **Board pane focused**: the board's column⇄menu ring (MENU-002) — `→` off
  BLOCKED lands on the MUX-level bar's first item, then chips, then wraps.
  The ring's chip set is the GLOBAL open-session list (the bar is
  mux-level, chips are global — the agreed model), NOT the pane window.
  This means the board's `menu_ring` helper must be told the bar's chip
  count (the mux passes `agent_store.open_sessions().len()`), and the
  board's `menu_focus` state lives on `MultiplexLayout` (not the board
  store) while mux is active — RED CARD: single source of `menu_focus`
  (store vs. layout). Recommendation: keep it on `MultiplexLayout` in mux,
  on `BoardStore` in single-view; the `menu_ring` helpers are
  parameterized by the focus holder so the ring MATH is shared.
- **Agent pane focused**: the MENU-003 empty-input entry, but owned by the
  mux (the bar is mux-level). `←` with empty draft enters the mux bar at
  `Item(0)`; `←/→` ring-walk; Enter executes; char/`↑↓` drop back into the
  focused agent pane's input.
- **Files / Checkpoints pane focused**: those panes' key handlers are
  simple (Esc closes, arrows scroll); the bar is entered via `→`/`←` at
  their edge? RED CARD — recommendation: the bar is entered ONLY from the
  Board and Agent panes (the panes that have a meaningful "next" ring);
  Files/Checkpoints panes keep their Esc-close and do NOT feed the bar
  (keeps the model simple; the bar is always visible and mouse-clickable).
- **Mouse**: clicking a bar item/chip focuses the mux bar + opens/executes
  (the bar's hit-test rects are cached on `MultiplexLayout` by the render).
  Clicking a bar item does NOT change pane focus (the bar is an overlay
  control, not a pane). Clicking OUTSIDE the dropdown (on a pane) closes
  the dropdown and forwards the click to that pane (existing
  click-to-focus in `navigator_mux_events.rs::handle_mux_event`).
- `Shift+←/→` pane focus is UNTOUCHED (the mux "trap" keys, `keys.rs`
  L46-54) — they never enter the bar.

## State placement

- `MultiplexLayout` (or a small `MuxMenuBar` struct it owns) gains:
  - `menu_focus: Option<MenuFocus>`
  - `open_menu: Option<(usize category, usize cursor)>`
  - `menu_item_rects: Option<Vec<Rect>>` (cached at render for hit-test)
  - `menu_chip_rects: Option<Vec<Rect>>`
- The bar's snapshot is built fresh each frame from `board_store` +
  `agent_store` (both already passed to `render_with_stores`).

## Draw tick (animate the braille chips)

The run loop's 16 ms tick decides whether to redraw via
`tick_should_draw` operands (the existing gates: `is_mux_flash_active`,
`is_view_loading`, `is_input_animating` — see `navigator.rs`
`is_view_loading` / `is_mux_flash_active`). A new operand:

```
any open session is Running|Compacting AND the active surface paints a menu bar
```

- `Running`/`Compacting` → the chip's braille/`↻` glyph advances every 80 ms
  (`components/spinner::DOTS_INTERVAL_MS`); without the tick the spinner
  freezes (the exact BUG-194 / MUX-006 problem: "the 16ms tick stops
  drawing when nothing else demands a frame and the braille spinner
  freezes").
- Where to gate: `Navigator` already exposes `is_mux_flash_active` (L139-141)
  and `is_view_loading` (L118-131) which the run loop reads. Add
  `Navigator::is_menu_bar_animating()` → true iff `active_view` is
  Board/Agent/Mux AND `agent_store` has a Running/Compacting session. The
  run loop ORs it into `tick_should_draw`.
- The bar painter takes a `clock_ms: u64` (from the run loop's frame
  clock) so the braille frame is `current_frame_glyph(clock_ms)` — pure,
  testable (pass a fixed `clock_ms` in tests to pin the glyph).

## Actions (shared with MENU-002/003)

No NEW `Action` variants beyond those MENU-002/003 introduce
(`MenuMove`, `MenuFocusToColumns`, `MenuOpenDropdown`, `MenuCloseDropdown`,
`MenuDropdownCursor`, `MenuExecuteItem`, `MenuChipActivate`). `App::dispatch`
resolves them against the ACTIVE surface: in `ViewMode::Mux` it mutates
`MultiplexLayout.menu_focus/open_menu`; in `ViewMode::Board` it mutates
`BoardStore.menu_focus`; in `ViewMode::Agent` it mutates
`AgentView.menu_focus/open_menu`. One dispatch arm, three targets — the
surface is selected by `navigator.active_view` (the same way
`Navigator::handle_mux_event` / `handle_event` already branch).

## Tests that move / new

- `mux001__*` layout snapshots: body shrinks by 1 row (the bar) — the
  existing divider/flash/footer assertions must still pass (dividers now
  span `body.y+1..`); regenerate the affected mux snapshots.
- New: bar renders at the top full-width; per-pane rows suppressed (board
  row 3 blank, agent chrome 5-row); ring from board pane walks to chips and
  wraps; ring from agent pane (empty input) works; click-outside closes
  dropdown + forwards click to pane; `Shift+←/→` still moves pane focus and
  does NOT touch the bar; `is_menu_bar_animating` returns true when a
  session is Running in mux mode and false when all idle; draw-tick keeps
  the braille frame advancing across a fake clock.

## Open red cards

1. `menu_focus` holder in mux: `MultiplexLayout` (my recommendation) vs.
   reuse `BoardStore` — parameterize the ring helpers either way.
2. Bar entry from Files/Checkpoints panes: none (my recommendation) vs.
   `←/→` at their edge.
3. Chip source in mux: global open-sessions (my recommendation, matches the
   agreed "agent status chips" model) vs. the pane window only.
4. Min mux height 3→4 — confirm (a 3-row terminal with mux on simply won't
   render the bar; the panes still render with the old body height?
   RED CARD: degrade to no-bar on 3-row terminals vs. require ≥4).
