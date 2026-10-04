# MENU-003 — Research: Agent view surface (v2 — two-zone GUI menu model)

Scope: wire the MENU-001 `MenuBar` + `MenuDropdown` into the Agent view: a
new 1-row strip directly BELOW the `SessionHeader`, with the GUI menu
tracking (dropdown under the item) and the chip ring, entered without
disturbing the text cursor or Shift+arrow session cycling.

## What the agent view looks like

```text
#1 (MENU-002: specifying): claude-sonnet-4 [R] [V] [128k] [T:Med]   tokens: 12.4k↓ 3.1k↑ [42%]   ← SessionHeader (row 0, UNCHANGED)
Actions Help │ #1●  #2⠋  #3○                                                                                    ← NEW MenuBar row (chips only, non-mux)
──────────────────────────────────────────────────────────────────────────────────────────────────────────
  <scrollback ...>
> Type a message... 'Ctrl+J' newline, 'Shift+↑/↓' history, 'Shift+←/→' sessions, 'Tab' turns
```

- Non-mux: Zone B is chips only (per the agreed model). No view labels.
- The bar is painted by `pane_render.rs::render_session_pane` — it sits in
  the chrome between header and role banner.

## Layout change (the pinned-constraint problem)

Today `AgentView::pane_layout_constraints` (`views/agent.rs` L270-278) is
PINNED by `rpc013-source-shape.feature`
(`agent_view_splits_into_scrollback_input_and_footer_rows`):

```rust
[ Length(1), Length(role_height), Min(0), Length(1), Length(input_height) ]
//  header      role                scrollback   footer  input
```

and `pane_render.rs` builds `ChromeAreas { header: split[0], role: split[1],
scrollback: split[2], footer: split[3], input: split[4] }`
(`chrome_paint.rs` L16-22).

Options:

1. **Always insert the row** (6 constraints) and regenerate the
   source-shape test. Cost: every agent snapshot in the whole suite
   (`app_with_mock_backend*`, mux pane snapshots, the 300+ test binaries)
   shifts by one row. The board/agent/mux all gain the bar anyway, so the
   shift is uniform — but rpc013's feature text explicitly pins the 5-row
   shape and would need a superseding scenario.
2. **Layout flag on `PaneSession`** (`pub menu_row: bool`): the constraint
   list gains `Length(1)` at index 1 ONLY when the flag is set; single-view
   mode sets it always; mux panes set it false (MENU-004 paints the
   mux-level bar instead). The pinned 5-constraint list stays byte-identical
   for the flag-off path — the source-shape test keeps passing, and the
   flag-on path gets its own new scenario. `ChromeAreas` gains
   `menu: Rect` (a `Rect` with height 0 when off — the painter no-ops on
   zero height, same guard as every other painter).

