# Provider Settings / Model Selector audit: races, rendering quirks, state model

Read-only audit of `~/projects/fspec` at HEAD (PROV-176 `850a0a17d` included). Nothing was edited, built or run.
Paths are relative to `rust/fspec-tui/src/` unless they start with `rust/`.

Findings: 23, in four cards.

| Card | Findings | Theme |
|---|---|---|
| A. Provider Settings async ordering and state | F1-F7 (7) | no generations on loads, two-stage refresh, focus identity, OAuth generation reset, one shared status string |
| B. Model selector async ordering and state | F8-F14 (7) | view never reset, every load resets expansion and cursor, no generations, sticky loading/refreshing, current-model identity |
| C. PROV-176 list latency and the Codex section | F15-F17 (3) | synchronous /models in every list, startup model overwrite, unavailable_reason display |
| D. Rendering, layout and mouse | F18-F23 (6) | scroll window off by one, possible slice panic, scrollbar drag snaps back, truncation, dead state |

Top 5 by user impact: **F9, F8, F16, F1, F15**. Next in line: F11, F18, F4.

---

## Patterns the codebase already uses (use these for the fixes)

- **Generation fences.** `oauth_generation` (`views/provider_settings/mod.rs:96`, checked in `app/dispatch_provider_settings_oauth.rs:153,177,229,254`). `ModelSync { revision, tail }`: a per-session revision plus a serialized tail (`app/dispatch_model_sync.rs:11-15,40-44,105-111`). BUG-549's `PendingInputHydrated { generation }` (`app/dispatch_model_thinking_dialogs.rs`, `try_dispatch_model_thinking_dialogs`).
- **Identity captured when the request is made** (BUG-552/BUG-560): a result carries the session or view it was made for and is applied only there.
- **Render-time reconcile** (model selector, MODEL-007): carve the filter and error rows off the body, set `visible_rows` from what is left, then call `adjust_scroll()` (`views/model_selector/render.rs:105-151`). Provider Settings does not do this (F18).
- **Deterministic tests.** BUG-549/552 drive the real `App` against `tests/common::MockBackend` with `#[tokio::test(start_paused = true)]`. They reproduce an interleaving by putting actions on the channel in the losing order before draining (`tests/bug552_submission_goes_to_typed_in_session.rs:1-60`). `MockBackend` has no per-call delay or gate today (`tests/common/mock_methods_2_b.rs:25-33` for `list_providers`). Add one, for example a `tokio::sync::Notify` or a paused-clock `sleep` per method for `list_providers`, `list_provider_credentials` and `list_provider_settings`, so that two in-flight calls can be made to resolve in reverse order.

---

## Card A: Provider Settings async ordering and state

### F1: The Settings form can open before the provider settings load; saving it erases the stored settings
- **Where:** `views/provider_settings/list_actions.rs:64-74` (`provider_configs.get(..).cloned().unwrap_or_default()`). `app/dispatch_provider_settings_profiles.rs:12-23` (settings are read only after credentials load). `app/dispatch_provider_settings_defaults.rs:8-27`. Backend: `rust/sessions/src/provider_settings.rs:142-167` → `rust/sessions/src/profile_persistence.rs:189+` (`set_or_remove`: a `None` field **removes** the key).
- **Failing sequence:** `/provider` opens, `list_provider_credentials` resolves and `ProviderCredentialsLoaded` builds the nav tree. The user expands `anthropic` and presses Enter on **Settings** before `ProviderSettingsConfigsLoaded` arrives. The form opens with an all-default `ProfileDefinition`. The user changes one field and presses Enter. `SaveProviderSettings` sends defaults for every other field, so contextWindow, maxOutputTokens, compactionThreshold, streaming, autoContinue, maxImages and the loop-detection fields are deleted from `providers.anthropic.settings`.
- **Why:** the two-stage load leaves a window where Settings rows exist but `provider_configs` is empty. Nothing marks the configs as not loaded, and the form cannot tell "empty" from "not loaded".
- **Severity:** user-visible data loss. The window is short on the embedded transport (a file read) and longer on WebSocket.
- **Fix direction:** keep a `settings_loaded: bool`, or `provider_configs: Option<HashMap>`. Until it is loaded, Enter on Settings is consumed with a status such as "loading settings…", or the form opens read-only. Better still, load credentials and settings in one task and apply them in one action (see F2), so Settings rows never appear before their data.
- **Test:** App with MockBackend seeded with `seed_provider_settings`. Dispatch `ProviderCredentialsLoaded(list)` by hand, but do not drain the settings read (or gate it). Press Right then Enter on Settings, then submit. Assert that either no `save_provider_settings` call happens, or the saved definition equals the seeded one.

### F2: Credential and settings loads have no request generation; concurrent refreshes overwrite each other
- **Where:** list spawners with no tag: `app/dispatch_provider_settings.rs:44-64` (open), `:92-94` (save credentials), `:164-166` (refresh models), `:206-208` (delete credentials). `app/dispatch_provider_settings_profiles.rs:127-129` (save profile), `:164-166` (delete profile). `app/dispatch_provider_settings_oauth.rs:242-246` (login success). `app/oauth_disconnect.rs:20-22`. The settings reload runs on every credentials load (`dispatch_provider_settings_profiles.rs:21`) and also after a settings save (`dispatch_provider_settings_defaults.rs:48-57`). Both are applied unconditionally by `views/provider_settings/settings_config.rs:6-26` and `:29-67`.
- **Failing sequences:**
  1. Save profile P1 (task S1: write, then list L1, then settings read R1). Delete profile P2 right after (task S2: delete, then L2, then R2). If R1 starts before S2's delete lands but resolves after R2, then `set_provider_settings_configs` (R1) runs last and P2 comes back in the tree until the next reload.
  2. Save settings (write, then R_a), and a credentials load triggered at the same time spawns R_b. Whichever settings read finishes last wins, regardless of which started last.
  3. Every `ProviderCredentialsLoaded` spawns another `list_provider_settings`, so N quick writes produce 2N backend reads and 2N tree rebuilds. Each rebuild runs focus restore (F4).
