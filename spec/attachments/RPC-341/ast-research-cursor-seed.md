# AST Research — RPC-341 cursor-seed on current model

Tool: Grep/Read structural search over `codelet/fspec-tui/src`.

## ModelSelectorView state (views/model_selector/mod.rs:40-78)
- Fields: `current_model_id: Option<String>` (:50), `selected_index: usize` (:46),
  `scroll_offset: usize` (:47), `rows: Vec<ModelSelectorRow>` (:45), `visible_rows` (:53).

## Setters
- `set_current_model(&mut self, model_id: Option<String>)` (:84-86) — pure setter,
  stores id only (used today only for the green `(current)` marker).
- `set_providers(&mut self, providers)` (:92-102):
  - expands ALL providers (:93), stores providers, `loaded=true`, `is_refreshing=false`
  - `rebuild_rows()` (:97)
  - cursor fallback: if `selected_index >= rows.len() || !row_is_selectable(idx)` →
    `first_selectable_or_zero` (:98-100) — NEVER consults `current_model_id`.
  - `adjust_scroll()` (:101) — RPC-340 helper keeps cursor visible.

## Dispatch order (app/dispatch_model_selector.rs)
- `:29 view.set_current_model(current_model)` runs BEFORE
- `:43 set_providers(providers)` (via async ListProvidersLoaded).
- => seeding can happen synchronously inside `set_providers`; no `hasAutoExpanded` latch needed.

## Row helpers
- `first_selectable_or_zero(rows)` rows.rs:127-129 — `rows.iter().position(|r| r.selectable).unwrap_or(0)`.
- `ModelSelectorRow` (components/model_selector_dialog_rows.rs:33-37): `selectable: bool`,
  `provider_key: String`, `model_id: String`. Header rows have `selectable=false`,
  empty `model_id`; model rows `selectable=true`, `model_id=model.id`.
- No existing `index_of_model` — needs to be added.

## adjust_scroll (RPC-340) — mod.rs:114-130
- Reuses `scroll_viewport::ensure_visible(&mut scroll_offset, selected_index, visible_rows, rows.len())`.
- Already called at end of `set_providers`, so seeding `selected_index` BEFORE
  `adjust_scroll()` automatically scrolls the seeded row into view (satisfies RPC-340 interaction rule).

## Plan (matches deep-dive attachment)
1. Add `rows::index_of_model(rows, current_model_id: Option<&str>) -> Option<usize>`
   with a `r.selectable` guard.
2. In `set_providers`, after `rebuild_rows()`: if `index_of_model` Some → set
   `selected_index`; else keep existing validate-or-first-selectable fallback. Then `adjust_scroll()`.
3. Keep expand-all (don't break mod.rs is_expanded("openai") test).
