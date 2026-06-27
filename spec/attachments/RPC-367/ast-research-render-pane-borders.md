# AST Research — RPC-367 Pane Border/Divider Parity

Tool: AstGrep (rust). Scope: `codelet/fspec-tui/src/views/`.

## Render entry points and pane-header helper

Pattern: `fn pane_header($$$ARGS) -> Rect { $$$BODY }`

- `codelet/fspec-tui/src/views/checkpoints/render.rs:104` — `fn pane_header(area, buf, label, focused) -> Rect`
- `codelet/fspec-tui/src/views/changed_files/render.rs:80` — `fn pane_header(area, buf, label, focused) -> Rect`

**Finding:** `pane_header` is **duplicated** between the two views (identical signature + body: a `Layout` vertical split `[Length(1), Min(0)]`, green band when focused, returns the content Rect below the 1-row header). This is the natural place to add a heading-underline rule, and the duplication should be lifted into `diff_common` (DRY — Rule 6).

## Shared scrollbar helper (model for the new shared border helper)

Pattern: `pub fn render_pane_scrollbar($$$ARGS) { $$$BODY }`

- `codelet/fspec-tui/src/views/diff_common/mod.rs:26` — already the shared gutter-painter that reserves a 1-col Rect. The new vertical-divider helper should follow the same shape (take a `content` Rect + reserved column) and live alongside it.

## Layout split sites (where divider columns/rows must be reserved)

Pattern: `Layout::default()`

- `checkpoints/render.rs:57` — vertical `[40%, 60%]` rows (top row vs diff)
- `checkpoints/render.rs:61` — **horizontal `[40%, 60%]` top row (Checkpoints | Files)** ← needs vertical divider column
- `checkpoints/render.rs:108` — `pane_header` vertical split
- `changed_files/render.rs:52` — **horizontal `[40%, 60%]` (Files | Diff)** ← needs vertical divider column
- `changed_files/render.rs:84` — `pane_header` vertical split

## Confirmation: no existing borders

Grep for `borders|Borders|Block::|border_style|BorderType` in both `views/changed_files/` and `views/checkpoints/` → **zero matches**. The Rust port draws no inter-pane borders at all.

## Implementation targets (derived)
1. Lift `pane_header` into `diff_common` and extend it to paint a 1-row `─` underline beneath the heading; both render.rs files call the shared version.
2. Add `render_vertical_divider(content_or_gutter_rect, buf)` to `diff_common` painting `│` in a reserved 1-col gutter, default colour.
3. At each horizontal `Layout::split` site, reserve a 1-col gutter between panes (e.g. `[Percentage(40), Length(1), Percentage(60)]` or carve a column) and pass it to the divider helper.
4. Recompute cached `last_*_rect` content rects so mouse-wheel `pane_at` hit-testing and `page_step` math stay correct after reserving divider space.
