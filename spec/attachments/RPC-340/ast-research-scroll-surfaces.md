# RPC-340 — AST research (scroll-fix surfaces)

Structural survey (AstGrep + Grep) of the exact code surfaces the scroll fix
touches, and the parity reference to mirror.

## Reusable helper (no change)
- `components/scroll_viewport.rs` — `pub fn ensure_visible(scroll_offset: &mut usize,
  selected: usize, visible_rows: usize, total: usize)` at lines 46-66. Already
  unit-tested (scroll_viewport.rs tests 178-225). Exported via `components/mod.rs`.

## Parity reference — ProviderSettingsView (mirror this exactly)
- `views/provider_settings/mod.rs:18` — `use crate::components::scroll_viewport::ensure_visible;`
- `views/provider_settings/mod.rs:240-248` — `pub(crate) fn adjust_scroll(&mut self)`:
  ```rust
  pub(crate) fn adjust_scroll(&mut self) {
      let total = self.visible_providers().len();
      ensure_visible(&mut self.scroll_offset, self.selected_index, self.visible_rows, total);
  }
  ```
- `views/provider_settings/mod.rs:250-260` — `move_clamped` calls `self.adjust_scroll()`
  after updating `selected_index` (`:259`).

## Target file — model_selector/mod.rs (current state, all confirmed)
- Fields: `scroll_offset: usize` (`mod.rs:47`), `visible_rows: usize` (`mod.rs:53`,
  default 12 at `:76`).
- `scroll_offset` written ONLY at init (`:70`); read at `:326` (render) and inside
  `rows::render_body`/`render_scrollbar`. **Never reconciled.**
- Mutation sites that change `selected_index` / rebuild `rows` (each needs
  `adjust_scroll()` after):
  - `fn move_up(&mut self)` — `mod.rs:146-153`
  - `fn move_down(&mut self)` — `mod.rs:155-164` (AstGrep confirmed signature)
  - `Home` arm — `mod.rs:253-256`
  - `End` arm — `mod.rs:257-261`
  - `fn set_providers(&mut self, ...)` — `mod.rs:92-101`
  - `fn handle_filter_key(&mut self, ...)` Esc/Backspace/Char branches —
    `mod.rs:198-220`
  - `fn toggle_expansion(&mut self, expand: bool)` if-changed block — `mod.rs:178-193`
- `handle_mouse` (`mod.rs:294-307`) calls `move_up`/`move_down` → wiring
  `adjust_scroll()` inside those two covers keyboard AND wheel in one place.
- `render` (`mod.rs:309-332`): `self.visible_rows = body_area.height - 1` assigned
  at `:320` inside the body closure → add `self.adjust_scroll()` immediately after
  for the defensive resize/initial-draw reconcile, before `rows::render_body`.
- `visible_rows_for(area)` (`mod.rs:334-337`) uses `CHROME_ROWS` (=3, outer chrome)
  — the WRONG quantity for body windowing; do NOT use it for scroll math.

## Conclusion
Pure additive change inside `model_selector/mod.rs`: one private `adjust_scroll`
wrapper + call sites, reusing `ensure_visible`. No wire-type, Action, or
`rows.rs` changes needed; scrollbar/overflow arrows follow once `scroll_offset`
tracks the cursor.