- **Why:** results are not tied to a request generation. The view applies whatever arrives last. The two halves of one refresh are separate actions that can interleave with other refreshes.
- **Severity:** latent on embedded (reads are fast). Real on the WebSocket transport and with slow keyring lookups.
- **Fix direction:** add an App-level `provider_settings_refresh_gen: u64` (not on the view; see F5). Each refresh task captures it and returns a single `ProviderSettingsRefreshed { gen, creds, configs }`. Drop results whose gen is older than the last applied one. Coalesce: a new refresh request supersedes any queued one.
- **Test:** MockBackend with a gated `list_provider_settings`. Start two refreshes, release the second gate first, then the first. Assert the view shows the second snapshot.

### F3: The "navigate to provider" target is consumed by whichever reload arrives first
- **Where:** set when the disconnect starts (`app/oauth_disconnect.rs:7-9`), on login success (`app/dispatch_provider_settings_oauth.rs:235`) and on profile delete (`app/dispatch_provider_settings_profiles.rs:66-75`). Consumed in `views/provider_settings/nav_tree_ops.rs:158-170` (`take()`), from both `dispatch_provider_settings_profiles.rs:20` and `settings_config.rs:66`.
- **Failing sequences:**
  1. A save-credentials refresh L1 is in flight. The user confirms "Logout from OAuth [Codex]", which sets the target to `codex`. L1 lands first and moves the cursor to the Codex provider row while the Logout row is still there. The real disconnect reload L2 then finds no target. The Logout row vanishes and the cursor lands on whatever row slid into its index.
  2. The target is set before `oauth_clear_tokens` returns. If the user moves the cursor in the meantime, the reload yanks it back to the provider row.
- **Why:** a single `Option<String>` slot with no link to the request that set it.
- **Severity:** user-visible but intermittent.
- **Fix direction:** put the target in the refresh request and result: `ProviderSettingsRefreshed { gen, navigate_to: Option<String> }`. Apply it only to that result, and only if the user has not navigated since (record a `nav_epoch` on key navigation).
- **Test:** dispatch `OAuthDisconnect`. Before draining, dispatch a `ProviderCredentialsLoaded` that still contains the token. Then drain. Assert the cursor ends on the provider row after the final reload, and not on the row that followed Logout.

### F4: Focus restore keys on a display label, and `set_providers` clamps the nav cursor using the provider count
- **Where:** `NavItemKind::Profile { profile_name }` holds the **display label** `"{name} → {baseUrl} — missing …"` (`views/provider_settings/nav_item.rs:32`, built by `settings_config.rs:76-96`). Focus restore compares items for equality (`settings_config.rs:7,21-25` and `:30,60-65`). `views/provider_settings/mod.rs:141-147` `set_providers` clamps `selected_index` to `providers.len()-1`, although `selected_index` indexes `nav_items` (index-space mismatch, the PROV-102 class). It runs inside `apply_provider_credentials` (`settings_config.rs:20`).
- **Failing sequences:**
  1. Edit profile `local` (cursor on its row, index 12) and change its base URL, rename it, or fill a missing limit (BUG-547 suffix). After the save, stage 1 still uses the old labels, so the cursor holds. Stage 2 (`set_provider_settings_configs`) rebuilds with the new label, the focused item is not found, and `selected_index` stays 12. Profiles are sorted by label, so the renamed profile can move and the cursor lands on a different profile, or on Create new profile.
  2. Any reload where the focused row is gone and focus is not found in stage 1 (for example an api-key or OAuth row that disappears without a navigate target): `set_providers` has already clamped the cursor to `providers.len()-1` (for example 9). That is an unrelated row in the expanded tree.
  3. `selected_index` is never clamped to `nav_items.len()`, so after a shrink it can point past the end. Nothing is highlighted, Enter and `d` do nothing (`focused_nav_item()` is `None`), and Up jumps to the last row.
- **Severity:** user-visible (the cursor jumps after a profile edit).
- **Fix direction:** give `NavItemKind::Profile` a stable identity `{ name, label }` and compare by `(provider_id, kind discriminant, name)`. Remove the clamp from `set_providers`, or make it clamp in nav space after the rebuild. After every rebuild run `selected_index = min(selected_index, nav_len-1)` and `adjust_scroll()`.
- **Test:** pure view test. Load configs with profiles `a`, `b`, `c` and focus `b`. Reload configs with `b` renamed to `z`. Assert the focus is on `z` (or on the provider row), and not on `c`.

