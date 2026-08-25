# Research — BUG-160: Search dialog rows show no title/description snippet

Research date: 2026-08-25. Scope: `rust/fspec-tui` dialog row builders.

## Observed symptom

Every result row in the board `/` search dialog shows only the work-unit id
(e.g. `AUTH-001`). While typing, the user cannot tell *which* unit a match
is — they must remember which id maps to which title. The user needs a
snippet of the title/description text next to the id.

## Current rendering

`rust/fspec-tui/src/components/work_unit_search_rows.rs::build_rows`
(lines 72–79):

```rust
let id = &matches[abs_i];
let is_sel = abs_i == selected_index;
let marker = if is_sel { MARKER_SELECTED } else { MARKER_UNSELECTED };
out.push(DialogRow {
    spans: vec![Span::raw(marker.to_string()), Span::raw(id.clone())],
    selectable: true,
    selected: is_sel,
});
```

Each row is exactly `[marker][id]`. The dialog only holds `matches:
Vec<String>` (unit **ids**) — see
`work_unit_search_dialog.rs::filter_work_units` which returns
`.map(|u| u.id.clone())`. The title/description text is dropped at filter
time, so the row builder has no snippet to show.

## Data already available

`WorkUnitInfo` (rpc-types/src/lib.rs:37) carries `id`, `title`,
`description: Option<String>`, `status`. The dialog already holds the full
snapshot `units: Vec<WorkUnitInfo>` (line 93). So the snippet text is
available — it is simply never threaded into `build_rows`.

## The existing DRY primitives to reuse

| Primitive | File | Relevance |
|-----------|------|-----------|
| `label_description_row(label, description, selected)` | `components/dialog_theme_rows.rs:96` | Builds a `DialogRow` of `[marker][label] - [dimmed description]`. **This is exactly the id + snippet shape the user wants.** The description span is `Modifier::DIM` when not selected. Reuse this instead of hand-rolling spans. |
| `truncate_to(s, max_chars)` | `views/board/details_strip.rs:229` | Truncates a string to a width with a trailing `…` (uses `unicode_width`). The snippet must be width-bounded so long titles don't blow up the dialog width. |
| `truncate_path(path, max_width)` | `views/diff_common/row.rs:33` | Same end-ellipsis truncation, in the shared `diff_common` module. |

`label_description_row` is the canonical "marker + label + dimmed
description" row already used by `ThinkingLevelDialog` and
`ModelSelectorDialog`. It is the single source of truth for that visual
contract — the search dialog should consume it, not re-implement the spans.

## Recommended design

1. **Thread the snippet into the row builder.** Change
   `filter_work_units` (or add a sibling) to return a richer match shape,
   e.g. `struct SearchMatch { id: String, snippet: String }`, where
   `snippet` is the title (Id/Title mode) or the description (Description
   mode, or the title as a fallback when the unit has no description).
   - Keep `matches: Vec<String>` of ids for the existing selection/scroll
     logic, and add a parallel `Vec<String>` of snippets (or store
     `Vec<SearchMatch>`). The selection index / `ensure_visible` math is
     unchanged — it operates on list length, not element shape.
2. **Build each row via `label_description_row`.**
   `label_description_row(&id, &snippet, is_sel)` yields
   `[marker][id] - [dimmed snippet]`. This reuses the canonical row shape.
3. **Width-bound the snippet.** The dialog is shrink-to-content (see
   BUG-159). A long title would widen the frame. Truncate the snippet to a
   fixed budget (e.g. ~40 chars, or the dialog's `min_width` minus the id
   width) using `truncate_to`. Once BUG-159 switches the dialog to a
   fixed-rect render, the budget becomes the fixed body width — recompute
   the budget from `body_content_rows`/inner width so the snippet fills the
   available row without overflowing.
4. **Mode-aware snippet.** In Description mode show the description; in
   Id/Title mode show the title. This matches the field the user is
   filtering on, so the snippet reinforces the match.

## Files to change

- `components/work_unit_search_dialog.rs` — `filter_work_units` returns
  id + snippet; dialog stores the snippet list.
- `components/work_unit_search_rows.rs` — `build_rows` takes the snippet
  list and builds rows via `label_description_row` + `truncate_to`.
- `components/dialog_theme_rows.rs` — no change (reuse
  `label_description_row` as-is).

## Tests to add (ACDD — write first)

- A row for a unit with a long title is truncated with a trailing `…` and
  does not exceed the row width budget (assert the painted row width).
- In Title mode the snippet equals the (truncated) title; in Description
  mode it equals the (truncated) description.
- A unit with no description in Description mode falls back to the title
  (or shows a dimmed placeholder) — never an empty snippet.
- The selected row's snippet is not dimmed; unselected rows' snippets are
  `Modifier::DIM` (assert via `label_description_row` semantics).
- The id is always shown first, followed by the snippet (assert row prefix).

## Out of scope

- Highlighting the matched substring within the snippet (a later
  enhancement; the dimmed snippet already disambiguates).
- Multi-line snippet / description preview.
