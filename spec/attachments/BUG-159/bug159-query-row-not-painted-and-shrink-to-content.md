# Research — BUG-148: Search dialog has no visible query input; results push it out of view

Research date: 2026-08-25. Scope: `rust/fspec-tui` dialog rendering + the
BOARD-022 `WorkUnitSearchDialog`.

## Observed symptom

When the board `/` search dialog is open, the user cannot see what they are
typing. As matches appear the dialog grows and the (nonexistent) input line
is not anchored anywhere — the user gets no feedback about the active query
or the active mode beyond the title row.

## Root cause 1 — the query text is never painted

`rust/fspec-tui/src/components/work_unit_search_dialog.rs`:

- `self.query` is only ever *read* by the filter
  (`filter_work_units(&self.units, self.mode, &self.query)` in `re_filter`,
  line 147) and passed to `build_dialog_rows` (line 263) purely to build the
  *empty-state* literal `"(no work units match \"<query>\")"`.
- `render` (line 258) builds an `FspecDialog { title, rows, footer,
  min_width }` and calls `render_dialog`. **No span anywhere carries the
  query string.** The title is
  `format!("Search Work Units [{}]", self.mode.label())` — it shows the mode
  but not the text.
- `work_unit_search_rows.rs::build_rows` renders one row per match id
  (marker + `id`) and, on overflow, `↑`/`↓` indicator rows. It never renders
  the query.

So the query is invisible by construction, not by a layout bug.

## Root cause 2 — shrink-to-content centering moves the dialog as it grows

`rust/fspec-tui/src/components/dialog_theme.rs`:

- `render_dialog` → `dialog_rect` (line 108) computes the dialog size from
  the content: `body_h = dialog.rows.len()` and
  `height = natural.min(area.height)` where
  `natural = 2 + 2 + 1 + 1 + body_h + footer_block`.
- It then **centers** the rect:
  `y = area.y + area.height.saturating_sub(height) / 2` (line 119).

Consequences:

1. The dialog is *shrink-to-content*: its height is a function of the number
   of visible rows. When the user types and the match count changes, the
   natural height changes and the whole dialog re-centers on every render.
2. There is no stable "input line" to anchor to — the top of the body is
   always the first match row, which shifts as the list grows/shrinks.

This is the same class of problem the full-screen `TurnContentModal` solved
(RPC-383) by switching from `render_dialog` (shrink-to-content) to
`render_dialog_at(fixed_dialog_rect(area), …)`.

## The existing DRY primitives to reuse (do not re-invent)

| Primitive | File | What it gives |
|-----------|------|---------------|
| `fixed_dialog_rect(area)` | `components/dialog_theme_rows.rs:193` | A **fixed** centered rect (`area.width-4` × `area.height-6`) independent of content length. The `TurnContentModal` uses it so the frame does not move as content scrolls. |
| `render_dialog_at(rect, buf, dialog)` | `components/dialog_theme.rs:145` | Paints a dialog at an **explicit caller-computed rect** instead of `dialog_rect`. Shares the exact body/title/footer painting with `render_dialog`, so the visual contract stays in one place. |
| `body_content_rows(rect_height, footer_h)` | `components/dialog_theme_rows.rs:211` | Returns the number of content rows that fit in a fixed-rect dialog, using the *same* spacious/compact fallback as `render_dialog_at`. This is the single source of truth for "how many rows does the body actually paint" — the scroll reducer and the render path must agree on this number. |
| `build_dialog(accent, title, rows, footer, min_width)` | `components/dialog_theme_rows.rs:76` | Builds an `FspecDialog` without a raw struct literal (keeps the `field_reassign_with_default` lint contained in one file). |
| `TurnContentModal` render path | `views/agent/turn_modal.rs:104` | The reference consumer: `let rect = fixed_dialog_rect(area); … render_dialog_at(rect, buf, &dialog)`. |

## Recommended design

1. **Add a query row to the dialog body.** Introduce a `query_row: Option<&str>`
   (or a dedicated `DialogRow`) on `FspecDialog` that `render_dialog_at`
   paints on the row immediately after the inner title, *before* the body
   rows, in a distinct style (e.g. `▸ <query>` with the mode label). This row
   is always present for the search dialog so the input is visible and
   anchored.
   - `dialog_rect` / `body_content_rows` must account for the extra row so
     the body viewport math stays correct (add a `+1` to the content
     reservation when `query_row` is `Some`).
2. **Switch the search dialog to a fixed rect.** In
   `WorkUnitSearchDialog::render`, replace `render_dialog(area, …)` with
   `render_dialog_at(fixed_dialog_rect(area), …)` (mirroring
   `turn_modal.rs`). Use `body_content_rows(rect.height, footer_h)` to size
   the visible-rows window instead of the ad-hoc
   `(area.height - 8).clamp(1, 20)` currently at line 259.
   - This makes the frame static; only the body rows scroll. The query row
     stays pinned under the title no matter how many matches appear.
3. **Keep the visual contract in `dialog_theme`.** The query-row painting
   lives in `render_dialog_at`, so every dialog that wants an input line
   reuses it rather than each dialog hand-painting a cursor.

## Files to change

- `components/dialog_theme.rs` — add `query_row` to `FspecDialog`; paint it
  in `render_dialog_at`; bump `dialog_rect`/natural-height math.
- `components/dialog_theme_rows.rs` — `body_content_rows` + `dialog_rect`
  must reserve the query row.
- `components/work_unit_search_dialog.rs` — `render` uses
  `fixed_dialog_rect` + `render_dialog_at` + `body_content_rows`; sets
  `query_row` to the live query.
- `components/work_unit_search_rows.rs` — unchanged (match rows only).

## Tests to add (ACDD — write first)

- A fresh dialog with a non-empty query renders the query text on its own
  row (assert the painted buffer contains the query string and a stable
  query-row y-position).
- Rendering with 1 match vs 20 matches produces the **same** dialog frame
  rect (top-left corner) — i.e. the frame no longer re-centers as the list
  grows (assert `dialog_rect`/painted top row is invariant).
- `body_content_rows` returns one less than before when a query row is
  present (the extra reserved row).
- The query row is still visible when the body is at maximum height.

## Out of scope

- Cursor column tracking within the query (the query is edited by
  append/backspace only; a trailing `▏` block cursor is enough).
- Multi-line query.
