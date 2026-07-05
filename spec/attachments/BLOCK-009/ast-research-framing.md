# AST research — BlocklistView framing/chrome parity (BLOCK-009)

## Shared scaffold (reuse)

`codelet/fspec-tui/src/views/full_screen_shell.rs`:
- `render_full_screen_scaffold(area, buf, title, count, suffix, footer_hint, body_fn, overlay)`
  → title row renders `"{title} ({count} {suffix})"`. Passing title="Blocklist Rules",
  count=N, suffix="rules" yields `Blocklist Rules (N rules)` (matches TS reference).
- `render_full_screen_scaffold_raw_title(area, buf, title, footer_hint, body_fn, overlay)`
  → verbatim title.
- Reserves CHROME_ROWS=3 (title + separator + footer); body height = area.height - 3.

Used by: `changed_files/render.rs` (render_full_screen_scaffold_raw_title),
`model_selector/render.rs`, `provider_settings`, `resume_session`, `search_history`.

## Shared divider (reuse)

`codelet/fspec-tui/src/views/diff_common/pane.rs:75`
`pub fn render_vertical_divider(gutter: Rect, buf: &mut Buffer)` — re-exported at
`diff_common/mod.rs:23`. `changed_files/render.rs` splits body into
`[Percentage(40), Length(1), Percentage(60)]` and paints the divider in the
`Length(1)` gutter.

## Footer helper

`crate::views::agent::mode_view_render::render_footer_hint` (invoked internally by
the scaffold). The BlocklistView just supplies the hint string.

## Current blocklist chrome (to replace)

`codelet/fspec-tui/src/views/blocklist/render.rs::render`:
- Hand-rolls `Block::default().borders(Borders::ALL).title(" Blocklist ")`.
- Splits inner into `[Min(1), Length(1)]` (body + footer) — its own footer.
- Body split `[Percentage(50), Percentage(50)]` with NO divider gutter.
- Footer string: ` j/k: navigate  |  Enter/Space: toggle  |  Esc: back `.

## Plan

1. Replace the `Block`/manual footer with `render_full_screen_scaffold`
   (count-title variant: title="Blocklist Rules", count=rules.len(), suffix="rules"),
   footer=`↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close`, overlay None.
2. Inside the body closure: keep the empty-state branch; for the populated branch
   split body into `[Percentage(50), Length(1), Percentage(50)]`, paint
   `render_vertical_divider` in the middle gutter, then render left/right panes as
   today (windowed left pane + scrollbar from BLOCK-008 stays).
3. The `Showing X-Y of N` indicator from BLOCK-008 stays inside the left pane.
4. Do NOT touch handle_key/set_rules/adjust_scroll/derive_category/session-disabled.