### F5: The OAuth generation resets to 0 with every new view, and one start task is never tracked
- **Where:** the view is replaced on every open (`app/dispatch_provider_settings.rs:27`), so `oauth_generation` starts at 0 again (`views/provider_settings/mod.rs:130`). The generation is bumped only on Esc (`views/provider_settings/oauth_login.rs:91-97`), not on start (`:32`), retry (`:122-128`) or success (`:73-78`). The copilot device-start task is pushed to `pending_tasks` but **not** to `track_oauth_task` (`app/dispatch_provider_settings_copilot.rs:44-69`). Compare `dispatch_provider_settings_oauth.rs:139,197,220`.
- **Failing sequence:** in `/provider`, start a GitHub Copilot login: `OAuthCopilotDeviceStart{gen 0}`, with the HTTP device-start in flight. Press Esc (gen becomes 1; tracked tasks are aborted, but this one is untracked and keeps running). Press Esc again to close. Reopen `/provider` (new view, gen 0). The old task resolves with `OAuthDeviceReady{gen 0}`, which matches the new view. The list view jumps to "device waiting" with a code the user never asked for, and `handle_oauth_device_ready` starts a real `oauth_device_poll`. The same reuse lets any already-queued result from a previous view instance pass the fence.
- **Why:** the fence counter belongs to the view instance, and the view is recreated on every open and every Tab round trip.
- **Severity:** user-visible but needs fast hands. The fence itself is unsound.
- **Fix direction:** move the counter to `App` (monotonic for the whole process) and bump it on every login start, retry and cancel. Track every OAuth task, copilot start included, and abort on Esc and Drop. Bump the counter as well when the view is replaced.
- **Existing coverage:** `tests/provider_settings_oauth_login_prov113.rs:822` covers Esc within one view instance only.
- **Test:** start copilot (gen 0), Esc, Esc, dispatch `OpenProviderSettingsView`, then dispatch `OAuthDeviceReady{generation: 0}`. Assert the mode is still `List` and no `oauth_device_poll` call was recorded.

### F6: One shared `status` string takes every async result, in every mode, with no provider and no lifetime
- **Where:** `app/dispatch_provider_settings.rs:146` (test result for any provider), `:184-186`, `:220-222`. `dispatch_provider_settings_profiles.rs:124-133,155-171`. `dispatch_provider_settings_defaults.rs:13-24,44-62`. Rendered in List mode at `views/provider_settings/body_render.rs:110-129`, in the forms in **red** whatever the text (`views/provider_settings/profile_form_render.rs:143-147`), and in EditApiKey in red (`views/provider_settings/detail.rs:256-262`). Form Cancel does not clear it (`views/provider_settings/profile_form_submit.rs:45-48`), and Submit clears it only implicitly when the next async status arrives.
- **Failing sequences:**
  1. Save profile A, then open profile B's edit form at once. "✓ openai: A profile saved" appears in **red** under B's form.
  2. Enter a non-numeric Max Images value; the hint shows. Press Esc. The hint stays at the bottom of the list (BUG-334 painter) until some other status replaces it.
  3. "✓ X credentials cleared" stays in List mode for the rest of the visit. It also takes a list row, which hides the cursor (F18).
  4. `ProviderTestComplete` for provider A prints "✓ A ok" while the user is on B. The RPC-158 inline per-provider decoration (`test_result`) is never set in production code (only `clear_test_result` is called).
- **Severity:** user-visible, cosmetic to confusing.
- **Fix direction:** a typed status `{ text, level: Ok|Err, scope: Provider(id)|Form|List, epoch }`. Colour by level. Clear it on a mode change. Route test results to `set_test_result(provider_id, …)`.
- **Test:** pure view. Set the form, dispatch `ProviderSettingsStatus("✓ …")`, render, and assert the line is not red. Cancel the form after an `Err` hint and assert the status is empty.

### F7: Loading looks like empty, and stage 2 makes the tree flicker
- **Where:** `views/provider_settings/list_nav_render.rs:35-47` and `views/provider_settings/list.rs:237-249` paint "(no providers configured)" whenever `nav_items` is empty. That is the case before the first `ProviderCredentialsLoaded`, and permanently after "✗ list failed". Stage 1 builds profiles from the previous `profile_configs`, which are empty on first open (`settings_config.rs:8-18`), so the OpenAI profile rows, the "(N profiles)" badge and the title count "(N items)" appear only after stage 2.
- **Failing sequence:** open `/provider` on a slow transport. You see "(no providers configured)", then the providers, then the openai badge and profile rows a moment later. The header count changes twice.
- **Severity:** user-visible cosmetic. This is the same class PROV-104 fixed in the model selector (`views/model_selector/rows.rs:29-37`).
- **Fix direction:** add a `loaded` flag with "Loading providers…" and a distinct failed state. Apply credentials and configs together (F2).
- **Test:** render before any load, assert "Loading"; dispatch a list failure, assert a failure text instead of "(no providers configured)".

---

## Card B: Model selector async ordering and state

### F8: The model selector view is never reset on open; it shows the previous visit's state until a late list wipes it
- **Where:** `app/dispatch_model_selector.rs:21-31` only calls `set_session` and `set_current_model`, then spawns the list. `ModelSelectorView::new()` runs once, in `views/navigator.rs:93`. So `providers`, `rows`, `loaded=true`, `filter`, `filter_mode`, `expanded`, `selected_index`, `scroll_offset`, `custom_model_mode`/`form` and `is_refreshing` all carry over. Provider Settings, by contrast, resets on open (`app/dispatch_provider_settings.rs:27`).
- **Failing sequence:** open `/model`, type a filter or expand Anthropic, press Esc. Later (often by Tab from `/provider` after signing in to Codex), open `/model` again. The old rows show at once with no "Loading models…", including the old Codex "(not signed in…)" reason and the old filter prompt. The user starts navigating or expanding. Then the new `ListProvidersLoaded` lands; with PROV-176 that is after a Codex `/models` round trip, up to 10 s (F15). `set_providers` re-collapses every section and moves the cursor (F9), throwing away what the user did. A list from the previous visit that resolves after the close is also applied to the hidden view.
- **Severity:** user-visible, and made worse by PROV-176 latency.
- **Fix direction:** add a `ModelSelectorView::reset_for_open()` that clears `loaded`, `rows`, `filter`, `filter_mode`, `custom_model_mode`, `is_refreshing`, `selected_index`, `scroll_offset` and `has_selection` (keep `expanded` only if product wants that). Pair it with a request generation (F10) so a load from a previous visit is dropped.
- **Test:** open, load, filter "gpt", close. Reopen and render before draining. Assert "Loading models…" and no filter row.

