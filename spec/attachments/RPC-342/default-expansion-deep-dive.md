# RPC-342 — Model selector default expansion inverted (all-expanded vs all-collapsed)

**Severity: MEDIUM** — parity divergence that AMPLIFIES the RPC-340 scroll bug by
guaranteeing viewport overflow on first open.

## Summary

TS starts with `expandedProviders` = empty Set (all collapsed) and auto-expands
ONLY the section containing the current model. Rust `set_providers` expands EVERY
provider (`mod.rs:93`). The Rust default guarantees the list overflows the
viewport immediately, which makes the dead-scroll bug (RPC-340) bite on the very
first open.

This card is tightly coupled to **RPC-341** (current-model seeding) — the
collapse-default + auto-expand-only-current + cursor-seed are the single direct
port of `ModelSelectorScreen.tsx:93-119` and should ideally land together.

---

## PART 1 — TypeScript expansion behavior

- Empty-Set default: `useModelSelectorState.ts:148-150`
- `toggleSectionExpansion` (pure add/delete on cloned Set): `useModelSelectorState.ts:198-208`
- flat-list build is expansion-driven: `useModelSelectorState.ts:169-172`
  (`buildFlatModelList(providerSections, expandedProviders)` — only expanded
  providers emit model rows)
- Auto-expand ONLY the current model's section on open:
  `ModelSelectorScreen.tsx:93-119` (esp. `:102-104` `toggleSectionExpansion`)
- **Filter does NOT force-expand** (render-time narrowing of an already-flat list):
  `useModelSelectorState.ts:174-177` → `filterFlatItems(flatItems, filter)`.
  Since `flatItems` only contains rows for expanded providers, filtering CANNOT
  reveal models inside collapsed providers. (Behavioral divergence from Rust;
  out of scope for this card.)

---

## PART 2 — Rust expansion behavior

- Expand-all on load (the divergence): `mod.rs:93`
- `expanded` field/init: `mod.rs:44`, `mod.rs:67`
- `is_expanded` / `toggle_expansion`: `mod.rs:119-121`, `mod.rs:166-194`
- `build_view_rows` consumes `expanded`: `rows.rs:46-101`
  - `rows.rs:70`: `let is_expanded = filtering || expanded.contains(&provider.key);`
    → **Rust filter force-expands every surviving provider** (reveals matches even
    in collapsed providers). This is independent of the `expanded` set and stays
    correct regardless of the default — **no change needed.**
  - `rows.rs:64-67`: filter drops providers with no matching models
- `model_count` (counts selectable rows in current projection): `mod.rs:127-129`
- `title_text`: `mod.rs:131-138`

---

## PART 3 — Existing Rust tests that assert all-expanded (WILL break)

| Test | File:line | Why it breaks |
|------|-----------|---------------|
| `left_collapses_right_expands_focused_provider` | `mod.rs:470-486` | `:474` `assert!(v.is_expanded("openai"))` immediately post-load |
| `slash_enters_filter_then_typing_narrows` | `mod.rs:524-542` | `:528` `assert_eq!(v.model_count(), 3)` before any filter |
| `down_arrow_skips_provider_headers` | `mod.rs:390-408` | assumes openai models visible from start |
| `enter_with_session_emits_model_selected` | `mod.rs:412-435` | `End` → `claude-sonnet` assumes anthropic expanded |
| `enter_without_session_is_noop` | `mod.rs:439-454` | `Home` → first model assumes openai expanded |
| `title_text_reports_model_count` | `mod.rs:458-467` | `:464` expects "(3 models)" |
| `overflow_shows_indicators_and_wheel_advances...` | `mod.rs:546-600` | expects models rendered/overflowing from start |
| `r_key_emits_refresh_and_sets_refreshing` | `mod.rs:490-509` | mild; `set_providers` re-call `:507` now collapses |

`rows.rs` tests are NOT affected (they pass `expanded_set(&[...])` explicitly,
e.g. `collapsed_provider_hides_models_expanded_shows_them` `rows.rs:502-530`).

---

## PART 4 — Proposed change (start all-collapsed, expand only current section)

### 4a. Replace expand-all in `set_providers` (`mod.rs:92-101`)

```rust
pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
    // RPC-342 parity: start all-collapsed; expand ONLY the section
    // containing the current model (ModelSelectorScreen.tsx:94-119).
    self.expanded = HashSet::new();
    if let Some(current) = self.current_model_id.as_deref() {
        if let Some(p) = providers
            .iter()
            .find(|p| p.models.iter().any(|m| m.id == current))
        {
            self.expanded.insert(p.key.clone());
        }
    }
    self.providers = providers;
    self.loaded = true;
    self.is_refreshing = false;
    self.rebuild_rows();
    // (cursor-seed: see RPC-341 — index_of_model else first_selectable_or_zero)
}
```

### 4b. Make `model_count`/title count ALL models, not projected rows (`mod.rs:127-129`)

```rust
pub fn model_count(&self) -> usize {
    self.providers.iter().map(|p| p.models.len()).sum()
}
```
Matches the TS title (shows the total) and avoids a confusing "(0 models)" on
open when everything starts collapsed.

### 4c. Filter auto-expand — NO change (`rows.rs:70` keep `filtering ||`).

---

## PART 5 — Test changes required

1. `left_collapses_right_expands_focused_provider` (`mod.rs:470-486`): invert
   `:474` to `assert!(!v.is_expanded("openai"))`; restructure to
   collapsed → cursor on header → `Right` expands → `Left` collapses.
2. `slash_enters_filter_then_typing_narrows` (`mod.rs:528`): valid as `3` IF 4b
   applied (count = all models); else expect 0 and expand first.
3. `down_arrow_skips_provider_headers`, `enter_with_session_emits_model_selected`,
   `enter_without_session_is_noop`, `overflow_...`: expand the relevant section
   first. Preferred: extend `loaded_view()` (`mod.rs:378-386`) to
   `set_current_model(Some("gpt-4o"))` BEFORE `set_providers` so openai
   auto-expands and the cursor seeds. Tests asserting anthropic models must press
   `Right` on its header (or assert against openai's last model).
4. `title_text_reports_model_count` (`mod.rs:464`): passes iff 4b applied.
5. `r_key_emits_refresh_and_sets_refreshing` (`mod.rs:490-509`): assertions
   unaffected.

New tests to add:
- `set_providers_starts_all_collapsed_without_current` — no current → `!is_expanded("openai")`
- `set_providers_auto_expands_only_current_section` — `set_current_model("claude-sonnet")`
  then load → `!is_expanded("openai") && is_expanded("anthropic")`, cursor on the
  `claude-sonnet` row

`rows.rs` tests + `rpc337_navigator_model_selector.rs`: no changes.

---

## PART 6 — Dependency / interaction

- **RPC-341 (current-model seeding):** *tightly coupled.* The expand-only-current
  step needs `current_model_id` set before `set_providers` (dispatch order
  already guarantees this: `dispatch_model_selector.rs:29` before `:42-44`). The
  cursor-seed half of 4a IS the RPC-341 work — **recommend implementing
  collapse-default + auto-expand + cursor-seed as one unit**, since they are the
  same TS port.
- **RPC-340 (scroll):** becomes more important after this — the auto-expanded
  current section's cursor can start below the fold and would render off-screen
  while `scroll_offset` is never updated. RPC-340 must seed/track `scroll_offset`
  in `set_providers` and `move_*`.
