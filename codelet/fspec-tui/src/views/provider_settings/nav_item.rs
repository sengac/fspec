//! RPC-103 — Flat NavItem tree data model & pure builder.
//!
//! Feature: spec/features/rpc103-provider-settings-flat-tree-nav-model.feature
//!
//! Mirrors the TypeScript `SettingsNavItem` discriminated union + the
//! `buildNavItems` pure function from
//! `src/tui/hooks/useProviderSettingsState.ts:132-206`. The flat list
//! contains six row variants; expansion of a provider injects child
//! rows beneath it in a fixed order: oauth-status?, oauth-login×N?,
//! api-key?, profiles×N?, add-profile?.
//!
//! This module is intentionally pure: `build_nav_items` is a function
//! of `(providers, expanded, filter)` with no interior mutability.

use std::collections::HashSet;

/// Which OAuth login flow a `NavItemKind::OAuthLogin` row drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthMethod {
    Browser,
    Headless,
}

/// One row in the flat ProviderSettings tree. Six variants total —
/// every row, including child rows under an expanded provider, is one
/// of these. The `provider` variant doubles as the header AND the
/// toggle target.
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
pub struct NavItem {
    pub provider_id: String,
    pub kind: NavItemKind,
}

/// Display-layer projection of a provider, enriched with the registry
/// metadata needed by `build_nav_items` (TS analog: `ProviderDisplayInfo`
/// in `src/tui/components/ProviderSettingsPanel.tsx:41`).
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

/// Pure builder: walk `providers` in canonical registry order, apply
/// parent-anchored filter, and emit a flat `Vec<NavItem>` with child
/// rows in the fixed order: oauth-status?, oauth-login×N?, api-key?,
/// profiles×N?, add-profile? (openai only).
///
/// Mirrors `buildNavItems` in
/// `src/tui/hooks/useProviderSettingsState.ts:132-206`.
pub fn build_nav_items(
    providers: &[ProviderDisplayInfo],
    expanded: &HashSet<String>,
    filter: &str,
) -> Vec<NavItem> {
    let mut items: Vec<NavItem> = Vec::new();
    let filter_lower = filter.to_lowercase();

    for provider in providers {
        // Parent-anchored filter (Rule 5): if neither name nor id
        // contains the substring, the provider AND its children are
        // dropped from navItems.
        if !filter.is_empty()
            && !provider.name.to_lowercase().contains(&filter_lower)
            && !provider.id.to_lowercase().contains(&filter_lower)
        {
            continue;
        }

        let is_expanded = expanded.contains(&provider.id);

        // The provider row itself.
        items.push(NavItem {
            provider_id: provider.id.clone(),
            kind: NavItemKind::Provider {
                expanded: is_expanded,
            },
        });

        if !is_expanded {
            continue;
        }

        // Child-row order (Rule 4):
        //   1. oauth-status?  (only when is_oauth_provider && has_oauth_tokens)
        //   2. oauth-login×N? (only when is_oauth_provider)
        //   3. api-key?       (only when id != "openai" && (requires_api_key || env_var.is_some()))
        //   4. profiles×N?    (openai only)
        //   5. add-profile?   (openai only — always trails, so users can create the first)

        if provider.is_oauth_provider && provider.has_oauth_tokens {
            let label = provider
                .oauth_status_label
                .clone()
                .unwrap_or_else(|| format!("Logout from OAuth [{}]", provider.name));
            items.push(NavItem {
                provider_id: provider.id.clone(),
                kind: NavItemKind::OAuthStatus { label },
            });
        }

        if provider.is_oauth_provider {
            for (method, label) in &provider.oauth_login_methods {
                items.push(NavItem {
                    provider_id: provider.id.clone(),
                    kind: NavItemKind::OAuthLogin {
                        method: *method,
                        label: label.clone(),
                    },
                });
            }
        }

        if provider.id != "openai" && (provider.requires_api_key || provider.env_var.is_some()) {
            items.push(NavItem {
                provider_id: provider.id.clone(),
                kind: NavItemKind::ApiKey,
            });
        }

        if provider.id == "openai" {
            for profile_name in &provider.profiles {
                items.push(NavItem {
                    provider_id: provider.id.clone(),
                    kind: NavItemKind::Profile {
                        profile_name: profile_name.clone(),
                    },
                });
            }
            // The trailing add-profile pseudo-row is ALWAYS present
            // for openai when expanded (mirrors TS lines 196-200).
            items.push(NavItem {
                provider_id: provider.id.clone(),
                kind: NavItemKind::AddProfile,
            });
        }
    }

    items
}