### F9: Every list load resets expansion and the cursor, including `r` refresh, custom-model writes and the late initial load (log evidence)
- **Where:** `views/model_selector/state.rs:74-116`: `self.expanded = HashSet::new()` at `:84`, re-seed or `has_selection=false` at `:109-114`. Called for every `ListProvidersLoaded` (`app/dispatch_model_selector.rs:73-75`), whatever caused it: open (`:77-94`), `r` (`:42-69`), or add/edit/delete custom model (`:100-164`).
- **Failing sequences:**
  1. Expand Anthropic, press `r`. When the list returns Anthropic is collapsed and the cursor is back on the current model (or nothing is selected).
  2. Add a custom model under profile `local` (`a`). The write's reload collapses `local` unless it holds the current model, so the user cannot see the model they just added, and the cursor moves away from where they were.
  3. F8's late initial load.
- **Log evidence** (`~/.fspec/logs/private/fspec-combined.log.2026-11-06`), right after a Claude OAuth login:
  ```
  00:59:14.137 INFO codelet_providers::claude_oauth_server Claude OAuth login successful — tokens persisted to claude_auth.json
  00:59:19.437 INFO model_select [MODEL-SELECT] Enter pressed in browse mode has_selection=true row_count=4 selected_index=3 ...
  00:59:19.437 INFO model_select [MODEL-SELECT] Enter on header -> toggling expansion provider_key=anthropic was_expanded=false
  00:59:20.121 INFO model_select [MODEL-SELECT] navigator relaying Emit onto action bus action=RefreshModelSelector has_tx=true
  00:59:20.121 INFO codelet_providers::models::cache Force refreshing models cache
  00:59:21.136 INFO codelet_providers::models::cache Models cache updated: /home/rquast/.fspec/cache/models.json
  00:59:21.284 INFO profile_sections profile model discovery discovery=supported ...      <- list_providers being built
  00:59:22.259 INFO model_select [MODEL-SELECT] Enter pressed in browse mode has_selection=true row_count=4 selected_index=3 ...
  00:59:22.259 INFO model_select [MODEL-SELECT] Enter on header -> toggling expansion provider_key=anthropic was_expanded=false
  00:59:23.974 INFO model_select [MODEL-SELECT] Enter pressed ... row_count=21 selected_index=6 ... model_id=claude-opus-5-5
  ```
  The user expanded Anthropic at :19, pressed `r` at :20, and the reload at about :21.3 collapsed it again (`row_count=4`, `was_expanded=false` at :22), so they had to expand it a second time.
- **Spec conflict:** `spec/features/model-selector-default-expansion.feature` (RPC-342) rule 6 and its scenario "Reloading providers re-applies the collapse default" (`views/model_selector/tests_collapse.rs:102-126`) say that a reload re-applies collapse-by-default. **This needs a product decision.** Proposal: collapse-by-default applies on **open** only. `r` and custom-model reloads keep `expanded` (pruned to keys that still exist) and keep the cursor on the same `(provider_key, model_id)`, or on its header. After a custom-model add, also expand the target profile and focus the new model.
- **Severity:** user-visible, confirmed by the log above.
- **Test:** load, Right on Anthropic, dispatch `RefreshModelSelector`, drain the MockBackend list. Assert `is_expanded("anthropic")` and the cursor still on the same row. A second test: AddCustomModel into a collapsed profile, then assert the profile is expanded and the new model focused.

### F10: List loads have no generation; concurrent lists resolve in arbitrary order
- **Where:** three spawners send the same untagged `Action::ListProvidersLoaded` (`app/dispatch_model_selector.rs:58-66, 83-92, 155-162`). `r` sets `is_refreshing=true` and fires a new refresh with no dedupe (`views/model_selector/dispatch.rs:46-49`). Each refresh runs `refresh_models_cache("")`, which rewrites `~/.fspec/cache/models.json`, and then lists.
- **Failing sequences:**
  1. Open `/model` (list A starts, slow /models). Add a custom model straight away on the stale rows (F8): write, then list B. B started after the write and returns first with the model. A, built before the write, returns last and replaces it, so the new custom model disappears until the next reload.
  2. Tab between `/provider` and `/model` repeatedly. Each `OpenModelSelectorView` starts another list (each with a /models fetch), and the slowest one wins.
  3. Press `r` twice. Two force-refreshes write `models.json` at the same time, `is_refreshing` is cleared by the first arrival while the second is still running, and the second arrival collapses everything again (F9).
- **Severity:** user-visible with PROV-176 latency.
- **Fix direction:** an App-level `model_list_gen`. `ListProvidersLoaded { gen, providers }` is applied only if `gen == latest_requested`. A list in flight is superseded rather than raced. `r` while refreshing is a no-op, or coalesced. `is_refreshing` is cleared only by the matching gen (and on failure, F11).
- **Test:** MockBackend with gated `list_providers`. Open (gate A), AddCustomModel (gate B). Release B, then A. Assert the custom model is still listed.

### F11: A failed list leaves "Loading models…" or "(refreshing...)" on screen forever
- **Where:** the error branches send only `SessionOperationFailed`, which opens the session-storage dialog (`app/session_storage.rs:50-53,61-68`) and never touches the selector (`app/dispatch_model_selector.rs:62,87,157`). `is_refreshing` is cleared only in `set_providers` (`views/model_selector/state.rs:96`). `loaded` likewise (`:95`). Title: `state.rs:230-238`. Body placeholder: `views/model_selector/rows_render.rs:64-84`.
- **Failing sequences:** (a) On first open with a corrupt providers file (RPC-451), the dialog appears and, once dismissed, the selector says "Loading models…" for good. (b) After `r`, a failed list leaves the title on "Select Model (N models) (refreshing...)" until a later list succeeds.
- **Severity:** user-visible.
- **Existing coverage:** `tests/rpc451_provider_listing_failures_are_shown.rs:51-68` asserts only that the dialog shows.
- **Fix direction:** add a `ListProvidersFailed { gen, error }` action for the selector that sets `loaded=true`, `is_refreshing=false`, stores `load_error` and renders it in the body (like BUG-325 `write_error`). Keep the dialog if product wants it.
- **Test:** extend the RPC-451 test. After the dialog, render the selector and assert no "Loading models…" and no "(refreshing...)".

