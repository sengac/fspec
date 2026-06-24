# PROV-104 — Model View Scroll/Viewport Parity with TypeScript

**Type:** Bug. The model view does not scroll: the selected row goes off-screen and the list
never follows the cursor. This is the #1 user-reported defect ("doesn't even fucking scroll").

## Symptom

In the model view (`ModelSelectorView`), pressing Down past the bottom of the visible window
leaves the highlighted model row hidden/off-screen; the viewport "follows" the cursor 1–2 rows
short, and at long lists the selected item is never visible. Same class of failure at the top edge.

## Root Cause (Rust) — arrow glyphs overwrite content rows

The scroll math is correct in isolation but the renderer steals content rows:

- **State/primitives:** `model_selector/mod.rs:57` `selected_index`, `:63` `scroll_offset`,
  `:69`/`:97` `visible_rows` (default 12). The list is `rows` (flat headers + model rows).
- **`adjust_scroll()` `mod.rs:178-195`** → `scroll_viewport::ensure_visible(&mut scroll_offset,
  selected_index, visible_rows, rows.len())` (`components/scroll_viewport.rs:46-66`). Standard,
  correct windowing; plus a first-row special-case (`mod.rs:189-194`).
- **`visible_rows` set at render** `mod.rs:710`: `body_area.height.saturating_sub(1)`, then
  `adjust_scroll()` `:714`, then `rows::render_body(...)` `:715-722`.
- **THE BUG — `rows::render_body` `rows.rs:166-256`:**
  - `list_height = area.height - 1` (`:191`); `visible_rows = list_height` (`:210`) — agrees with
    `mod.rs:710`. ✅
  - Slice `so .. (so+visible_rows).min(total)` (`:212-224`) — applies scroll_offset correctly. ✅
  - **`:213-214, 232-249`:** when `up_arrow = so > 0`, `rel == 0` paints `↑` and `continue`s
    (overwrites that content row). When `down_arrow = so + visible_rows < total`, the LAST visible
    row paints `↓` and `continue`s. So only `visible_rows − (arrows shown)` rows ever show content,
    but `ensure_visible` assumed all `visible_rows` show content.
  - The helper's own doc (`scroll_viewport.rs:43-45`) says it deliberately does NOT account for
    arrow glyphs because popups are supposed to draw glyphs *alongside* cards — `render_body`
    violates that by drawing glyphs *instead of* a content row.

**Concrete failure:** Down to the bottom edge → `ensure_visible` sets `scroll_offset =
selected_index + 1 − visible_rows`, placing `selected_index` at the last visible slot; but that
slot is overwritten by `↓` and `continue`d → the selected model is never painted (appears
off-screen). Internal unit tests pass because they only assert `scroll_offset`/`selected_index`
arithmetic, never that the selected row is actually painted.

Secondary: nav handlers run `adjust_scroll()` against stale default `visible_rows = 12` until first
render; render re-runs `adjust_scroll`, so it self-corrects on paint — but combined with the
arrow-overpaint the corrected window still hides the selected row at edges. There is also NO
PageUp/PageDown handler (`mod.rs:616` catch-all) — TS supports paging.

## TypeScript reference (the parity baseline)

- Single flat-index space: `flatItems` = headers + expanded models (`flat-model-list.ts:24,27`).
- `useModelSelectorState.ts`: `navigateDown` (`:218-231`) `if (newIdx >= scrollOffset + visibleHeight)
  setScrollOffset(newIdx - visibleHeight + 1)`; `navigateUp` (`:233-246`) `if (newIdx < scrollOffset)
  setScrollOffset(newIdx)`; filter resets offset to 0 (`:295-305`); open resets to 0 (`:286-293`).
- `visibleHeight = height - 6` set once (`ModelSelectorScreen.tsx:82-84`).
- **Render slices full window, steals NO content row** — `ModelSelectorView.tsx:144-145`
  `flatItems.slice(scrollOffset, scrollOffset + visibleHeight)`; selected row at bottom edge
  (`:147-148`) always painted.
- **Scroll indicator is a SEPARATE column** (`ModelSelectorView.tsx:236-252`): a scrollbar thumb in
  its own `<Box marginLeft={1}>` of `visibleHeight` cells beside the list — consumes ZERO content rows.

## Fix Direction (choose for TS parity)

Preferred (true parity): render the `↑`/`↓` / scrollbar indicator in a **dedicated column** beside
the list (TS approach) and slice the full `visible_rows` so every content row is painted; the
selected row at the edge is always visible.

Acceptable alternative: subtract the actually-shown arrow rows (1–2 row gutter) from the height
passed to `ensure_visible` (and/or add a one-row scroll margin) so the selected row can never land on
an arrow slot. Also consider adding PageUp/PageDown for TS parity.

Either way, navigation must match TS: single flat-index space, reset-on-filter, reset-on-open.

## Acceptance direction

- Selecting (Down/Up/Home/End/wheel/PageDown) any model ALWAYS keeps the highlighted row painted
  and visible within the viewport — for lists longer than the viewport.
- Add a test that asserts the SELECTED row is actually rendered/visible (not just that scroll_offset
  arithmetic is right) at both top and bottom edges and mid-list.
- Behavior matches the TS navigateUp/navigateDown/reset semantics.
- Offline tests; no unwrap/expect; files <300 lines.

## Relevant files

- `codelet/fspec-tui/src/views/model_selector/mod.rs` (scroll state, adjust_scroll, key/wheel, render)
- `codelet/fspec-tui/src/views/model_selector/rows.rs` (`render_body` — primary fix, :166-256)
- `codelet/fspec-tui/src/components/scroll_viewport.rs` (`ensure_visible`, contract doc)
- TS reference: `src/tui/components/ModelSelectorView.tsx`, `src/tui/hooks/useModelSelectorState.ts`,
  `src/tui/components/ModelSelectorScreen.tsx`, `src/tui/utils/flat-model-list.ts`

## Related

- PROV-103 covers the SEPARATE provider-settings nav-tree scroll bug (move_clamped/adjust_scroll use
  `visible_providers().len()` instead of `nav_items.len()`). Same scroll-parity theme, different file.
