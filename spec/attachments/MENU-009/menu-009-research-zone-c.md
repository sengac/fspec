# MENU-009 — Research: Menu bar Zone C — right-aligned New Agent / Close Agent buttons

Scope: add a third zone (Zone C) to the shared menu bar component (MENU-001) —
right-aligned action buttons painted at the right edge of the bar row:

- **Board view**: `New Agent`
- **Agent view**: `New Agent` + `Close Agent`
- **Mux view**: none (empty Zone C — see open question Q1)

Both buttons reuse EXISTING bus actions — no new backend/RPC work:

- New Agent → `Action::OpenAgentView(Option<SessionId>)`
- Close Agent → `Action::AgentExitChoice { choice: ExitChoice::CloseSession }`

## 1. Current architecture (what we re-use)

### 1.1 Component layer — `rust/fspec-tui/src/components/menu_bar/`

The 2-zone bar is a shared PURE component (stateless painters over an owned
per-frame snapshot — the SessionHeader pattern). File map:

| File | Role |
|---|---|
| `mod.rs` | `MenuSnapshot` (built fresh by each surface per frame), `MenuFocus::{Item(usize), ZoneB(usize)}`, `ZoneBCell::{View { label, active }, Chip(usize)}`, and the **unified ring walk** `MenuSnapshot::advance` (items → Zone B cells → wrap; a `None` focus enters at `Item(0)` on +1 or the LAST cell on -1) |
| `items.rs` | `CATEGORIES` registry (`Kanban`/`Tools`/`Settings`/`Help`, MENU-008) + the `MenuAction` enum whose `to_action(new_agent_target)` maps onto existing bus `Action`s. Key precedent: `MenuAction::NewAgent → Action::OpenAgentView(target)` with the **R8 caller-substitution rule** — the registry is `&'static`; the caller substitutes its live `Option<SessionId>` at execute time |
| `chips.rs` | pure chip snapshot builder (`build_chips`, R9 `Cleared` drop, R4 status glyphs) |
| `layout.rs` | pure geometry pass `menu_bar_layout(area, snap) → Option<MenuLayout>`. `MenuLayout { level, item_rects, separator, cells, cell_rects }`. Left→right: Zone A items (1-cell gaps), `│` separator (3 cells), Zone B cells (2-cell gaps). 1-cell horizontal padding (R1). **R6 truncation ladder** (most → least detailed): 0 full (chips show WU id suffix) → 1 drop WU id → 2 fold chips beyond 3rd into `+N` → 3 drop non-active view labels → 4 drop Zone B entirely (level 4 costs zero width). A `proptest` pins: the painted row never exceeds `area.width` |
| `paint.rs` | 1-row painter `paint_menu_bar(area, buf, snap, theme) → Option<MenuLayout>`: `#333333` bg on EVERY row cell (R1 — already covers the full width, so a right-aligned Zone C inherits it for free), inverse-video highlight (`bg Cyan / fg Black / bold`) for the focused/open Zone A item (R5) and for the store's CURRENT session chip (MENU-006) |

### 1.2 Three surfaces, three state holders

All three paint the same component but own the ring/dropdown state and the
cached per-frame geometry differently. Zone C must slot into each:

**Board** — `views/board/{menu_snapshot,menu_keys,menu_mouse,render}.rs` + `store/board_menu.rs`
- Snapshot: `menu_snapshot::build_snapshot` (full `CATEGORIES` + chips from
  `active_menu_session_ids` + `build_chips`, R9 `Cleared` drop). The shared
  helpers `active_menu_session_ids` / `chip_inputs` / `zone_b_and_chips` are
  ALSO used by the agent and mux snapshot builders — one source of truth.
- Ring state: `BoardStore` (`store/board_menu.rs`) — the ring =
  `7 columns → 4 items → chips` (`RING_COLUMNS = 7`). The chip count is fed
  at the top of EVERY dispatch tick by `App::feed_menu_ring_size` →
  `set_menu_ring_size` (R8 lockstep). Zone C is a compile-time constant per
  surface, so NO per-tick feed is needed for it.
- Geometry cache: `BoardView.last_menu_bar_geometry` (a `RefCell` holding
  `MenuBarGeometry { bar_y, layout: MenuLayout, open_panel }`).
- Keys (`menu_keys.rs`): open-dropdown modal gate → bar-focus arms
  (Up/Down → `MenuFocusToColumns`, Enter → open/activate, Left/Right →
  `MenuMove`) → no-focus edge (Left/Right walk from the column).