### F12: The renderer ignores `has_selection`, and `selected_index` is never reset or clamped
- **Where:** highlight at `views/model_selector/rows_render.rs:114` (`abs_i == selected_index`, no `has_selection` check). `state.rs:109-114`: when there is no current model only `has_selection=false` is set, and `selected_index` keeps its old value. Navigation helpers `components/model_selector_dialog_rows.rs:68-84` (`move_up_clamped` from an out-of-range index stays out of range). `a`, `e` and `d` ignore `has_selection` (`views/model_selector/crud.rs:18-79`).
- **Failing sequences:**
  1. With no current model (PROV-101 "nothing selected"), row `selected_index` is still drawn highlighted. When it is a model row, Enter on it does nothing (`dispatch.rs:155-161`), so the UI shows a selection that Enter will not act on.
  2. Previous visit: the user ended on row 40 of an expanded list. On reopen (F8) with no current model the rows collapse to 6 headers and `selected_index` stays 40. Nothing is highlighted. Up goes to 39, which is still invisible. Left, Right, Enter, `a`, `e` and `d` all no-op (`rows.get(40)` is `None`) until Down or Home.
  3. With a stale highlighted custom-model row, `d` opens a delete confirm for a model the user never explicitly selected.
- **Severity:** user-visible.
- **Existing coverage:** `views/model_selector/tests_no_auto_select.rs:12-40` checks only `has_active_selection()`, and its Enter lands on a header, so it passes trivially.
- **Fix direction:** when `!has_selection`, render no selected band, or a dim "cursor" style. In `set_providers` and after every `rebuild_rows`, clamp `selected_index` to `rows.len()-1`, or set it to the first header when nothing is selected. Gate `e` and `d` on `has_selection`.
- **Test:** pure view. Set `selected_index=40` with an expanded load, reload collapsed with no current model. Assert `selected_index < rows.len()`, and that the render has no row in the selected style.

### F13: "Current model" is identified by model id alone; the store drops the provider, and a late metadata load never re-expands
- **Where:** `app/dispatch_model_sync.rs:84-88,113-120` stores only `SessionInfo.model_id` (the bare model, provider dropped: `rust/sessions/src/session_manager/manager_impl_2.rs:97-112`, `manager_impl_3.rs:84-87`). Matching ignores `provider_key`: `views/model_selector/rows.rs:148-155`, `state.rs:85-93`, and the "(current)" marker at `rows_render.rs:177`. `reconcile_current_model` (`state.rs:59-63`) updates the id but does not re-expand or re-seed. The same metadata task also bundles `get_model_info`: if that fails the whole result is `Err` and the model id is not stored (`dispatch_model_sync.rs:83-94,123-126`).
- **Failing sequences:**
  1. The same id in two sections, for example a custom provider and Codex both listing `gpt-6.1-terra`, or two local profiles serving the same `qwen…` model. The first section is auto-expanded even if the session uses the second, and "(current)" is painted in **both** sections.
  2. `/model` opened right after session creation, before `ModelSelectionLoaded` is stored: `current_model_id=None`, so nothing auto-expands and the cursor is not seeded. When the metadata lands, "(current)" appears, but the section stays collapsed.
- **Log pattern, consistent but not conclusive:** three times a session is created at startup with `openai:Strata/qwen3.8-flash-next-iq3_s` and `/model` is opened about 2 s later. Each time the first logged key is on the `codex` header with `row_count=4`, meaning every section was collapsed, Strata included:
  ```
  03:53:27.929 ... create_session_with_id: session creation complete model=openai:Strata/qwen3.8-flash-next-iq3_s
  03:53:29.604 INFO profile_sections profile model discovery ...
  03:53:30.867 INFO model_select [MODEL-SELECT] Enter pressed ... row_count=4 selected_index=2 ... provider_key=codex
  04:48:17.328 ... session creation complete model=openai:Strata/qwen3.8-flash-next-iq3_s
  04:48:21.178 INFO model_select [MODEL-SELECT] Enter pressed ... row_count=4 selected_index=2 ... provider_key=codex
  05:10:11.208 ... session creation complete model=openai:Strata/qwen3.8-flash-next-iq3_s
  05:10:15.081 INFO model_select [MODEL-SELECT] Enter pressed ... row_count=4 selected_index=2 ... provider_key=codex
  ```
  With a current model of `qwen3.8-flash-next-iq3_s`, `set_providers` should have expanded `openai:Strata` (rows would be greater than 4). Either the current model was not known at open (sequence 2), or the user collapsed Strata by hand each time. The existing logs cannot tell these apart. Add a `debug!` in `handle_open_model_selector_view` and `set_providers` with the current model and the expanded key.
- **Severity:** user-visible (wrong section expanded, duplicate markers, no auto-expand after a fresh session).
- **Fix direction:** keep a single source of truth with `(provider_key, model_id)` in `agent_view_store`, carrying `provider_id` from `SessionInfo`, and match on both. Split the `list_sessions` model lookup from `get_model_info`, so one failure does not discard the other. When `reconcile_current_model` changes the model and nothing has been navigated yet (`!has_selection`), re-run the expand-and-seed.
- **Test:** pure view. Two providers both containing `m1`, current `(p2, m1)`. Assert `p2` is expanded and exactly one "(current)" is shown. App test: open the selector, deliver `ListProvidersLoaded`, then deliver `ModelSelectionLoaded`, and assert the current section is expanded.

