# PROV-102 — Enter on OpenAI Profile Opens Anthropic Detail View

**Type:** Bug. **Depends on:** PROV-101 (fallback removal).

## Symptom

In Provider Settings, expand the **OpenAI** provider, move the cursor to one of its **profile**
rows, and press **Enter**. The Detail view that opens shows **Anthropic's** data instead of the
selected OpenAI profile's models.

## Root Cause: index-space mismatch (NOT a literal hardcoded "anthropic")

1. Key dispatch: `ProviderSettingsView::handle_key` (`provider_settings/mod.rs:187-228`) →
   List mode → `list::handle_list_key` (`provider_settings/list.rs:28`).
2. Enter handler (`list.rs:86-136`): inspects focused NavItem via `view.focused_nav_item()`
   (`nav_tree_ops.rs:50-52` → `self.nav_items.get(self.selected_index)`). It has explicit arms
   only for `NavItemKind::Provider` (toggle expand) and `NavItemKind::ApiKey` (edit key).
3. **`NavItemKind::Profile { .. }` hits the catch-all `_ => {}` (lines ~110-114)** and falls
   through to the legacy fallback (lines ~117-135):
   ```rust
   let visible = view.visible_providers();          // == self.providers (top-level list)
   let Some(focused) = visible.get(view.selected_index) else { ... };
   let pid = focused.provider_id.clone();
   view.mode = ProviderSettingsMode::Detail { provider_id: pid, sub };
   ```
4. `view.selected_index` indexes the **flat `nav_items` tree** (incl. expanded children), but
   `visible_providers()` returns only **top-level providers**. The index is reinterpreted against
   the wrong array.

## Why Anthropic specifically

Canonical provider order is `openai (0), anthropic (1), cohere (2), ...`
(confirmed: `tests/provider_settings_canonical_order_rpc107.rs:77`). Expanding openai (nav_items[0])
injects child rows (`nav_item.rs:149-164`):
```
nav_items = [0: Provider(openai), 1: Profile(A), 2: Profile(B), ..., AddProfile]
```
Selecting the first OpenAI profile sets `selected_index = 1`. The legacy fallback does
`self.providers.get(1)` → **anthropic** (registry index 1). The wrong id is threaded from the moment
of fall-through; `detail.rs:210` then renders anthropic's `model_count`/data.

## Defects

- **Primary:** `NavItemKind::Profile` has no dedicated Enter arm; falls into legacy `_ => {}`.
- **Mechanism:** legacy path indexes `visible_providers()` with `selected_index` (a flat-tree index).
  The correct id (`"openai"`) is already on the focused NavItem (`item.provider_id`) but discarded.

## Fix Direction

In the `KeyCode::Enter` arm of `handle_list_key`, add an explicit `NavItemKind::Profile` arm that
uses the focused NavItem's own `provider_id` (and `profile_name`) to open the intended per-profile
models Detail view. Do **not** fall through to the `selected_index`-on-`visible_providers()` path.
With PROV-101, the legacy index-into-wrong-list fallback should be removed entirely (or made an
explicit error) so it can never silently mis-select.

## EXPANDED SCOPE (from PROV-103 review WARN — MUST be fixed here)

PROV-103 fixed nav so the cursor can now LAND on child rows. That makes the `list.rs` Enter (and
`d`) legacy fallthrough actively dangerous for **every** non-Provider/non-ApiKey NavItemKind, not
just Profile:

- `list.rs:117-135` (Enter) and `list.rs:137-153` (`d`): `Profile`, `AddProfile`, `OAuthLogin`,
  `OAuthStatus` all fall through to `view.visible_providers().get(view.selected_index)` where
  `selected_index` is now a `nav_items` index. → single expanded provider: `None` (silent no-op);
  multiple expanded providers: resolves to a DIFFERENT provider's Detail.

**Fix ALL of them via `focused_nav_item().kind` + `provider_id`** (TS approach,
`src/tui/inputHandlers/listModeHandler.ts:120-153`):
- `Profile` → open that profile's Detail/models view for its provider_id.
- `AddProfile` → start add-profile flow for its provider_id.
- `OAuthLogin` → start OAuth login for its provider_id.
- `OAuthStatus` → open disconnect/status for its provider_id.
- `d` on the appropriate rows → matching delete/disconnect action by NavItem identity.

Remove the `visible_providers()[selected_index]` fallthrough entirely so no mismatched-index path
remains (consistent with PROV-101 no-fallback mandate). Verify against TS for each row's action.

## Relevant files

- `codelet/fspec-tui/src/views/provider_settings/list.rs` (Enter handler — primary fix)
- `codelet/fspec-tui/src/views/provider_settings/nav_item.rs` (NavItemKind, child-row injection)
- `codelet/fspec-tui/src/views/provider_settings/nav_tree_ops.rs` (`focused_nav_item`)
- `codelet/fspec-tui/src/views/provider_settings/mod.rs` (`visible_providers`, modes)
- `codelet/fspec-tui/src/views/provider_settings/detail.rs` (Detail render)
- TS reference: `src/tui/provider-config.ts`, `src/tui/profile-management.ts`

## Acceptance direction

- Enter on an OpenAI profile row opens that OpenAI profile's Detail/models view.
- The opened Detail view's provider_id equals the focused NavItem's provider_id (never re-derived
  from selected_index against the provider list).
- Offline tests: simulate expanded nav tree, Enter on a Profile row, assert Detail provider_id.
