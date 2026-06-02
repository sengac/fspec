# RPC-103 — AST Research: Flat Tree Nav Model Integration Sites

## Current shape (before RPC-103)

### `ProviderSettingsView` struct (codelet/fspec-tui/src/views/provider_settings/mod.rs:55)

```rust
pub struct ProviderSettingsView {
    pub providers: Vec<ProviderCredentialInfo>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: ProviderSettingsMode,        // List or Detail{ provider_id, sub }
    pub filter: String,
    pub filter_mode: bool,
    pub delete_confirm: Option<ConfirmDialog>,
    pub status: String,
    visible_rows: usize,
}
```

### `ProviderSettingsMode` enum (mod.rs:33)

```rust
pub enum ProviderSettingsMode {
    List,
    Detail { provider_id: String, sub: DetailSub },
}

pub enum DetailSub {
    Summary { last_status: Option<DetailStatus> },
    EditApiKey { draft: String },
    OAuthNotice,
}
```

### `ProviderCredentialInfo` (codelet/rpc-types/src/lib.rs:393)

Carries only:
- `provider_id: String`
- `display_name: String`
- `configured: bool`
- `credential_type: String` ("api_key" | "oauth" | "custom")
- `model_count: u32`

**Does NOT carry** the registry metadata that TS `ProviderDisplayInfo` carries
(`hasOAuthTokens`, `requiresApiKey`, `envVar`, `profiles`, `isOAuthProvider`).

## TS source-of-truth (verified citations)

| TS Element | File | Lines |
|---|---|---|
| `ProviderDisplayInfo` interface | `src/tui/components/ProviderSettingsPanel.tsx` | 41–49 |
| `SettingsNavItem` discriminated union (6 variants) | `src/tui/components/ProviderSettingsPanel.tsx` | 120–135 |
| `buildNavItems(providers, filter)` pure builder | `src/tui/hooks/useProviderSettingsState.ts` | 132–206 |
| Parent-anchored filter check | `src/tui/hooks/useProviderSettingsState.ts` | 141–147 |
| OAuth-status conditional `isOAuthProvider && hasOAuthTokens` | `src/tui/hooks/useProviderSettingsState.ts` | 159–165 |
| OAuth-login rows via `buildOauthLoginNavItems` | `src/tui/hooks/useProviderSettingsState.ts` | 171–174 |
| API-key gating `id !== 'openai' && (requiresApiKey \|\| envVar)` | `src/tui/hooks/useProviderSettingsState.ts` | 177–185 |
| Profile rows + trailing add-profile (openai-only) | `src/tui/hooks/useProviderSettingsState.ts` | 188–201 |
| `expandedProviderIds: useRef<Set<string>>` survives reload | `src/tui/hooks/useProviderSettingsState.ts` | 243, 340 |

## Planned shape (after RPC-103)