### F14: Two selection paths are not tied to the latest selection (BUG-547 retry; no-session double create)
- **Where:** `app/dispatch_model_sync.rs:59-81` turns a failure into `OpenMissingProfileLimitPrompt { retry: ModelSelected(X) }`, with no revision. `app/dispatch_missing_profile_limit.rs:93-97` re-dispatches `ModelSelected(X)` unconditionally. The no-session path `app/dispatch_model_thinking_dialogs.rs:62-95` runs `set_default_model` plus `run_session_creation` per selection, unserialized.
- **Failing sequences:**
  1. Select profile model X, which is missing a limit, then quickly select Y. The syncs are chained, so Y waits for X. X fails and the prompt is queued; Y succeeds. The prompt for X then appears over a session already on Y. Answering it saves the limit and **re-selects X**, overriding Y.
  2. With no session, select a model twice in quick succession (reopen `/model` and select again before the first creation returns). Two `set_default_model` calls race, and two sessions are created.
- **Severity:** latent.
- **Fix direction:** carry `revision` in `MissingLimitRetry::ModelSelected` and drop the retry if the session's `ModelSync.revision` has moved on (save the limit, skip the reselect). Serialize no-session selections through one tail, as `ModelSync` does.
- **Test:** script `set_session_model` to fail for X with a MissingProfileLimit error, then select X and Y. Deliver `SaveMissingProfileLimit`. Assert the last `set_session_model` call is Y.

---

## Card C: PROV-176 list latency and the Codex section

### F15: Codex `/models` is fetched synchronously inside every `list_providers`; it blocks a server worker and races the 10 s RPC deadline
- **Where:** `rust/sessions/src/codex_section.rs:29-43,96-109` (a blocking `std::thread::scope` and a new runtime on each list). The caller is the synchronous `list_providers` (`rust/sessions/src/handle_impl/handle_impl_5.rs:5-140`), called from the async tarpc handler with no `spawn_blocking` (`rust/rpc/src/service_implementation/providers.rs:5-11`). Fetch timeout: `rust/providers/src/codex/models_catalog/fetch.rs:23` `FETCH_TIMEOUT = 10s`. Local profile probes run one after another, each through `block_in_place` (`rust/sessions/src/profile_discovery.rs:67-80`, `profile_sections.rs:148-163`). The tarpc client deadline is 10 s (`tarpc-0.34.0/src/context.rs:91-105`; confirmed in `client.log.2026-10-28` line 13: `otel.name=FspecService.list_providers rpc.deadline=…07:00:23.437` for a call sent at `07:00:13.437`). Call sites: selector open, `r` (after a forced models.dev refresh), every custom-model write, every Tab back to `/model`, and **startup** (`app/bootstrap_model.rs:33`).
- **Failing sequence:** chatgpt.com is slow (8-10 s) and one local profile probe takes 2 s. `list_providers` takes about 11 s, which is past the 10 s tarpc deadline. The client gets a deadline error and shows "The provider list could not be read" (modal), plus F11's stuck "Loading models…". The Codex `unavailable_reason` path is never reached, although it exists for exactly this case. Short of the deadline, every list is still slowed by the /models round trip, which widens the F8, F9 and F10 windows. The handler also holds a server worker thread for the whole fetch.
- **Severity:** user-visible under poor network, and every list is slower.
- **Fix direction:** fetch Codex `/models` asynchronously with its own short budget (2-3 s), well under the RPC deadline. Cache the last good catalog with a TTL, so open and Tab reuse it and only `r` forces a fetch. Run `list_providers` in `spawn_blocking` on the server. Probe profiles concurrently.
- **Test:** sessions crate. Point `/models` at a local mock that sleeps longer than the budget and assert `list_providers` returns within the budget with `unavailable_reason` set. TUI: assert a slow `list_providers` (MockBackend gate) leaves the selector usable.

### F16: At startup, a transient `/models` failure permanently replaces the user's saved model
- **Where:** `app/bootstrap_model.rs:20-52` lists providers and resolves with `rust/sessions/src/startup_model_resolution.rs:52-92`, which keeps only sections with models (`:60-64`; a failed Codex section has `models=[]`) and falls back to first-available. It then calls `set_default_model`, which **persists** `tui.lastUsedModel` (`rust/sessions/src/session_manager/manager_impl_1.rs:114-133`).
- **Failing sequence:** the last used model is `codex/gpt-6.1-terra`. fspec starts while offline, on a captive portal, or with chatgpt.com slow. The Codex section comes back with `unavailable_reason` and no models, so the persisted model is not eligible. First-available, for example `openai:Strata/qwen…`, is committed and written to `~/.fspec/fspec-config.json`. The next run, online, starts on the profile model. The user's choice is silently lost. Before PROV-176, Codex models came from the on-disk models.dev catalog and did not depend on the network at startup.
- **Severity:** user-visible, latent until the next network blip.
- **Fix direction:** in startup resolution, treat a section with `unavailable_reason` as **unknown**, not absent: do not fall back away from a persisted model in that section, and do not persist a fallback. Or keep the default unpersisted until a successful listing confirms it.
- **Test:** `resolve_startup_model` with a Codex section `{models: [], unavailable_reason: Some(..)}` and persisted `codex/x`. Assert it returns `None`, or a "keep persisted" result, rather than first-available. App bootstrap with MockBackend: assert no `set_default_model` that differs from the persisted model.

