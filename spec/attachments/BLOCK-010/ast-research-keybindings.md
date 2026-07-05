# AST Research — BLOCK-010 keybinding parity

## Goal
Confirm the exact `handle_key` seams and the canonical Page/Home/End keybinding
set to mirror, so blocklist can drop vim `j`/`k` and gain Page/Home/End.

## `handle_key` methods across the mode-views (AstGrep: `pub fn handle_key(&mut self, key: KeyEvent) -> $RET { $$$BODY }`)

| File | Line | Signature |
|------|------|-----------|
| `views/blocklist/mod.rs` | 150 | `pub fn handle_key(&mut self, key: KeyEvent) -> BlocklistEvent` — **target** |
| `views/model_selector/dispatch.rs` | 10 | `pub fn handle_key(&mut self, key: KeyEvent) -> ModelSelectorEvent` — **reference** |
| `views/provider_settings/mod.rs` | 184 | `pub fn handle_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent` — reference |
| `views/checkpoints/keys.rs` | 18 | `pub(super) fn handle_key(...) -> CheckpointsEvent` |
| `views/model_selector/form.rs` | 125 | overlay form handler (not nav) |

## Reference keybinding arms (model_selector/dispatch.rs, verified by Read)
- `KeyCode::Up` (69), `KeyCode::Down` (73), `KeyCode::Home` (77), `KeyCode::PageDown` (82),
  `KeyCode::PageUp` (86), `KeyCode::End` (90). **No `KeyCode::Char('j')`/`Char('k')` arms exist.**
- Paging helpers `views/model_selector/navigation.rs`: `page_down` (50-63), `page_up` (65-80) —
  `step = visible_rows.max(1)`, loop the per-row clamp mover `step` times, then `adjust_scroll()`.

## Blocklist current state (mod.rs, verified by Read)
- `handle_key` (150-182) has vim arms: `'j'|'J'` (168-171) → `move_down`; `'k'|'K'` (172-175) → `move_up`.
  Space `' '` (176) → `toggle_focused`. No Page/Home/End arms (fall through to `_ => Consumed` at 180).
- Single-step movers only: `move_down` (184-189), `move_up` (191-196), both call `adjust_scroll()`.
- `adjust_scroll` (128-135) delegates to `components::scroll_viewport::ensure_visible`.
- Test seams (in-module `#[cfg(test)]`): `scroll_offset()` (110-113), `set_visible_rows()` (117-120).
- `FOOTER_HINT` (render.rs:24) = `"↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close"`.

## Conclusion
Blocklist is the sole outlier binding vim keys. Fix = remove the two vim arms, add
PageDown/PageUp/Home/End arms + `page_down`/`page_up`/`jump_top`/`jump_bottom` helpers (all
`adjust_scroll()`-reconciled, empty-list safe), and rewrite `FOOTER_HINT`. All rules are
selectable so no header-skipping (unlike model_selector). Scope: `mod.rs` + `render.rs`.