### New module `codelet/fspec-tui/src/views/provider_settings/nav_item.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthMethod { Browser, Headless }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavItemKind {
    Provider     { expanded: bool },
    Profile      { profile_name: String },
    AddProfile,
    ApiKey,
    OAuthLogin   { method: OAuthMethod, label: String },
    OAuthStatus  { label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavItem {
    pub provider_id: String,
    pub kind: NavItemKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderDisplayInfo {
    pub id: String,
    pub name: String,
    pub configured: bool,
    pub credential_type: String,
    pub model_count: u32,
    pub has_oauth_tokens: bool,
    pub is_oauth_provider: bool,
    pub requires_api_key: bool,
    pub env_var: Option<String>,
    pub profiles: Vec<String>,
    pub oauth_login_methods: Vec<(OAuthMethod, String)>,
    pub oauth_status_label: Option<String>,
}

pub fn build_nav_items(
    providers: &[ProviderDisplayInfo],
    expanded: &HashSet<String>,
    filter: &str,
) -> Vec<NavItem>;
```

### `ProviderSettingsView` field additions (mod.rs)

```rust
pub struct ProviderSettingsView {
    pub providers: Vec<ProviderCredentialInfo>,      // KEPT for backward-compat
    pub display_providers: Vec<ProviderDisplayInfo>, // NEW, source of truth for nav_items
    pub expanded: HashSet<String>,                   // NEW, survives reload
    pub nav_items: Vec<NavItem>,                     // NEW, derived from above
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: ProviderSettingsMode,
    pub filter: String,
    pub filter_mode: bool,
    pub delete_confirm: Option<ConfirmDialog>,
    pub status: String,
    visible_rows: usize,
}

impl ProviderSettingsView {
    pub fn set_provider_display_infos(&mut self, infos: Vec<ProviderDisplayInfo>);
    pub fn toggle_expansion(&mut self, provider_id: &str);
    pub fn rebuild_nav_items(&mut self);
    pub fn focused_nav_item(&self) -> Option<&NavItem>;
}
```

### Keybind dispatch additions (list.rs)

```rust
KeyCode::Enter => match view.focused_nav_item().map(|n| &n.kind) {
    Some(NavItemKind::Provider { .. }) => {
        let pid = view.focused_nav_item().unwrap().provider_id.clone();
        view.toggle_expansion(&pid);
        ProviderSettingsEvent::Consumed
    }
    Some(NavItemKind::ApiKey) => {
        let pid = view.focused_nav_item().unwrap().provider_id.clone();
        view.mode = ProviderSettingsMode::Detail {
            provider_id: pid,
            sub: DetailSub::EditApiKey { draft: String::new() },
        };
        ProviderSettingsEvent::Consumed
    }
    // ... other NavItemKind handlers landed in follow-up cards (RPC-104..108)
    None => ProviderSettingsEvent::Consumed,  // legacy path
    _ => /* existing legacy handler */,
}
```

## Scope boundary for THIS card (RPC-103)

**IN SCOPE:**
- New `nav_item.rs` module with the full data model
- Pure `build_nav_items` function with parent-anchored filter, fixed child ordering, registry-derived OAuth/api-key gating
- `expanded: HashSet<String>` field + survival across `set_provider_display_infos` calls
- `toggle_expansion(provider_id)` method that adds/removes from set AND rebuilds nav_items
- `set_provider_display_infos(Vec<ProviderDisplayInfo>)` method that stores + rebuilds nav_items WITHOUT clearing `expanded`
- Enter on Provider NavItem → toggle expansion, selected_index unchanged
- Enter on ApiKey NavItem → transition to existing `ProviderSettingsMode::Detail { sub: EditApiKey { draft } }`
- 8 integration tests verifying scenarios 1-8

**OUT OF SCOPE (follow-up cards):**
- Row rendering migration (RPC-104 — row icons / colors / indents)
- Footer hint refactor (RPC-106)
- Full `DetailSub::{Summary, OAuthNotice}` removal — keep these for backward-compat
- Backend RPC changes to populate `ProviderDisplayInfo` from `ProviderCredentialInfo`
- Registry helper module that converts `ProviderCredentialInfo` → `ProviderDisplayInfo`
- OAuth-login Action dispatch (separate sibling card)
- Profile-form / add-profile transitions (separate sibling card)

## Test file plan

New `codelet/fspec-tui/tests/provider_settings_flat_tree_rpc103.rs` (~350 LoC, 8 tests):

1. `fresh_view_with_collapsed_providers_yields_one_nav_item_per_provider`
2. `expanding_anthropic_without_oauth_tokens_injects_oauth_login_and_api_key_children`
3. `expanding_openai_with_profiles_injects_profile_rows_and_add_profile`
4. `expanding_anthropic_with_oauth_tokens_prepends_oauth_status_row`
5. `filter_is_parent_anchored`
6. `expansion_state_survives_set_provider_display_infos`
7. `enter_on_provider_toggles_expansion_without_mutating_selected_index`
8. `enter_on_api_key_transitions_to_edit_api_key_state`

Each test uses purely the `ProviderDisplayInfo` constructor helpers + the public View API
(`set_provider_display_infos`, `toggle_expansion`, `handle_key`, inspection getters).
