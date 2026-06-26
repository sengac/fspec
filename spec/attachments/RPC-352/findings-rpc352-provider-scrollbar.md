# RPC-352 — `/provider` list-mode scrollbar parity

**Author:** Supervisor (self-investigated via DeepSearch + direct reads)
**Scope:** Render-only — add a proportional scrollbar to the `/provider` List view.
**Goal:** Match (a) the TS reference and (b) the already-correct Rust `/model` view.

---

## 1. Authoritative source references

### TS reference — `src/tui/components/ProviderSettingsPanel.tsx`
- Lines **773–794**: renders a scrollbar column ONLY when `navItems.length > visibleHeight`:
  ```tsx
  {navItems.length > visibleHeight && (
    <Box flexDirection="column" marginLeft={1}>
      {Array.from({ length: visibleHeight }).map((_, i) => {
        const thumbHeight = Math.max(1, Math.floor((visibleHeight / navItems.length) * visibleHeight));
        const thumbPos    = Math.floor((scrollOffset / navItems.length) * visibleHeight);
        const isThumb = i >= thumbPos && i < thumbPos + thumbHeight;
        return <Text key={i} dimColor>{isThumb ? '■' : '│'}</Text>;
      })}
    </Box>
  )}
  ```
  Thumb glyph `■`, track glyph `│`, both `dimColor`. Window slice at `:569–570`
  → `navItems.slice(scrollOffset, scrollOffset + visibleHeight)`.

### Parity-correct Rust reference — `/model` (DO NOT MODIFY — reference only)
- **File:** `codelet/fspec-tui/src/views/model_selector/rows_render.rs`
- `render_body` (`:30–130`):
  - `:92` → `let overflow = total > visible_rows;`
  - `:93–97` → when overflowing, reserve a column: `list_width = area.width - 1;`
  - `:116–129` → if `overflow`, call `render_scrollbar(...)` into the column at
    `x = area.x + list_width`.
- `render_scrollbar` (`:178–206`):
  - `thumb_h = ((visible * h) / total).max(1)` (`:189`)
  - `thumb_pos = (scroll_offset * h) / total` (`:190`)
  - Loop `0..h`, paint `■` when `i >= thumb_pos && i < thumb_pos + thumb_h`, else `│`,
    all `Modifier::DIM`.

---

## 2. The defect in the Rust `/provider` port

- **File:** `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs`
- **Function:** `render_nav_items` (`:25–94`)
  - `:26` → `let visible_rows = body_area.height;`
  - `:44` → `let end = (view.scroll_offset + visible_rows).min(nav_items.len());`
  - `:45–93` → paints the windowed slice `nav_items[view.scroll_offset..end]` at the
    **full** `body_area.width` (`:52`).
  - **NO `overflow` check, NO reserved column, NO `render_scrollbar` call.**
- A grep for `scrollbar` across `provider_settings/` returns **zero** matches.

### Shared scroll-state model (already identical to `/model`)
- `scroll_offset` field: `provider_settings/mod.rs:73`, init 0 at `mod.rs:116`.
- `visible_rows` field: `mod.rs:105`, default 18 at `mod.rs:130`; set each render in
  `body_render.rs:22` to `body_area.height`.
- `ProviderSettingsView::adjust_scroll` (`mod.rs:241–249`) uses the SAME shared
  `crate::components::scroll_viewport::ensure_visible(...)` helper `/model` uses.

So the scroll *state* is already correct — only the visual indicator is missing.

---

## 3. Required behaviour (acceptance-criteria seeds)

1. When the nav-item list **overflows** the viewport (`nav_items.len() > visible_rows`),
   the `/provider` List view renders a **1-cell-wide scrollbar column** on the right
   edge of the body area.
2. The scrollbar uses a **proportional `■` thumb** over a `│` track, both **DIM**,
   matching `/model`'s `render_scrollbar` math exactly:
   `thumb_h = ((visible_rows * h) / total).max(1)`, `thumb_pos = (scroll_offset * h) / total`.
3. When the list **fits** the viewport (no overflow), **no scrollbar** is drawn and the
   list keeps the full body width (unchanged from today).
4. When the scrollbar IS drawn, the list content width shrinks by 1 column
   (`list_width = body_area.width - 1`) so rows never paint under the scrollbar.
5. The thumb position tracks `scroll_offset` so paging/scrolling visibly moves the thumb.

---

## 4. Constraints / notes for implementer

- **Render-only change.** Do NOT alter scroll-state logic (`adjust_scroll`,
  `ensure_visible`, `scroll_offset` updates) — it already matches `/model`.
- **DO NOT modify the `/model` view** — it is the reference.
- **Reuse, don't duplicate:** strongly prefer extracting/calling a shared scrollbar
  painter. The `/model` `render_scrollbar` is currently private to `rows_render.rs`;
  the cleanest option is to lift a small `render_list_scrollbar(area, buf, scroll_offset,
  visible, total)` helper into a shared module (e.g. `components/scroll_viewport.rs` or a
  new `components/list_scrollbar.rs`) and have BOTH `/model` and `/provider` call it.
  If you do this, update `/model` to call the shared helper too — but keep its rendered
  output byte-identical (existing `/model` snapshot tests must still pass).
- **300-LoC ceiling:** `list_nav_render.rs` is ~94 lines today; the older duplicate
  windowed loop in `list.rs:230–236` exists too — confirm which path actually renders in
  List mode (`body_render.rs:24` calls `list::render_list`) and add the scrollbar to the
  live path. Keep every touched file < 300 LoC.
- **Verify:** `cargo test -p codelet-fspec-tui --lib` green; no `/model` snapshot
  regression; add a new snapshot/unit test proving the `■`/`│` column appears on overflow
  and is absent when the list fits.

---

## 5. Verified line references (captured at investigation time)

```
TS  ProviderSettingsPanel.tsx:773-794   scrollbar column (■/│), gated on navItems.length > visibleHeight
TS  ProviderSettingsPanel.tsx:569-570   slice(scrollOffset, scrollOffset + visibleHeight)
Rust model rows_render.rs:92            let overflow = total > visible_rows;
Rust model rows_render.rs:93-97         list_width = area.width - 1 (reserve column)
Rust model rows_render.rs:116-129       if overflow { render_scrollbar(...) }
Rust model rows_render.rs:178-206       render_scrollbar (thumb ■ / track │, DIM)
Rust prov  list_nav_render.rs:25-94     render_nav_items — NO scrollbar, full-width
Rust prov  body_render.rs:22,24         visible_rows = body_area.height; list::render_list(...)
Rust prov  mod.rs:73,116,241-249        scroll_offset field/init + adjust_scroll (ensure_visible)
```
