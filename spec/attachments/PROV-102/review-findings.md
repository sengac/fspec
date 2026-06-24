# Review Findings — PROV-102 (Enter on OpenAI profile opens Anthropic detail view)

**Reviewer:** spawned ACDD reviewer (session 741b5bb9), supervised.
**Result:** PASS. No 🔴, no blocking 🟡.

## Verified
- Index-space mismatch ELIMINATED. Enter (`list.rs:85-116`) dispatches via
  `list_actions::enter_on_nav_item(view, provider_id, kind)` using `focused_nav_item().provider_id`
  + `.kind`; the legacy `visible_providers().get(selected_index)` block is reachable ONLY after the
  `focused_nav_item()` early-return — i.e. only when nav_items is empty. Same for `d`
  (`list.rs:117-141`). Grep confirms no remaining populated-tree fallthrough.
- Delete-confirm Primary path (`mod.rs:191-202`) resolves target via `delete_target_provider_id()`
  (`nav_tree_ops.rs:83-91`), preferring `focused_nav_item().provider_id`, never `providers[idx]`.
- CRITICAL proof test (`prov102_nav_item_action_dispatch.rs:78-109`): `multi_provider_view()` loads
  openai(0)/anthropic(1)/gemini(2) via both set_providers AND set_provider_display_infos, expands
  openai, steps onto the "fast" profile (selected_index=1), Enter → asserts provider_id == "openai"
  AND != "anthropic". Old code would have yielded anthropic (providers[1]). Genuinely catches the
  regression. `d`→OAuthStatus test asserts ConfirmDeleteProviderCredentials("anthropic"), not gemini.
- PROV-101 mandate honored: no new silent selection fallback. AddProfile Enter and
  Profile/AddProfile/OAuthLogin `d` are explicit Consumed no-ops; no row-0/registry-index snap.
- Gates green: cargo test full crate 0 failed; clippy --all-targets -D warnings clean; fmt --check
  clean; build success. All touched files <300: list.rs 265, list_actions.rs 130, nav_tree_ops.rs 92,
  mod.rs 296. // @step byte-exact; tests fully offline. Coverage 100% (7/7).

## 🟢 Observations
- TS parity is an HONEST, DOCUMENTED gap (acceptable), not silently-wrong behavior. vs
  listModeHandler.ts:119-153: TS Profile→edit-profile, AddProfile→new-profile, OAuthLogin→browser/
  device login, OAuthStatus→disconnect. The Rust port lacks those modes, so it routes
  Profile→Detail::Summary, OAuthLogin/OAuthStatus→Detail::OAuthNotice, AddProfile→no-op — each
  carrying the row's own provider_id (no mismatched-index path). Disclosed in the feature docstring
  and list_actions.rs:17-22. Wiring the real flows is out of scope (recommend follow-up cards).