**Recommendation: option 2** (flag). It matches how `PaneSession` already
carries per-pane selection into the render without touching shared state
(BUG-163: "carries the per-pane selection into `AgentView::render_session_pane`
without touching the live input / viewport state"), and it keeps the
single-view constraint list stable for every existing test that doesn't
opt in. (Note for MENU-004: the mux agent panes use flag=false, so their
chrome is unchanged and the mux-level bar is the only bar on screen.)

- `chrome_paint::ChromeAreas` gains `menu: Rect`;
  `paint_header_and_role` gains a `menu: Rect` param (or a new
  `paint_menu_bar` step after the header). Only the FOCUSED pane refreshes
  view-level geometry caches (`last_render_area` etc., BUG-163 rule) — the
  menu row needs no cached geometry except the item/chip rects, which live
  in `AgentView` as `last_menu_item_rects: Option<Vec<Rect>>` /
  `last_menu_chip_rects` (refreshed by the focused pane only, mirroring
  `last_input_area`).

## Focus entry — the tricky part

Current agent key handling (from `views/agent/dispatch.rs` +
`multiline_input_key.rs`):

- bare `←/→` = text cursor (inside `MultiLineInput`),
- `Shift+←/→` = session cycling (`navigate_next/prev`,
  `store/agent_view/navigation.rs` — end-of-list → CreateDialog / Board),
- `Tab` = turn-select toggle, `Esc` = back to board (stage cascade),
- bare character keys = type into the input.

New rule (agreed): **when the input draft is EMPTY, bare `←` enters the bar**
(focus → `Item(0)`… red card: `Item(0)` vs the focused session chip —
recommendation: `Item(0)`, the menu bar is the primary zone); once the bar
has focus:

- `←/→`: ring walk (items → chips → wrap to items), same ring as MENU-002.
- `Enter` on an item: open dropdown / execute row; `Enter` on a chip:
  activate (jump to that session — in single-view mode this is the
  equivalent of Shift+arrows: `store.navigate_to(index)` /
  `current_session_index` flip; the chip index IS the open-sessions index).
- `↑/↓` on an item with dropdown open: cursor; on bar focus closed: **drop
  focus back into the input** (symmetric with MENU-002's "drop into the
  grid").
- **Any character key** (and `↑/↓` into the text) → focus returns to the
  input (GUI menus: typing dismisses and forwards). Red card: does the
  character get FORWARDED to the input (typed) or swallowed? Recommendation:
  FORWARDED — `handle_event` returns Ignored after clearing bar focus, so
  the key reaches the `MultiLineInput` on the SAME event (the dispatch
  cascade in `views/agent/dispatch.rs` runs the input handler after the
  early arms; a "bar focus + character" arm clears focus and falls through
  instead of consuming).
- **Esc**: two-stage — dropdown open → close it; bar focused (closed) →
  clear bar focus (input regains focus, no view flip — this is the ONE
  place Esc does NOT mean "back to board" while bar-focused; red card —
  alternative: Esc always backs out entirely. Recommendation: two-stage,
  GUI parity, the bar is a small detour).
- While a dropdown is open: true-modal key consumption (BUG-161 parity) —
  `j/k`/wheel drive the cursor, `←/→` ring-walk + open adjacent, everything
  else consumed.

Implementation shape: `AgentView` gains
`menu_focus: Option<MenuFocus>` + `open_menu: Option<(usize, usize)>`
(plain fields — `AgentView` is already `&mut` through
`render_session_pane`; no `RefCell` needed, unlike the board).
`AgentView::handle_event` (the dispatch entry) gains an early arm BEFORE the
input arms: if `menu_focus.is_some()` → handle the menu ring/dropdown and
consume; else if the input is empty and the key is bare `←` → set
`menu_focus = Some(Item(0))` and consume. The empty-draft check is
`self.input.is_empty()` (used at `dispatch.rs` L192 for the Home key — same
seam).

**Mux note (for MENU-004):** in mux mode the agent PANES get `menu_row:
false` (no per-pane bar) — so this whole surface is single-view only. The
mux-level bar's Zone B in an agent pane is entered via the SAME logic but
owned by the mux (MENU-004); the code is shared through the
`menu_bar::ring` helpers so the semantics stay identical.

## Dropdown in the agent view

- Painted by `pane_render` AFTER the scrollback/input (anchored overlay,
  turn-modal precedent — `paint_turn_modal(area, buf, store)` renders into
  the full pane `area` after the main content).
- Geometry: under the item, clamped to the pane rect (in mux the bar is at
  the top of the mux, so the dropdown is always anchored to the MUX-level
  bar, not a pane — MENU-004 owns that case; here it's pane-local).
- Height clip: the agent pane can be short; clip + `⋯` row (MENU-001 rule).

## Session chip activation (single view)

Chip `j` (0-based open-sessions index) → jump to that session:
`store.set_current_session_index(j)`-style mutation via the existing
navigation store (RPC-097/096 — `navigate_next/prev` +
`first_open_session_id` in `store/agent_view/navigation.rs`); emitted as
`Action::MenuChipActivate(j)` and resolved in `App::dispatch` (which holds
`agent_view_store`), reusing the Shift+arrow dispatch arm's logic (DRY —
factor a `set_session_index_clamped`).

## Tests that move / new

- `rpc013-source-shape` — passes UNCHANGED with the flag-off path; gains a
  new scenario for the flag-on constraint list (6 rows, `menu` between
  header and role).
- New integration suite: empty-input `←` entry; ring walk; Enter
  open/execute (execute flips the view for `C`/`F`/`M`/`D`, jumps session
  for chips, opens the model/thinking/etc. dialog for action rows);
  character-forwarding (type `x` while bar-focused → input receives `x`,
  bar focus cleared); Esc two-stage; dropdown true-modal; click behaviors
  (the agent view's mouse dispatch — `mouse_dispatch.rs` — gains the bar
  hit-test using `last_menu_item_rects` / `last_menu_chip_rects`).
- The `INPUT_PLACEHOLDER_HINT` string (`views/agent.rs` L102-103) gains a
  `←` mention when the input is empty? Red card — recommendation: leave the
  hint as-is (the bar is visible; discoverability via the visible items).

## Open red cards

1. Layout: flag on `PaneSession` (my recommendation) vs. always-6-row
   (regenerate every snapshot + supersede rpc013 shape).
2. Entry key: only `←` (my recommendation) vs. also `→` when input empty.
3. Character key while bar-focused: forward to input (my recommendation)
   vs. swallow.
4. Esc while bar-focused (closed): clear bar focus (my recommendation) vs.
   immediate back-to-board.
5. Initial focus on entry: `Item(0)` (my recommendation) vs. focused
   session chip.
