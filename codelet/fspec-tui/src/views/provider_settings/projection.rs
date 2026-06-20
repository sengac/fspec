//! RPC-349 — Pure projection: `ProviderCredentialInfo` -> `ProviderDisplayInfo`.
//!
//! Feature: spec/features/provider-settings-rich-tree.feature
//!
//! The live `/provider` dispatch path loads a `Vec<ProviderCredentialInfo>`
//! from the backend. To render the rich RPC-103 NavItem tree (instead of
//! the legacy flat list) those records must be enriched into
//! `ProviderDisplayInfo` and handed to
//! `ProviderSettingsView::set_provider_display_infos`. This module owns
//! that pure enrichment so the dispatch layer stays a thin caller and the
//! mapping is unit-testable in isolation.
//!
//! Mirrors the TS enrichment in `useProviderSettingsState.ts::reload`
//! (OAuth detection, api-key gating, openai profiles) plus the per-provider
//! OAuth login labels from `oauthProviderLabels` / `buildOauthLoginNavItems`.

use codelet_rpc_types::ProviderCredentialInfo;

use super::nav_item::{OAuthMethod, ProviderDisplayInfo};

/// The canonical OAuth providers (TS `isOAuthProvider`). Any provider whose
/// id is in this set — OR whose backend `credential_type` is `"oauth"` — is
/// treated as OAuth-capable.
fn is_oauth_provider(provider_id: &str, credential_type: &str) -> bool {
    credential_type == "oauth" || matches!(provider_id, "anthropic" | "codex" | "github-copilot")
}

/// Human-friendly provider name used in OAuth status/login labels. Falls
/// back to the supplied display name when the id is unknown.
fn pretty_name(provider_id: &str, display_name: &str) -> String {
    match provider_id {
        "anthropic" => "Anthropic".to_string(),
        "codex" => "Codex (ChatGPT)".to_string(),
        "github-copilot" => "GitHub Copilot".to_string(),
        _ if !display_name.is_empty() => display_name.to_string(),
        _ => provider_id.to_string(),
    }
}

/// Per-provider OAuth login rows (method + label). Mirrors TS
/// `buildOauthLoginNavItems`: anthropic offers browser + headless code,
/// codex offers browser + device, github-copilot offers device only.
fn oauth_login_methods(provider_id: &str) -> Vec<(OAuthMethod, String)> {
    match provider_id {
        "anthropic" => vec![
            (OAuthMethod::Browser, "Sign in with browser".to_string()),
            (OAuthMethod::Headless, "Sign in with code".to_string()),
        ],
        "codex" => vec![
            (OAuthMethod::Browser, "Sign in with browser".to_string()),
            (
                OAuthMethod::Headless,
                "Sign in with device code".to_string(),
            ),
        ],
        "github-copilot" => {
            vec![(
                OAuthMethod::Headless,
                "Sign in with device code".to_string(),
            )]
        }
        // Unknown OAuth providers still get a generic browser login so the
        // expansion is never empty.
        _ => vec![(OAuthMethod::Browser, "Sign in with browser".to_string())],
    }
}

/// Project a single backend credential record into the display-layer shape
/// consumed by `build_nav_items`. `openai_profiles` is only consulted for
/// the `openai` provider (profiles are OpenAI-only, per TS Rule 29).
fn project_one(info: &ProviderCredentialInfo, openai_profiles: &[String]) -> ProviderDisplayInfo {
    let is_oauth = is_oauth_provider(&info.provider_id, &info.credential_type);
    // For OAuth providers, `configured` doubles as "has stored tokens"
    // (TS sets hasOAuthTokens when a token lookup succeeds and reports the
    // provider as configured).
    let has_oauth_tokens = is_oauth && info.configured;
    let name = pretty_name(&info.provider_id, &info.display_name);

    let oauth_status_label = if has_oauth_tokens {
        Some(format!("Logout from OAuth [{name}]"))
    } else {
        None
    };
    let login_methods = if is_oauth {
        oauth_login_methods(&info.provider_id)
    } else {
        Vec::new()
    };

    // api-key gating mirrors TS: every non-openai provider that isn't a
    // pure-OAuth provider needs an env-var-backed API key. OAuth providers
    // that ALSO accept an api key (anthropic) keep requires_api_key=true so
    // the api-key row still appears beneath them.
    let requires_api_key = info.provider_id != "openai";

    let profiles = if info.provider_id == "openai" {
        openai_profiles.to_vec()
    } else {
        Vec::new()
    };

    ProviderDisplayInfo {
        id: info.provider_id.clone(),
        name,
        configured: info.configured,
        credential_type: info.credential_type.clone(),
        model_count: info.model_count,
        has_oauth_tokens,
        is_oauth_provider: is_oauth,
        requires_api_key,
        env_var: None,
        profiles,
        oauth_login_methods: login_methods,
        oauth_status_label,
    }
}

/// Project a full backend credential list into display infos, preserving
/// the backend's (canonical registry) ordering.
pub fn project_display_infos(
    creds: &[ProviderCredentialInfo],
    openai_profiles: &[String],
) -> Vec<ProviderDisplayInfo> {
    creds
        .iter()
        .map(|info| project_one(info, openai_profiles))
        .collect()
}