### F17: How `unavailable_reason` is displayed
- **Where:** `views/model_selector/header.rs:39-48` appends ` ({reason})` in red, cloud branch only. The profile/unreachable branch (`:50-67`) ignores the reason. The reason strings are long (`rust/providers/src/codex/models_catalog/fetch.rs:72-76`: "Codex thinking levels come from the Codex /models endpoint, which needs ChatGPT sign-in; Codex is not signed in with ChatGPT (an API key cannot fetch them)"; `:35-58` "the Codex /models request failed: …").
- **Quirks:**
  1. The text is about **thinking levels** (MODEL-014 wording reused) while the header is about the model list.
  2. `Paragraph` cuts at the right edge with no ellipsis. On an 80-100 column terminal the actionable tail ("not signed in with ChatGPT") is lost after `▶ Codex (ChatGPT) (0 models) (Codex thinking levels come from the Codex /models endpoint, wh`.
  3. Enter or Right "expands" the empty section (the arrow flips to ▼ with nothing under it).
  4. Because the fetch runs on every list, the Codex section switches between N models and "0 models (reason)" across refreshes. When the current model is a Codex model, auto-expand and "(current)" disappear on a failed refresh, and the order of rows under the cloud group shifts.
- **Severity:** user-visible, cosmetic to confusing.
- **Fix direction:** reason strings written for the selector ("not signed in with ChatGPT — sign in under /provider", "Codex model list unavailable: <short cause>"). Put the reason on its own dim child row when it is too long, or truncate the middle with an ellipsis. Do not toggle sections with no models. Reuse the last good catalog for display (F15).
- **Test:** render at width 80 with a long reason and assert the visible text contains the key phrase.

---

## Card D: Rendering, layout and mouse

### F18: Provider Settings' scroll window is off by one when the filter row or status line is shown, so the cursor goes off-screen
- **Where:** `views/provider_settings/body_render.rs:24-30` sets `visible_rows` from the list area **including** the filter row, which `views/provider_settings/list.rs:205-224` then takes away. The status line (BUG-334) takes a row (`body_render.rs:110-129`) whenever an async status lands, but nothing calls `adjust_scroll` at render time. `ensure_visible` runs only on key and wheel input (`views/provider_settings/mod.rs:251-271`). Scrollbar geometry uses the same stale `visible_rows` (`views/provider_settings/mouse.rs:167-177`).
- **Failing sequences:**
  1. Type `/open`, press Enter, then press Down to the bottom. `ensure_visible` thinks there is one more row than is painted, so the selected row sits one line below the last painted row: the cursor is invisible.
  2. With the cursor on the last visible row, an async "✓ saved" arrives. The list shrinks by a row and the cursor is hidden until the next key.
- **Severity:** user-visible.
- **Fix direction:** do as the model selector does (`views/model_selector/render.rs:126-151`). Compute the list rect after the status and filter rows, set `visible_rows` from it, and call `adjust_scroll()` inside render.
- **Test:** render 80x12, set a filter, press End, render. Assert the selected style appears on a painted row. Repeat with `set_status("✓ x")`.

### F19: Possible panic: `nav_items[scroll_offset..end]` with `scroll_offset > len` after a shrinking reload
- **Where:** `views/provider_settings/list_nav_render.rs:48,59`. `end = min(scroll+rows, len)` is less than `scroll_offset` when `scroll_offset > len`, and slicing panics. Nothing clamps `scroll_offset` or `selected_index` after `apply_provider_credentials`, `set_provider_settings_configs` or `set_browser_login_enabled` (`settings_config.rs`, `nav_tree_ops.rs:97-136`). The model selector guards this (`views/model_selector/rows_render.rs:88` `so = scroll_offset.min(total-1)`).
- **Failing sequence:** scroll near the bottom of a long tree (many openai profiles, several providers expanded; offset 25). A reload returns a much shorter tree (profiles removed by another process, a provider removed, the browser login rows gated off). The next render panics, crashing the TUI.
- **Severity:** latent crash.
- **Fix direction:** clamp `scroll_offset` and `selected_index` in one `after_rebuild()` helper called by every rebuild. Also clamp defensively in the renderer, as the model selector does.
- **Test:** set nav to 40 items with `scroll_offset=30`, call `set_provider_display_infos` with 5 items, render, and assert no panic and `scroll_offset == 0`.

### F20: Scrollbar drag snaps back to the selection in both views, and drags off the 1-column gutter are dropped
- **Where:** Provider Settings `views/provider_settings/mouse.rs:49-52` and model selector `views/model_selector/dispatch.rs:216-219` both do `scroll_offset = offset; adjust_scroll()`. `adjust_scroll` → `ensure_visible` immediately pulls the window back to contain `selected_index` (`components/scroll_viewport.rs:46-66`). The model selector also calls `adjust_scroll()` on every render (`views/model_selector/render.rs:151`). Both views route Drag and Up only while the pointer is inside the gutter rect (`rect_contains(sb_rect, …)`) and never consult `scrollbar_drag.is_active()`. `views/agent/mouse_dispatch.rs:100,210` in the agent view does it the right way.
- **Failing sequence:** drag the scrollbar thumb down. The list jumps, then snaps back to the cursor, or does not move at all in the model selector. Drift one column to the left mid-drag and the drag stops.
- **Severity:** user-visible (TUI-101 drag broken in these two views).
- **Fix direction:** follow `views/agent/resume_session_view_mouse.rs:57-63`. On drag, set `scroll_offset` and move `selected_index` into the window (or keep the selection, and skip `ensure_visible` for that frame by tracking a "user-scrolled" flag). Route Drag and Up whenever `scrollbar_drag.is_active()`.
- **Existing coverage:** `tests/provider_settings_scrollbar_rpc352.rs` is render-only.
- **Test:** 60 rows, selection at 0. Send Down at the thumb, then Drag to the bottom, and assert `scroll_offset > 0` after the next render.

