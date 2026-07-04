# MODEL-007 — Model selector view never renders the filter input row

## Summary
The Rust TUI `/model` (ModelSelector) view supports filtering *internally* (state,
key handling, and row filtering all work), but the **filter prompt is never drawn**.
The user types a filter and the list narrows, yet there is no visible "Filter: …_"
line telling them what they typed. The sibling **Provider Settings** view renders
this row correctly, so this is a parity regression.

## Evidence (root cause)

### Filter behaviour DOES exist
- `codelet/fspec-tui/src/views/model_selector/crud.rs` — filter state mutation on key input.
- `codelet/fspec-tui/src/views/model_selector/rows.rs:52-73` — rows are filtered by the
  active filter string when built.

### But the filter row is NEVER rendered
- `codelet/fspec-tui/src/views/model_selector/render.rs:79-99` — the browse-list body
  closure passes the **full** `body_area` straight into `rows::render_body`. No filter
  row is carved off the top. `self.visible_rows` is computed from the full body height
  (line 85) with no reservation for a filter line.

### Reference implementation (provider view — correct)
`codelet/fspec-tui/src/views/provider_settings/list.rs:200-223`:
```rust
pub(super) fn render_list(view: &ProviderSettingsView, area: Rect, buf: &mut Buffer) {
    let visible = view.visible_providers();
    let mut body_area = area;
    if (view.filter_mode || !view.filter.is_empty()) && area.height > 0 {
        let filter_row = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
        let prompt = if view.filter_mode {
            format!("Filter: {}_", view.filter)   // active: trailing cursor underscore
        } else {
            format!("Filter: {}", view.filter)     // committed: no cursor
        };
        Paragraph::new(prompt).render(filter_row, buf);
        body_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: area.height - 1 };
    }
    // ... render rows into body_area ...
}
```

## Fix direction
In `model_selector/render.rs`, inside the browse-list body closure (around lines 84-90),
**before** `self.visible_rows` is computed (line 85) and **before** `rows::render_body`:

1. When `filter_mode` is active OR `filter` is non-empty AND `body_area.height > 0`:
   - Carve a 1-line filter `Rect` off the **top** of `body_area`.
   - Render `Paragraph::new("Filter: {}_")` (with trailing `_` cursor only while
     `filter_mode` is active; without it once committed) into that row.
   - Shrink `body_area` by 1 row from the top (`y += 1`, `height -= 1`).
2. Compute `self.visible_rows` from the **reduced** `body_area.height` so scroll math
   accounts for the filter line.
3. Call `rows::render_body` with the reduced `body_area`.

Match the exact wording/format used by the provider view (`"Filter: {}_"` /
`"Filter: {}"`) for cross-view consistency.

## Acceptance criteria (for Example Mapping)
- **Rule:** When the model selector filter is active, a "Filter: <text>_" prompt row is
  rendered at the top of the body, with a trailing cursor underscore while typing.
- **Rule:** When a filter is committed (not actively editing) but non-empty, the prompt
  renders "Filter: <text>" without the trailing cursor.
- **Rule:** When there is no filter (empty and not in filter mode), no filter row is
  rendered and the full body height is used for rows.
- **Rule:** `visible_rows` / scroll math must reserve the filter line so no model row is
  hidden behind the prompt.
- **Example:** User opens /model, presses filter key, types "opus" → top row shows
  "Filter: opus_" and the list shows only matching models.
- **Example:** User commits the filter → row shows "Filter: opus" (no underscore).
- **Example:** No filter active → no filter row, list starts at the top of the body.

## Test strategy
Use the existing `TestBackend` render-harness pattern (as used across the fspec-tui
tests). Render the ModelSelector into a fixed-size buffer, set a filter, and assert the
top body row contains `Filter: <text>_`. Assert absence when no filter is active. Assert
`visible_rows` decreases by 1 when the filter row is present.

## Files
- Fix: `codelet/fspec-tui/src/views/model_selector/render.rs`
- Reference: `codelet/fspec-tui/src/views/provider_settings/list.rs:200-223`
- Related: `codelet/fspec-tui/src/views/model_selector/rows.rs`,
  `codelet/fspec-tui/src/views/model_selector/crud.rs`