- Mouse (`menu_mouse.rs`): click item → focus+toggle dropdown; click chip →
  `MenuChipActivate`; BUG-196 R3: an outside left click emits
  `Action::MenuDismissBar` INLINE (close + de-select) — its hit-test
  (`close_outside`) excludes the open panel and the item rects. A Zone C
  rect MUST be added to that exclusion (a click on the button is not an
  "outside" click).

**Agent** — `views/agent/{menu_render,menu_state,menu_keys,menu_mouse}.rs`
- Zone A is a **surface-local `&'static` constant** `AGENT_ZONE_A`
  (`menu_render.rs`) — the single `Board View` item, MENU-007. This is the
  EXACT precedent to follow for Zone C (surface-local static slice; shared
  painter stays surface-agnostic).
- State: `AgentView.menu_state` — a PLAIN `MenuBarState` field
  (`{ focus, open (always None — agent bar is dropdown-free), chip_count,
  bar_row, item_rects, cells, open_panel }`). Refreshed by the FOCUSED
  bar-row pane only (the BUG-163 rule); `clear_geometry()` resets caches.
- Ring: computed IN the view (`menu_keys.rs::walk_menu_ring`) —
  `len = AGENT_ZONE_A.len() + chip_count`. Enter on `Item(0)` →
  `activate_board_view_item` (`Action::BackToBoard`); Enter on a chip →
  `activate_menu_chip`.
- `PaneSession.menu_row` gates the bar row: single-view = true (6-constraint
  layout with the bar row under the SessionHeader); mux agent panes = false
  (the mux top-row bar is the only bar on screen, MENU-004 R-SUPPRESS).

**Mux** — `views/multiplex/{menu_snapshot,menu_state,menu_render,menu_keys,menu_mouse}.rs`
- State lives on `MultiplexLayout` (RED CARD 1 — the bar belongs to the mux
  surface): `menu_focus`, `open_menu`, `menu_chips` (fed by
  `refresh_menubar_ring`), cached `menu_item_rects` / `menu_cells` /
  `menu_open_panel` / `menu_bar_painted`.
- The ring walk runs `MenuSnapshot::advance` PER FRAME inside
  `App::dispatch_menu_mux` against a fresh snapshot — so a `zone_c` field on
  `MenuSnapshot` flows through with no mux-side ring code changes (empty
  slice → zero effect).
- Zone B = pane view labels (effective panes, non-agent kinds) + the GLOBAL
  session chips.
- Mouse: `classify_bar_mouse` → `MuxBarMouseDecision` enum (Claimed /
  Swallowed / CloseOutsideThenLand / Ignore / Pass).
## 2. Actions already exist — no backend work

