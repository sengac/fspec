# MODEL-008 — Model count in title renders in flat style instead of dim two-span style

## Summary
In the Rust TUI, the `/provider` view renders its title with a two-span style: a
**bold-yellow name** followed by a **dim DarkGray `(N items)`** count. The `/model` view
instead builds a single flat pre-formatted string `"Select Model (N models)"` and renders
it whole, so the count is drawn in the **same style as the title text** rather than the
dim-gray count style. This is a visual parity regression between the two sibling views.

## Evidence (root cause)

### Provider view — correct two-span style
`codelet/fspec-tui/src/views/agent/mode_view_render.rs:41-57`:
```rust
pub(crate) fn render_two_span_title(area, buf, name: &str, count: usize, suffix: &str) {
    let name_style  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let count_style = Style::default().fg(Color::DarkGray);
    let line = Line::from(vec![
        Span::styled(name.to_string(), name_style),
        Span::styled(format!(" ({count} {suffix})"), count_style),
    ]);
    Paragraph::new(line).render(area, buf);
}
```
Wired into the provider view via `render_full_screen_scaffold_with_title`
(`provider_settings/mod.rs:271-282`) → `render_two_span_title(..., "Provider Settings", count, "items")`.

### Model view — flat single-string title
- `codelet/fspec-tui/src/views/model_selector/state.rs:171-182` — builds a single flat
  string `"Select Model (N models)"` (`title_text()`).
- `codelet/fspec-tui/src/views/model_selector/render.rs:79-82` — renders it via
  `render_full_screen_scaffold_raw_title(&title, ...)`, which treats the whole string as
  one styled run. The count therefore inherits the title text style — no DarkGray.

## Fix direction
Switch the model view's browse-list title from the "raw title" scaffold variant to the
two-span variant so the count renders dim DarkGray, matching the provider view:

Option 1 (preferred): use `render_full_screen_scaffold_with_title` (or the equivalent
scaffold entry that accepts name/count/suffix and internally calls
`render_two_span_title`), passing name `"Select Model"`, `count = <model count>`,
suffix `"models"`.

Option 2: if the scaffold entry point is not readily available for the model view, render
the title area directly with `render_two_span_title(area, buf, "Select Model", count, "models")`.

Ensure this change applies ONLY to the browse-list title. The custom-model overlay titles
("Add / Edit / Delete Custom Model", render.rs:18-75) must remain unchanged, and the
shared blue-bold title used by ResumeSession / SearchHistory must not be affected
(RPC-350 R5 guard noted in mode_view_render.rs).

## Acceptance criteria (for Example Mapping)
- **Rule:** The model browse-list title renders "Select Model" in bold-yellow and the
  count "(N models)" in dim DarkGray, matching the provider view's two-span style.
- **Rule:** The count value (N) equals the number of models currently listed.
- **Rule:** Custom-model overlay titles (Add/Edit/Delete) are unaffected.
- **Rule:** The shared blue-bold title used by other views (ResumeSession/SearchHistory)
  is unaffected.
- **Example:** 12 models available → title shows bold "Select Model" + dim " (12 models)".
- **Example:** Opening the Add Custom Model overlay still shows "Add Custom Model" in its
  existing overlay style.

## Test strategy
`TestBackend` render harness: render the ModelSelector browse list, read the title row's
cells, and assert the span styles: "Select Model" cells carry `Modifier::BOLD` + yellow fg;
the "(N models)" cells carry `Color::DarkGray`. Assert the model count matches. Add a guard
assertion that the Add/Edit overlay title path is untouched.

## Files
- Fix: `codelet/fspec-tui/src/views/model_selector/render.rs`,
  `codelet/fspec-tui/src/views/model_selector/state.rs` (title construction)
- Reference: `codelet/fspec-tui/src/views/agent/mode_view_render.rs:41-57`,
  `codelet/fspec-tui/src/views/provider_settings/mod.rs:271-282`
