# RPC-105 — Header Title Count: Total Nav Items, Not Configured Count

## 1. Summary

The Rust `ProviderSettingsView::title_text()` returns
`"Provider Settings ({} configured)"` where the count is
`providers.iter().filter(|p| p.configured).count()`. This is **wrong vs the TS reference**.
In TS, the title shows `"({navItems.length} items)"` — the count of every visible row in
the flat tree, including expanded children. The header is a navigation breadcrumb
("you can move to N rows"), not a credential-state summary. This card switches the
semantics and the string template.

## 2. TypeScript Citations

| Element | File | Lines |
|---|---|---|
| Title rendered with `navItems.length` | `src/tui/components/ProviderSettingsPanel.tsx` | ~558 (the `<Text bold>` header inside the list body) |
| `navItems` count is what fluctuates with expand/collapse | `src/tui/hooks/useProviderSettingsState.ts` | 361–364 |
| `buildNavItems` is the SOLE source of `navItems` | `src/tui/hooks/useProviderSettingsState.ts` | 132–206 |
| Filter shrinks navItems → header count also shrinks | `src/tui/hooks/useProviderSettingsState.ts` | 141–147 |

## 3. Current Rust Deviation

```rust
// codelet/fspec-tui/src/views/provider_settings/mod.rs:112-118
pub fn configured_count(&self) -> usize {
    self.providers.iter().filter(|p| p.configured).count()
}

pub fn title_text(&self) -> String {
    format!("Provider Settings ({} configured)", self.configured_count())
}
```

Problems with this:
1. **Wrong noun** — "configured" implies credential state, but TS calls them "items"
   (nav rows).
2. **Wrong denominator** — even if the user expands a provider revealing 3 child rows, the
   count stays the same. TS bumps from 17 → 20.
3. **Wrong reactivity to filter** — typing `/openai` should drop the count to 1 (only the
   openai row visible); current Rust impl still reports the total configured count.
4. **Misleads first-time users** — a fresh install with no creds reads `"(0 configured)"`,
   which suggests the screen is empty. TS reads `"(17 items)"` which correctly signals
   "17 things to interact with".

## 4. Proposed Rust Change

Once RPC-103 lands its `Vec<NavItem>` (`build_nav_items`), the change is mechanical:

```rust
pub fn nav_item_count(&self) -> usize {
    self.nav_items.len()
}

pub fn title_text(&self) -> String {
    format!("Provider Settings ({} items)", self.nav_item_count())
}
```

The old `configured_count()` method can be deleted (no other caller) or retained for
debug/status text — but it must NOT appear in the title. The `render_title_with_count`
helper in `views::agent::mode_view_render` doesn't need changes; only the string fed
to it changes.

## 5. State-Transition Examples (Header Count Reactivity)

Assume canonical 17-provider registry, no creds configured yet.

| User action | TS header | Old Rust header | New Rust header (this card) |
|---|---|---|---|
| Open Provider Settings (fresh) | `(17 items)` | `(0 configured)` | `(17 items)` |
| Expand anthropic (no tokens) → +3 child rows | `(20 items)` | `(0 configured)` | `(20 items)` |
| Expand openai (0 profiles) → +1 add-profile row | `(21 items)` | `(0 configured)` | `(21 items)` |
| Configure 2 credentials, no expansions | `(17 items)` | `(2 configured)` | `(17 items)` |
| Type `/anth` filter | `(1 items)` (just anthropic row) | `(0 configured)` | `(1 items)` |
| Type `/anth` filter while anthropic expanded | `(4 items)` (anthropic + 3 children) | `(0 configured)` | `(4 items)` |

## 6. Architecture Considerations

* **Dependency on RPC-103** — this card cannot ship before RPC-103 introduces the flat
  `nav_items` vector. Until then there's no `navItems.length` to count. Mark
  RPC-105 `depends_on RPC-103`.
* **No pluralization logic required** — TS uses `"(N items)"` verbatim even for N=1.
  Don't introduce singular/plural variants; that's a deviation in its own right.
* **Singular wording match** — the literal must be `" items)"` with leading space and
  trailing `)`. The full template is `"Provider Settings ({n} items)"`. This will be
  asserted via snapshot tests in the integration suite.
* **Update existing snapshot tests** — the existing `mod.rs::title_text` unit tests and
  any rendering golden-files (search for the literal `"Provider Settings ("` in
  `codelet/fspec-tui/`) must be updated in the same commit.

## 7. Integration Test Plan

1. **`title_text_collapsed_all_returns_17_items`** — fresh view, no expansion → header
   reads `"Provider Settings (17 items)"`.
2. **`title_text_expands_one_provider_includes_child_rows`** — expand anthropic →
   `"Provider Settings (20 items)"` (assuming 2 oauth-login + 1 api-key children).
3. **`title_text_with_filter_reflects_filtered_count`** — filter="openai" →
   `"Provider Settings (1 items)"`.
4. **`title_text_with_filter_and_expansion`** — anthropic expanded, filter="anth" →
   `"Provider Settings (4 items)"`.
5. **`title_text_uses_items_not_configured`** — regression guard: assert the literal
   string does NOT contain "configured".

## 8. Out-of-Scope (Other Cards)

* Per-row icons / colors (RPC-104).
* Footer hint strings (RPC-106).
* Selection preservation across expand/collapse (new sibling card — see decomposition).
* Filter auto-expansion (new sibling card).

This card is intentionally narrow: rename + recount, nothing else.
