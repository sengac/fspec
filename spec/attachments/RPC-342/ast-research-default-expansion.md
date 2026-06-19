# RPC-342 — AST research: default expansion (collapse-by-default)

AST analysis of `codelet/fspec-tui/src/views/model_selector/` confirming the
exact change sites for the collapse-by-default parity fix.

## Change site 1 — `set_providers` (the divergence)

`AstGrep rust 'pub fn set_providers(&mut self, $$$ARGS) { $$$BODY }'`
→ `mod.rs:92` `pub fn set_providers(&mut self, providers: Vec<ProviderInfo>)`

Current body expands every provider:
```rust
self.expanded = providers.iter().map(|p| p.key.clone()).collect();
```
RPC-341 cursor-seeding already lives in this fn (`rows::index_of_model` →
`selected_index`, else first-selectable fallback, then `adjust_scroll`). The fix
replaces the expand-all line with collapse-default + auto-expand-only-current,
leaving the RPC-341 cursor-seed and RPC-340 `adjust_scroll` intact.

## Change site 2 — `model_count`

`AstGrep rust 'pub fn model_count(&self) -> usize { $$$BODY }'`
→ `mod.rs:161` currently `self.rows.iter().filter(|r| r.selectable).count()`
(counts projected/visible rows). With sections collapsed on open this would read
`(0 models)`. Change to sum all providers' models so the title shows the true total.

## Change site 3 (NONE) — filter force-expand

`rows.rs:46` `build_view_rows`; `rows.rs:70`:
```rust
let is_expanded = filtering || expanded.contains(&provider.key);
```
Filtering force-expands every surviving provider independent of the `expanded`
set (`rows.rs:64-67` drops non-matching providers first). This is correct
regardless of the collapse default — **no change needed**. Confirms rule 5.

## Test impact (mod.rs `#[cfg(test)]`)

`loaded_view()` (mod.rs:424) sets a session but NO current model → after the fix
all providers start collapsed, so tests asserting `is_expanded("openai")` or
visible openai models from load will need to expand first or seed a current model.
Affected: `left_collapses_right_expands_focused_provider`,
`slash_enters_filter_then_typing_narrows`, `down_arrow_skips_provider_headers`,
`enter_with_session_emits_model_selected`, `enter_without_session_is_noop`,
`title_text_reports_model_count`, `overflow_...`, `r_key_...` (re-load collapses).
`rows.rs` tests pass `expanded_set` explicitly → unaffected.
