# RPC-103 — Flat Tree Navigation Model (TS-Parity Spec)

## 1. Summary

The TypeScript `ProviderSettingsPanel` renders the provider settings screen as a **single
flat `Vec<SettingsNavItem>`** that is rebuilt from `providers + filter + per-provider
isExpanded` on every state change. The current Rust implementation (`codelet/fspec-tui/
src/views/provider_settings/`) instead uses a hierarchical state machine with a
`ProviderSettingsMode::Detail { sub: DetailSub::{Summary|EditApiKey|OAuthNotice} }`
sub-view. That deviation breaks the canonical UX: in TS, pressing Enter on a provider
does **not** push you into a new modal screen — it injects child rows directly below the
provider row in the same flat list, while leaving the cursor in place. This card defines
the data model, builder semantics, and integration test plan to bring the Rust port to
exact TS parity for the navigation tree.

## 2. TypeScript Source-of-Truth Citations

| Element | File | Lines |
|---|---|---|
| `SettingsNavItem` discriminated union | `src/tui/components/ProviderSettingsPanel.tsx` | 120–135 |
| `ProviderDisplayInfo.isExpanded: boolean` | `src/tui/components/ProviderSettingsPanel.tsx` | 41–49 |
| `buildNavItems(providers, filter)` pure builder | `src/tui/hooks/useProviderSettingsState.ts` | 132–206 |
| Filter check (drops provider entirely if no name/id match) | `src/tui/hooks/useProviderSettingsState.ts` | 141–147 |
| Conditional OAuth status row (`hasOAuthTokens && isOAuthProvider`) | `src/tui/hooks/useProviderSettingsState.ts` | 159–165 |
| OAuth login rows from `buildOauthLoginNavItems` (browser/headless) | `src/tui/hooks/useProviderSettingsState.ts` | 171–174 |
| API key row gating: `id !== 'openai' && (requiresApiKey || envVar)` | `src/tui/hooks/useProviderSettingsState.ts` | 177–185 |
| Profile rows (openai-only) + trailing `add-profile` pseudo-row | `src/tui/hooks/useProviderSettingsState.ts` | 188–201 |
| `expandedProviderIds: Set<string>` ref (survives `reload()`) | `src/tui/hooks/useProviderSettingsState.ts` | 243, 340 |
| `toggleProviderExpansion` mutates both state and the survival ref | `src/tui/hooks/useProviderSettingsState.ts` | 385–400 |
| `navigateToProviderRef` — destructive ops restore selection to parent row | `src/tui/hooks/useProviderSettingsState.ts` | 246, 367–379, 420, 448 |
| `useMemo(() => buildNavItems(...), [providers, filter])` | `src/tui/hooks/useProviderSettingsState.ts` | 361–364 |
| Enter dispatch: `currentItem.type === 'provider' → toggleProviderExpansion` | `src/tui/inputHandlers/listModeHandler.ts` | 119–121 |

## 3. `SettingsNavItem` Variant Enum (TS)

```ts
type SettingsNavItem =
  | { type: 'provider';    providerId: string; name: string }
  | { type: 'profile';     providerId: string; profileName: string }
  | { type: 'add-profile'; providerId: string }
  | { type: 'api-key';     providerId: string }
  | { type: 'oauth-login'; providerId: string; method: 'browser'|'headless'; label: string }
  | { type: 'oauth-status';providerId: string; label: string };
```

Six variants total. Every row in the list — including child rows under an expanded
provider — is one of these six. There is no separate "section header" type; the
`provider` variant doubles as the header AND the toggle target.

## 4. Builder Algorithm (Plain English)

For each provider in `providers` (ordered by the canonical registry):

1. **Filter gate** — if `filter` is non-empty AND neither `provider.name.toLowerCase()`
   nor `provider.id.toLowerCase()` contains the filter substring, **skip the whole
   provider including its children**. Filter is parent-anchored: a child profile name
   matching the filter is invisible if its parent didn't match.
2. **Push `{type:'provider'}`** unconditionally for non-filtered providers.
3. **If `provider.isExpanded` is true**, push children in this fixed order:
   1. `{type:'oauth-status'}` — only if `isOAuthProvider(id) && hasOAuthTokens`
   2. `{type:'oauth-login'}` × N — only if `isOAuthProvider(id)`; N comes from
      `buildOauthLoginNavItems` (anthropic→2, codex→2, copilot→1, etc.)
   3. `{type:'api-key'}` — only if `id !== 'openai'` and registry entry declares
      `requiresApiKey || envVar`
   4. **For openai only:** one `{type:'profile'}` per loaded profile, then a trailing
      `{type:'add-profile'}` pseudo-row (always present so users can create the first
      profile).

The output list is dense — no separator rows, no whitespace rows.

## 5. State Transition Diagram — Expand/Collapse

