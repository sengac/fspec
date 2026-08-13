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

/// OAuth source label shown as ` [{label}]` on an OAuth-logged-in
/// provider/api-key row. Mirrors TS `oauthProviderLabels.ts`:
/// anthropic→"Claude", codex→"ChatGPT", github-copilot→"GitHub Copilot",
/// fallback→"OAuth".
fn oauth_label(provider_id: &str) -> String {
    match provider_id {
        "anthropic" => "Claude".to_string(),
        "codex" => "ChatGPT".to_string(),
        "github-copilot" => "GitHub Copilot".to_string(),
        _ => "OAuth".to_string(),
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
    //
    // PROV-099: a present env api key (`masked_key = Some`) means an
    // api-key configuration, NOT an OAuth login. Anthropic declares
    // env_var=ANTHROPIC_API_KEY but auth_type=OAuth, so a mere env key
    // made `configured` true and wrongly emitted the
    // "Logout from OAuth [Anthropic]" row. Gate on `masked_key.is_none()`
    // so only a real OAuth credential (env key absent, e.g. the
    // claude_auth.json file) classifies the provider as OAuth-logged-in.
    let has_oauth_tokens = is_oauth && info.configured && info.masked_key.is_none();
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

    // PROV-098: DISPLAY masked_key/source priority (TS parity):
    //   1. backend env api key present -> copy masked_key/source verbatim.
    //   2. else OAuth-logged-in (is_oauth && configured, backend key
    //      absent) -> synthesize Some("OAuth") + Some(oauth_label).
    //   3. else -> None/None ((not configured)/(not set)).
    // NOTE: this reads BACKEND `info.masked_key`, never the display field,
    // so `has_oauth_tokens` above (PROV-099) is unaffected.
    let (display_masked_key, display_source) = if info.masked_key.is_some() {
        (info.masked_key.clone(), info.source.clone())
    } else if is_oauth && info.configured {
        (
            Some("OAuth".to_string()),
            Some(oauth_label(&info.provider_id)),
        )
    } else {
        (None, None)
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
        masked_key: display_masked_key,
        source: display_source,
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