### F21: Truncation and short-terminal layout
- **Where:** profile row labels (`views/provider_settings/settings_config.rs:76-96`) are cut by `set_stringn` with no ellipsis (`views/provider_settings/row_render.rs:126-155`). The profile form paints up to 17 lines (title, blank, name, auth note, 13 fields, status) with no scrolling (`views/provider_settings/profile_form_render.rs:111-149`; `PROFILE_FORM_FIELDS` has 13 entries, `profile_form.rs:24`). The body is `height - 3` (`views/full_screen_shell.rs:22`).
- **Quirks:**
  1. `local → http://192.168.1.50:8080/v1 — missing context window, max output tokens` loses the BUG-547 "missing …" tail (the actionable part) on narrow terminals, because the base URL comes first.
  2. On a terminal under about 20 rows the loop-detection fields and the status line, which carries the validation error, are clipped. The user can tab to a field they cannot see, and an error they cannot see blocks the save.
- **Severity:** user-visible on small terminals.
- **Fix direction:** put the missing-limits badge before the URL, or truncate the URL in the middle. Scroll the form so the focused field stays visible, and pin the status line.
- **Test:** render the form at 80x15 with the last field focused and a status set. Assert both are visible.

### F22: Dead and unreachable state in Provider Settings
- **Where:** `DetailSub::Summary` and `DetailSub::OAuthNotice` are reachable only through the legacy fallback when `nav_items` is empty (`views/provider_settings/list.rs:92-122`), which never happens with real data. So `Action::RefreshProviderModels` and `Action::TestProviderConnection` have no UI path (`views/provider_settings/detail.rs:67-76`; `app/dispatch_provider_settings.rs:105-187`). "Refreshing models…" in `last_status` would never clear, because `ProviderModelsRefreshed` writes `status`, not `last_status`. `test_result` (RPC-158) is never set. `focused_provider()` and `visible_providers()` index `providers` by a nav-space `selected_index` (`views/provider_settings/mod.rs:149-151`).
- **Severity:** latent, but it confuses anyone fixing F4 or F6.
- **Fix direction:** decide whether to remove these paths or wire them up. At minimum stop indexing `providers` with `selected_index`.

### F23: Model selector filter quirks
- **Where:** the filter survives close and reopen (F8). A committed filter cannot be cleared from browse mode: Esc closes (`views/model_selector/dispatch.rs:33`; Esc clears only inside `filter_mode`, `crud.rs:170-177`). A filter with no matches shows "No models available" (`rows.rs:32`, `rows_render.rs:68-72`), which reads as "you have no models".
- **Severity:** user-visible, minor.
- **Fix direction:** in browse mode with a committed filter, Esc clears the filter first (Provider Settings already does this, `views/provider_settings/list.rs:35-47`). Add a "No models match '<filter>'" state. Reset the filter on open (F8).

---

## Existing tests that already touch these areas

- `tests/provider_settings_oauth_login_prov113.rs:822` `cancelling_device_waiting_drops_late_poll_result`: generation fence within one view (not across reopen, F5).
- `tests/provider_settings_oauth_disconnect_prov112.rs`, `tests/provider_settings_profile_delete_navigation_prov116.rs`: the navigate target with a single in-flight reload (not interleaved, F3).
- `tests/provider_settings_profile_rename_prov136.rs`: the rename RPC path, not cursor placement after it (F4).
- `views/model_selector/tests_collapse.rs:102-126`: **encodes** collapse-on-reload (F9 conflict).
- `views/model_selector/tests_no_auto_select.rs`: `has_selection` only; Enter lands on a header (F12).
- `tests/rpc451_provider_listing_failures_are_shown.rs:51`: dialog only (F11).
- `tests/model_selector_refresh_forces_catalog_refetch_bug207.rs`: refresh ordering inside one task (not across tasks, F10).
- `tests/model_selector_filter_row_model007.rs`: model selector filter-row reservation, the behaviour Provider Settings lacks (F18).
- `tests/provider_settings_scrollbar_rpc352.rs`, `tests/mode_view_scroll_input_rpc353.rs`: scrollbar painting and wheel only (F20).
- No test covers any interleaving of `ProviderCredentialsLoaded`, `ProviderSettingsConfigsLoaded` or `ListProvidersLoaded`.

## Log evidence summary

- Files: `~/.fspec/logs/private/fspec-combined.log.2026-11-03..06` (11-03 to 11-05 are only bootstrap and profile-discovery lines), `client.log.2026-10-28`.
- **The logs end at `2026-11-06T11:13Z`, before the PROV-176 commit (`2026-11-06T13:08Z`).** There is no runtime evidence of the PROV-176 `/models`-per-list path yet, so F15-F17 rest on code reading.
- F9 (refresh collapses expansion): 2026-11-06 00:59:19-00:59:23, quoted above. This is the clearest real-world occurrence.
- F13 (current-model section not auto-expanded after a fresh profile session): 03:53:30, 04:48:21, 05:10:15, quoted above. Consistent with the bug but not conclusive.
- F15 (10 s deadline): `client.log.2026-10-28` line 13 shows `rpc.deadline` 10 s after the `list_providers` send. Pre-PROV-176 `list_providers` took about 1.1 s (line 14, received at 07:00:14.528).
- Three Claude OAuth servers were started within one minute (00:57:59, 00:58:48, 00:58:57) before a success at 00:59:14: repeated login attempts. This does not show a race by itself, but it is the flow where the F5 generation reuse would bite.
- No `list_provider_credentials failed`, `settings load failed`, `custom model write failed` or deadline errors appear in the logs.