```
                            ┌─── Enter on provider ────┐
                            │                          ▼
   [collapsed]   ──────►   toggleProviderExpansion  ──►  [expanded]
       ▲                            │                       │
       │                            ▼                       │
       │              expandedProviderIds.add/delete         │
       │              + providers[i].isExpanded = !prev      │
       │                            │                       │
       └────── Enter on provider ◄──┴── recompute navItems ──┘
                                       (useMemo dependency)
```

Critical invariants:
* Expansion state is stored **twice**: in the React state array (`providers[i].isExpanded`,
  drives rendering) AND in a `useRef<Set<string>>` (survives `reload()` which rebuilds
  the entire `providers` array from disk).
* `navItems` is recomputed reactively via `useMemo([providers, filter])` — not mutated.
* `selectedIndex` is **NOT touched** by toggle: when you expand a provider at index 3, the
  cursor stays at 3 and the new children appear at indices 4, 5, 6, … pushing the next
  provider down. Collapsing reverses this. Selection preservation across collapse is a
  separate concern (see RPC-110 sibling).

## 6. Proposed Rust Data Model

Replace `ProviderSettingsMode { List, Detail { sub: DetailSub } }` with a flat list:

```rust
// codelet/fspec-tui/src/views/provider_settings/nav_item.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavItemKind {
    Provider { expanded: bool },
    Profile { profile_name: String },
    AddProfile,
    ApiKey,
    OAuthLogin { method: OAuthMethod, label: String },
    OAuthStatus { label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthMethod { Browser, Headless }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavItem {
    pub provider_id: String,
    pub kind: NavItemKind,
}

pub fn build_nav_items(
    providers: &[ProviderDisplayInfo],
    filter: &str,
) -> Vec<NavItem> { /* mirrors TS algorithm in §4 */ }
```

The `ProviderSettingsView` becomes:

```rust
pub struct ProviderSettingsView {
    providers: Vec<ProviderDisplayInfo>,
    expanded: HashSet<String>,        // survives reload (TS expandedProviderIds ref)
    nav_items: Vec<NavItem>,          // rebuilt whenever providers/filter/expanded change
    selected_index: usize,
    scroll_offset: usize,
    filter: String,
    filter_mode: bool,
    pending_focus_provider: Option<String>, // TS navigateToProviderRef
    mode: PanelMode,                  // delete/edit/oauth modals only — NOT detail-view
    // … other fields …
}
```

`PanelMode` covers only the **modal overlays** (delete confirm, edit api-key, profile
form, oauth waiting/success/error) — list-mode navigation is no longer a `Mode` variant.
This removes `DetailSub::{Summary, EditApiKey, OAuthNotice}` entirely; their UX is
re-expressed as inline child rows that fire `Action`s when Enter is pressed.

## 7. How This Replaces `DetailSub`

| Old `DetailSub` variant | New flat-tree expression |
|---|---|
| `DetailSub::Summary` (showed key + status when provider focused) | Inline `oauth-status` child row when `hasOAuthTokens`, plus per-row icons on the provider row itself for visual status. |
| `DetailSub::EditApiKey { draft }` | Enter on a child `NavItemKind::ApiKey` row → `PanelMode::EditApiKey { provider_id, draft }` (modal overlay), exactly mirroring TS. |
| `DetailSub::OAuthNotice` | Enter on a child `NavItemKind::OAuthLogin { method }` → fires `Action::StartBrowserLogin` or `Action::StartHeadlessLogin`, transitioning `PanelMode` to one of the `oauth-*-waiting` variants. |

## 8. Integration Test Plan

1. **`build_nav_items_no_filter_collapsed_all`** — 17 providers, none expanded → exactly
   17 `NavItem`s, all `NavItemKind::Provider { expanded: false }`.
2. **`build_nav_items_expand_openai_yields_add_profile`** — expand `openai` with 0
   profiles → injects exactly 1 `AddProfile` child below the provider row.
3. **`build_nav_items_expand_anthropic_yields_oauth_login_and_apikey`** — anthropic
   expanded, no tokens → 2 OAuthLogin rows (browser + headless) + 1 ApiKey row.
4. **`build_nav_items_expand_anthropic_with_tokens_prepends_status`** — same provider
   with `hasOAuthTokens=true` → row order is `[provider, oauth-status, oauth-login×2,
   api-key]`.
5. **`build_nav_items_filter_drops_provider_and_all_children`** — expanded openai with
   profiles, filter="anth" → only anthropic rows survive; openai's children are gone.
6. **`expansion_set_survives_reload`** — call `set_providers(new_list)` after expanding
   anthropic; the rebuilt nav-list still shows anthropic expanded.
7. **`enter_on_provider_row_toggles_expansion_and_does_not_change_selected_index`** —
   selected_index stays put when expand/collapse fires.
8. **`enter_on_api_key_row_opens_editapikey_panel_mode`** — verifies Enter dispatch on
   child rows transitions `PanelMode`, not into a Detail sub-view.

All eight tests must pass with `cargo test -p fspec-tui provider_settings` before this
card moves to `done`.
