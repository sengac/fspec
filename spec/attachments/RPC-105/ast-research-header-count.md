# RPC-105 — AST Research: header nav-item count

## Source-shape probes (rust)

### Public methods on `ProviderSettingsView` (in `mod.rs`)
- `focused_provider(&self) -> Option<&ProviderCredentialInfo>` (line 109)
- `visible_rows(&self) -> usize` (line 121)
- `configured_count(&self) -> usize` (line 125) — **TO BE REMOVED**
- `title_text(&self) -> String` (line 129) — **TO BE REWRITTEN to use `nav_items.len()` and "items" suffix**
- `footer_hint(&self) -> &'static str` (line 133)
- `visible_provider_ids(&self) -> Vec<String>` (line 144)
- `visible_providers(&self) -> Vec<&ProviderCredentialInfo>` (line 151)

### Existing call sites of `configured_count`
```
codelet/fspec-tui/src/views/provider_settings/mod.rs:126   self.providers.iter().filter(|p| p.configured).count()      // method body
codelet/fspec-tui/src/views/provider_settings/mod.rs:130   format!("Provider Settings ({} configured)", self.configured_count())   // title_text body
codelet/fspec-tui/src/views/provider_settings/mod.rs:252   self.configured_count(),                                  // render() argument
```
**Conclusion**: no callers outside `mod.rs`. Safe to delete after rewriting the two usages.

### Existing call sites of `title_text`
```
codelet/fspec-tui/tests/provider_settings_view_rpc054.rs:462   let title = view.title_text();
```
Single test asserts the old "(2 configured)" literal — must be updated.

### Existing literal "Provider Settings ("
- `mod.rs:130` (title_text format string)
- `tests/provider_settings_view_rpc054.rs:463` (assertion comment)
- `tests/provider_settings_view_rpc054.rs:466` (assertion message)

### `render_title_with_count` helper (in `agent/mode_view_render.rs`)
```rust
pub(crate) fn render_title_with_count(area, buf, title: &str, count: usize, suffix: &str)
```
Already parameterised on suffix — we change `"configured"` → `"items"` and the count source `self.configured_count()` → `self.nav_items.len()`.

### Filter-mutation sites in `list.rs` (need `rebuild_nav_items()` calls for reactivity)
```
list.rs:41   view.filter.clear();      // Esc in list mode
list.rs:140  view.filter.clear();      // Esc in filter mode
list.rs:151  view.filter.pop();        // Backspace in filter mode
list.rs:157  view.filter.push(c);      // typing a char in filter mode
mod.rs:209   self.filter.clear();      // reload() resets filter
```
All five sites must follow up with `rebuild_nav_items()` so the title count tracks the filter (Rule 4).

## Plan summary
1. Rewrite `title_text()` to `format!("Provider Settings ({} items)", self.nav_items.len())`.
2. Delete `configured_count()` (no callers after step 1 + step 3).
3. Rewrite `render()` line 248-254 to pass `self.nav_items.len()` and `"items"`.
4. Add `self.rebuild_nav_items()` after every filter mutation in `list.rs` (4 sites) and `mod.rs:209`.
5. Update the existing rpc054 title test to assert the new `"(N items)"` shape.