### New Agent
`Action::OpenAgentView(Option<SessionId>)` (components/mod.rs L206), routed by
`App::handle_open_agent_view` (app/dispatch_session_cycle.rs):
- `Some(sid)` → `set_navigation_target(sid)` + flip `active_view = Agent`.
- `None` → probe `agent_view_store.first_open_session_id()`:
  - Some → resume that session (no dialog), flip to Agent
  - None → mount `CreateSessionDialog` over the Board (user stays on board
    until they confirm — RPC-097 reopen #1 contract)

All existing producers funnel through the same semantics (single source of
truth to reuse, DRY):
- the board `.` key (`views/board/keys.rs` L104-108) and Shift+Right
  (`views/board.rs` L184-187), both using `view.selected_session(store)`
  (the selected work unit's attached session)
- the Kanban dropdown row (`MenuAction::NewAgent`, R8 caller substitution in
  `App::dispatch_menu` / `dispatch_menu_mux` / `dispatch_board_keybinding`)
- the 'u' menu-bar help dialog (MenuBarHelpDialog row 0)

### Close Agent
`Action::AgentExitChoice { choice: ExitChoice::CloseSession }`
(components/mod.rs L547), emitted by the Esc-cascade
`ExitConfirmationDialog` (Priority::Critical, id
`"exit-confirmation-dialog"`) and routed by
`App::handle_agent_exit_choice` (app/dispatch_agent_exit.rs) — the 6-step
teardown (TS `destroySession()` parity, sessionService.ts:620-647):

1. SNAPSHOT — work-unit binding + session id before any mutation
2. BoardStore detach (`board_store.detach_session(wu_id)`)
3. `agent_view_store.remove_session_if_open(&session)` + `mux_sync_window()`
4. `agent_view_store.set_current_work_unit(None, None)`
5. `tokio::spawn(backend.destroy_session(session))` (kept in
   `pending_tasks`)
6. dispatch `Action::BackToBoard`

`ExitChoice::Detach` (stay in Agent, session keeps running) and
`ExitChoice::Cancel` are the dialog's other options; the button maps
EXCLUSIVELY to `CloseSession` (Q3 pending confirmation).

## 3. Proposed design (DRY)

### 3.1 Data model — mirrors Zone A exactly (static, store-free)

`components/menu_bar/mod.rs`:

```rust
/// One Zone C action button (right-aligned; surface picks the slice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneCButton {
    pub label: &'static str,
    pub action: MenuAction,
}

impl MenuSnapshot {
    pub zone_c: &'static [ZoneCButton],   // + Default::default() = &[]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuFocus {
    Item(usize),
    ZoneB(usize),
    ZoneC(usize),          // NEW — index into snap.zone_c
}
```

`items.rs`: add one `MenuAction` variant:

```rust
MenuAction::CloseAgent,  // → Action::AgentExitChoice { choice: ExitChoice::CloseSession }
```

Per-surface constants (the `AGENT_ZONE_A` precedent), each a `&'static`:
- Board: `&[ZoneCButton { label: "New Agent", action: MenuAction::NewAgent }]`
- Agent: `&[New Agent, Close Agent]`
- Mux: `&[]` (Q1)

### 3.2 Ring focus — extend the unified ring (R3), do not fork it

Ring order: **items → Zone B → Zone C → wrap**.

- `MenuSnapshot::advance` (mod.rs, used by the mux dispatched walk + tests):
  `len = zone_a.len() + zone_b.len() + zone_c.len()`; the `next` mapping
  gains a `ZoneC` arm. Empty `zone_c` (mux) ⇒ byte-identical behavior.
- `BoardStore::menu_move` (store/board_menu.rs):
  `len = RING_COLUMNS + items + chips + ZONE_C_COUNT` (a compile-time
  constant, e.g. `const BOARD_ZONE_C: usize = 1;` — no per-tick feed).
- `AgentView::walk_menu_ring` (views/agent/menu_keys.rs):
  `len = AGENT_ZONE_A.len() + chip_count + AGENT_ZONE_C.len()`.

Rationale: mouse-only buttons would break the R3 "one continuous ring"
invariant and the board/agent key-arm parity (Q2 recommended: keyboard-
focusable).

### 3.3 Layout + paint — right-aligned, same highlight

`layout.rs`:
- `MenuLayout` gains `zone_c: Vec<DisplayZoneC>` + `zone_c_rects: Vec<Rect>`,
  where `DisplayZoneC { label, orig }` mirrors `DisplayCell` (the `orig`
  index keeps focus correct through any truncation).
- Positioning: right edge of the inner area:
  `x_last = inner.x + inner.width - zone_c_width`; buttons laid out
  right-to-left with 2-cell gaps (the Zone B inter-cell gap — visual parity).
- Truncation: the R6 ladder fit check becomes
  `zone_b_width + zone_a_width + zone_c_width (+ inter-zone gap when Zone B
  present) <= inner.width`. Zone C is the OPTIONAL convenience zone: when the
  row cannot afford it, Zone C drops BEFORE Zone B content (a new step
  between current level 3 and 4 — or: level selection first tries the full
  row, then drops Zone C, then falls through the existing ladder). The
  existing proptest bound ("painted row never exceeds area.width") is
  preserved by extending the assertion to `zone_c_rects` too.

`paint.rs`:
- After the Zone B loop, paint Zone C labels at `zone_c_rects` with
  `theme.fg` dimmed or plain (TBD in spec — likely plain `theme.fg` to read
  as actionable buttons) + the SAME `inverse_style()` highlight when
  `MenuFocus::ZoneC(i)` (reuse the existing `inverse_style()` helper; the
  #333333 full-row bg already covers the area, R1).

### 3.4 Hit-testing & keys (the per-surface arms, all small)

**New bus token (single source for both surfaces):**
`Action::MenuZoneCActivate(usize)` — index into the surface's `zone_c`
slice. (Could alternatively reuse `MenuMoveToItem`-style dispatch per
surface, but a dedicated token keeps the dispatch arm readable and avoids
overloading the Zone A item index.)

**Board:**
- `menu_mouse.rs`: after the `cell_rects` loop, a `zone_c_rects` pass →
  `view.emit(Action::MenuZoneCActivate(idx))` → `Consumed`. AND add the
  Zone C rects to the BUG-196 `close_outside` exclusion (a click on the
  button must NOT fire `MenuDismissBar` and must NOT re-open a dropdown).
- `menu_keys.rs`: the `MenuFocus` match gains a `ZoneC(index)` arm (Enter →
  `MenuZoneCActivate(index)`; Left/Right → `MenuMove` as usual; Up/Down →
  `MenuFocusToColumns`).

**Agent:**
- `menu_mouse.rs`: `zone_c_rects` pass in the left-click arm → activate.
- `menu_keys.rs`: `ZoneC` arm in the focus match (Enter activates; the
  `menu_state` clears focus on activation, parity with chip/board-view
  item). `walk_menu_ring` length includes `AGENT_ZONE_C.len()`.
- `menu_state.rs` (`MenuBarState`): cache `zone_c_rects: Option<Vec<Rect>>`
  (+ optionally a `zone_c_count` for the ring, though it's a compile-time
  constant so the view can read `AGENT_ZONE_C.len()` directly) — refreshed
  by the focused pane (BUG-163) and cleared by `clear_geometry()`.

**Mux:**
- Geometry-cache field passthrough in `MultiplexLayout::cache_menu_geometry`
  (store `zone_c_rects` alongside `item_rects`/`cells`). With `zone_c = &[]`
  every arm is inert automatically. `classify_bar_mouse` gains a no-op
  `zone_c_rects` pass for symmetry (or is left untouched if we want the
  mux truly untouched — decision at implementation time).

**App dispatch (`app/dispatch_menu.rs`):**
- Add `Action::MenuZoneCActivate(_)` to `is_menu_action` + the
  `dispatch_menu` match (Board/Agent half) AND `dispatch_menu_mux` (no-op /
  defensive arm).
- Resolve per surface:
  - Board: `OpenAgentView(board_store.selected_session())` — identical to
    the `.` key / Kanban row (R8).
  - Agent idx 0 (`New Agent`): `OpenAgentView(current_session())`;
    Agent idx 1 (`Close Agent`): `AgentExitChoice { CloseSession }`.
  - Mux: no-op (no Zone C painted).

### 3.5 Files touched (~16), each a small pattern-following diff

Component:
- `components/mod.rs` — 1 new `Action::MenuZoneCActivate(usize)` variant
- `components/menu_bar/mod.rs` — `ZoneCButton`, `MenuSnapshot.zone_c`,
  `MenuFocus::ZoneC`, `advance` extension
- `components/menu_bar/items.rs` — `MenuAction::CloseAgent` (+ `to_action`)
- `components/menu_bar/layout.rs` — right-align + ladder (biggest chunk;
  may extract a `zone_c.rs` if this file exceeds the 300-LoC ceiling)
- `components/menu_bar/paint.rs` — Zone C paint arm

Store:
- `store/board_menu.rs` — `menu_move` ring length (Zone C constant)

Board:
- `views/board/menu_snapshot.rs` — pass `zone_c` into the snapshot
- `views/board/menu_keys.rs` — `ZoneC` key arm
- `views/board/menu_mouse.rs` — `zone_c_rects` hit-test + close_outside fix
- `views/board/render.rs` — (no change — `paint_menu_bar` returns the
  extended `MenuLayout`; the cache stores it whole)

Agent:
- `views/agent/menu_render.rs` — `AGENT_ZONE_C` constant + snapshot field
- `views/agent/menu_state.rs` — `zone_c_rects` cache
- `views/agent/menu_keys.rs` — `ZoneC` arm + ring length
- `views/agent/menu_mouse.rs` — `zone_c_rects` hit-test

Mux:
- `views/multiplex/menu_snapshot.rs` — `zone_c: &[]` (or omit)
- `views/multiplex/menu_state.rs` — cache passthrough (optional)
- `views/multiplex/menu_render.rs` — (no-op; empty zone_c)

App:
- `app/dispatch_menu.rs` — `MenuZoneCActivate` arm (Board/Agent/Mux)

Test:
- `tests/menu009_zone_c_buttons.rs` (new integration test)
- unit tests in `layout.rs` / `mod.rs` (+ the existing proptest extended
  with `zone_c_rects`)

## 4. Draft acceptance criteria (scenario list for the feature file)

Proposed feature file:
`spec/features/menubar-component-zone-c-right-aligned-new-agent-close-agent-buttons.feature`
(`@menu-bar`, `@tui-component`, `@agent-view`, `@board-surface`, `@MENU-009`)

Scenarios:
1. Board bar paints `New Agent` right-aligned (no `Close Agent`)
2. Agent bar paints `New Agent` + `Close Agent` right-aligned
3. Mux bar paints no Zone C buttons (byte-identical row to today)
4. New Agent on the board activates the `.`-key semantics (selected unit's
   session / resume first open / CreateSessionDialog) — Enter and click
5. New Agent on the agent view jumps into / resumes the current session —
   Enter and click
6. Close Agent on the agent view runs the Close Session teardown (session
   removed from open list, board detach, destroy_session, BackToBoard) —
   Enter and click
7. Zone C buttons carry the inverse-video ring-focus highlight
   (bg Cyan / fg Black / bold) exactly like Zone A items
8. Ring wrap: last Zone B cell → New Agent → (agent: Close Agent) → back to
   first item; Left from first item wraps to the last Zone C button
9. Left from a Zone C button lands on the last Zone B cell (both surfaces)
10. Truncation: at a width that cannot afford Zone C, the buttons drop
    before Zone B content (Zone B ladder unchanged); painted row never
    exceeds area width (proptest)
11. A left click on a Zone C button does NOT close an open board dropdown
    (BUG-196 R3 exclusion) and does NOT fall through to the board content
12. No open sessions (agent): the bar still shows both Zone C buttons (they
    are independent of the chip list)
13. Mux: Zone C absent ⇒ dispatched `MenuMove` ring walk is byte-identical
    to pre-MENU-009 (regression guard via the existing menu004 tests)

## 5. Open design questions (pending user confirmation)

- **Q1 — Mux coverage**: user said "board view and agent view". Default:
  NO Zone C in the mux top bar (`&[]`).
- **Q2 — Keyboard focusability**: default: Zone C joins the unified ring
  (mouse-only buttons would break the R3 invariant).
- **Q3 — Close Agent semantics**: default: exactly `ExitChoice::CloseSession`
  (destroy backend session + detach + BackToBoard), NOT Detach.
- **Q4 — Board New Agent semantics**: default: reuse the `.` key target —
  selected work unit's session, else resume first open, else
  CreateSessionDialog.
- **Q5 — Labels**: `New Agent` / `Close Agent` as-is (12 / 11 cells).

## 6. Estimation & risks

Estimate: **~8 story points** (complex: ring math in three holders + store +
component layout/paint + dispatch + integration tests). Splittable:
(a) component layer (model / ring / layout / paint + unit & proptest) ≈ 3–5
pts, (b) surface wiring + dispatch + integration tests ≈ 5 pts.

Risks / watch-outs:
- `layout.rs` and `menu_state.rs` (agent) are close to the 300-LoC ceiling —
  extract `zone_c.rs` in the component if needed (workspace 300-LoC rule).
- The R6 ladder proptest must keep its invariant after adding Zone C width
  to the fit check (extend the bound to `zone_c_rects`, don't weaken it).
- BUG-196 `close_outside` (board `menu_mouse.rs`) is the subtlest diff: a
  Zone C click must be a normal activation, not a dismiss+land.
- Mux regression: the `menu004` integration suite asserts exact ring walks —
  the empty-slice path must be byte-identical.
- No new `Action` payload beyond the single `MenuZoneCActivate(usize)`; no
  backend, transport, or RPC-type changes.

## 7. Verification plan (ACDD phases)

1. Feature file (above) → `fspec validate` + `fspec validate-tags`.
2. Failing tests: `tests/menu009_zone_c_buttons.rs` with `@step` comments
   (board/agent/mux paint, Enter+click activation, ring wrap, truncation,
   BUG-196 exclusion) + unit tests in `layout.rs`/`mod.rs` (+ proptest).
3. Implement component layer first (TDD red→green), then surface wiring,
   then dispatch; run `cargo test -p fspec-tui --test menu009_zone_c_buttons`
   and the existing `menu001`/`menu002`/`menu003`/`menu004`/`menu007`/
   `menu008`/`bug195`/`bug196` suites (regression guards).
4. `cargo clippy -p fspec-tui` (workspace denies unwrap/expect/panic) +
   `fspec validate` + link coverage.
