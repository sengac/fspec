# PROV-103 — Up/Down and Enter Dead in Provider Settings Nav Tree

**Type:** Bug. **Depends on:** PROV-101 (fallback removal).

## Symptom

In the Provider Settings nav-tree (the per-profile/model list reached by expanding a provider —
distinct from the Tab-reachable full-screen ModelSelectorView), the **Up/Down** arrows do nothing
past the top-level providers, and **Enter** on profile/model rows does nothing.

## Key routing IS wired (events reach the view)

1. `Navigator::handle_event` (`navigator.rs:89-97`) → `ViewMode::ProviderSettings` →
   `handle_provider_settings_event`.
2. `navigator_events.rs:24-52` forwards the `KeyEvent` to `ProviderSettingsView::handle_key`.
3. `mod.rs:187-228` → List mode → `list::handle_list_key` (`list.rs:28`).
4. `handle_list_key` binds Up/Down (`list.rs:63-85`) and Enter (`list.rs:86-136`). Focus/events
   ARE delivered — this is not a focus bug.

## Root Cause: navigation math sized to the WRONG list

Data-model mismatch between what is **rendered** and what navigation/scroll is **bounded** by:

- **Rendered rows** = `view.nav_items` (full flat tree incl. expanded children: profiles, ApiKey,
  OAuth, AddProfile). See `list.rs:224-227` (prefers nav_items when non-empty) and
  `list_nav_render.rs:43-46` (iterates nav_items, highlights `global_idx == selected_index`).
- **Focus/selection** = `nav_items[selected_index]` via `focused_nav_item()` (`nav_tree_ops.rs:50-52`).
- **BUT navigation + scroll are bounded by the raw provider list:**
  - `move_clamped` (`mod.rs:251-261`) clamps `selected_index` to `0..=visible_providers().len()-1`.
  - `adjust_scroll` (`mod.rs:241-249`) uses `total = visible_providers().len()`.

`visible_providers()` (`mod.rs:173-185`) returns only top-level `providers` — NOT the expanded child
rows. So `selected_index` can never exceed `providers.len()-1`, even though the rendered tree has many
more rows. Result:

- **Up/Down "do nothing"** on the per-profile section: cursor is trapped in the first N rows
  (N = number of top-level providers).
- **Enter "does nothing"** on profile/model rows: `selected_index` can never land on a
  `Profile`/`AddProfile` row, so `focused_nav_item()` never returns one and the Enter branch never fires.

## Why it regressed

Commit `e31a4792` (RPC-349/054/073) changed the live `/provider` dispatch
(`dispatch_provider_settings.rs:69-80`, `handle_provider_credentials_loaded`) to call
`project_display_infos(...)` + `set_provider_display_infos(...)`, which **populates `nav_items`**.
That flipped rendering from the legacy `visible_providers` loop to the nav_items tree, but
`move_clamped`/`adjust_scroll` were left bounded by `visible_providers()`. Before the commit
nav_items was empty on the live path, so render and nav bounds matched.

## Fix Direction

`move_clamped` and `adjust_scroll` must operate against the **rendered list length** —
`self.nav_items.len()` — with a legacy fallback to `visible_providers().len()` ONLY when
`nav_items` is empty. Ideally also skip non-selectable rows (the way
`ModelSelectorView::move_up/move_down` skip headers); minimal correctness fix is bounding by
`nav_items.len()`.

> Note: per PROV-101, the legacy `visible_providers()` fallback for selection (in the Enter path)
> is being removed. Keep nav bounds tied to what is actually rendered (nav_items); avoid introducing
> a new silent selection fallback.

## Relevant files

- `codelet/fspec-tui/src/views/provider_settings/mod.rs` (`move_clamped` ~251-261, `adjust_scroll`
  ~241-249, `visible_providers` ~173-185) — primary fix
- `codelet/fspec-tui/src/views/provider_settings/list.rs` (Up/Down ~63-85, Enter ~86-136, render
  ~224-227)
- `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs` (~43-46)
- `codelet/fspec-tui/src/views/provider_settings/nav_tree_ops.rs` (`focused_nav_item` ~50-52)
- `codelet/fspec-tui/src/app/dispatch_provider_settings.rs` (~69-80, regression origin)
- Reference for skip-non-selectable: `codelet/fspec-tui/src/views/model_selector/mod.rs`
  (`move_up`/`move_down`)

## Acceptance direction

- With an expanded provider, Up/Down can move the cursor onto every rendered nav_items row
  (providers, profiles, ApiKey, AddProfile), skipping non-selectable rows.
- Enter fires on profile/AddProfile rows.
- Scroll keeps the highlighted row visible across the full nav_items length.
- Offline tests: build an expanded nav tree, drive Up/Down, assert selected_index reaches child rows.
