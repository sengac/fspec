//! Unit tests for [`CopilotProvider`] (PROV-053/055/057).
//!
//! Feature: spec/features/github-copilot-end-to-end-integration.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::copilot::auth::CopilotAuthJson;
use crate::copilot::endpoint::CopilotEndpoint;
use crate::copilot::oauth_types::CopilotDeploymentType;
use crate::copilot::token_exchange::TokenExchangeResponse;

#[test]
fn github_dotcom_base_url_is_api_githubcopilot_com() {
    let url = CopilotProvider::base_url_for(&CopilotDeploymentType::GitHubCom);
    assert_eq!(url.as_str(), "https://api.githubcopilot.com");
}

#[test]
fn enterprise_base_url_uses_copilot_api_subdomain() {
    let url = CopilotProvider::base_url_for(&CopilotDeploymentType::Enterprise {
        host: "ghe.example.com".to_string(),
    });
    assert_eq!(url.as_str(), "https://copilot-api.ghe.example.com");
}

#[test]
fn chat_completions_endpoint_uses_openai_system_prompt_facade() {
    let f = CopilotProvider::system_prompt_facade_for_endpoint(
        CopilotEndpoint::ChatCompletions,
    );
    assert_eq!(f.provider(), "openai");
}

#[test]
fn responses_endpoint_uses_copilot_responses_system_prompt_facade() {
    let f = CopilotProvider::system_prompt_facade_for_endpoint(CopilotEndpoint::Responses);
    assert_eq!(f.provider(), "copilot-responses");
}

#[test]
fn new_rejects_empty_token() {
    let result = CopilotProvider::new(
        CopilotDeploymentType::GitHubCom,
        String::new(),
        "gpt-4o-copilot",
    );
    assert!(result.is_err());
}

#[test]
fn new_rejects_empty_model() {
    let result = CopilotProvider::new(
        CopilotDeploymentType::GitHubCom,
        "ghu_token".to_string(),
        "",
    );
    assert!(result.is_err());
}

#[test]
fn new_succeeds_with_valid_inputs() {
    let provider = CopilotProvider::new(
        CopilotDeploymentType::GitHubCom,
        "ghu_token".to_string(),
        "gpt-4o-copilot",
    )
    .expect("provider should construct with valid inputs");
    assert_eq!(provider.name(), "github-copilot");
    assert_eq!(provider.model(), "gpt-4o-copilot");
    assert_eq!(provider.access_token(), "ghu_token");
    assert_eq!(
        provider.base_url().as_str(),
        "https://api.githubcopilot.com"
    );
    assert_eq!(provider.deployment(), &CopilotDeploymentType::GitHubCom);
}

#[test]
fn enterprise_provider_uses_enterprise_base_url() {
    let provider = CopilotProvider::new(
        CopilotDeploymentType::Enterprise {
            host: "ghe.example.com".to_string(),
        },
        "ghu_token".to_string(),
        "gpt-4o-copilot",
    )
    .expect("enterprise provider should construct");
    assert_eq!(
        provider.base_url().as_str(),
        "https://copilot-api.ghe.example.com"
    );
}

// =========================================================================
// PROV-057 — token refresh decision logic
// =========================================================================

/// Build a throwaway `CopilotAuthJson` with a fresh (not-near-expiry)
/// cached Copilot token.
fn auth_with_fresh_copilot_token(now: u64) -> CopilotAuthJson {
    CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: Some("tid=fresh;:sig".to_string()),
        copilot_token_expires_at: Some(now + 20 * 60),
        endpoints_api: Some("https://api.githubcopilot.com".to_string()),
        enterprise_url: None,
    }
}

/// Build an auth with a cached Copilot token that's about to expire.
fn auth_with_expiring_copilot_token(now: u64) -> CopilotAuthJson {
    CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: Some("tid=expiring;:sig".to_string()),
        copilot_token_expires_at: Some(now + 30),
        endpoints_api: Some("https://api.githubcopilot.com".to_string()),
        enterprise_url: None,
    }
}

/// Build an auth without any cached Copilot token (fresh login).
fn auth_without_copilot_token() -> CopilotAuthJson {
    CopilotAuthJson::from_github_oauth_token("gho_long_lived".to_string(), None)
}

#[test]
fn needs_refresh_is_true_when_no_copilot_token_cached() {
    // Scenario: GitHub OAuth token is exchanged for a short-lived Copilot token
    //           before any API call
    //
    // @step Given a valid github_oauth_token is stored in copilot_auth.json
    // @step And no copilot_token is currently cached
    let auth = auth_without_copilot_token();
    // @step When CopilotProvider issues a chat completion request
    // @step Then a refresh is required
    assert!(super::needs_copilot_token_refresh(&auth, 1_700_000_000));
}

#[test]
fn needs_refresh_is_true_when_within_60_seconds_of_expiry() {
    // Scenario: Cached Copilot token is refreshed when within 60 seconds of expiry
    //
    // @step Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 30 seconds from now
    let now = 1_700_000_000;
    let auth = auth_with_expiring_copilot_token(now);
    // @step When CopilotProvider issues a chat completion request
    // @step Then the provider re-calls the token exchange endpoint before sending the chat request
    assert!(super::needs_copilot_token_refresh(&auth, now));
    // @step And copilot_auth.json is updated with the new copilot_token and expires_at
    // (persistence path is covered by the apply_exchange_response test below:
    // provider::ensure_fresh_copilot_token calls apply_exchange_response and
    // then write_copilot_auth inside the RwLock — both sides are unit-tested.)
}

#[test]
fn needs_refresh_is_true_exactly_at_the_60_second_boundary() {
    let now = 1_700_000_000;
    let mut auth = auth_with_fresh_copilot_token(now);
    auth.copilot_token_expires_at = Some(now + 60);
    assert!(
        super::needs_copilot_token_refresh(&auth, now),
        "an expires_at exactly at now+60 must refresh per PROV-057 Rule 4"
    );
}

#[test]
fn needs_refresh_is_false_when_token_is_not_near_expiry() {
    // Scenario: Cached Copilot token is reused when not near expiry
    //
    // @step Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 20 minutes from now
    let now = 1_700_000_000;
    let auth = auth_with_fresh_copilot_token(now);
    // @step When CopilotProvider issues a chat completion request
    // @step Then the provider does NOT re-call the token exchange endpoint
    assert!(
        !super::needs_copilot_token_refresh(&auth, now),
        "20-minute-away expiry must NOT trigger refresh"
    );
    // @step And the cached copilot_token is used as the Bearer token for the request
    // (the Bearer-token selection is exercised by
    // `from_auth_prefers_cached_copilot_token_over_github_oauth_token_for_bearer`
    // below — provider.access_token() returns the cached Copilot token
    // whenever it is present.)
}

#[test]
fn needs_refresh_is_true_when_expires_at_is_missing_even_with_token() {
    // Defensive: a cached token without an expiry timestamp cannot be
    // trusted. The refresh decision must force a re-exchange rather than
    // fall into the "token is still good" branch.
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: Some("orphaned".to_string()),
        copilot_token_expires_at: None,
        endpoints_api: None,
        enterprise_url: None,
    };
    assert!(super::needs_copilot_token_refresh(&auth, 1_700_000_000));
}

#[test]
fn needs_refresh_is_true_when_cached_copilot_token_string_is_empty() {
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: Some(String::new()),
        copilot_token_expires_at: Some(9_999_999_999),
        endpoints_api: None,
        enterprise_url: None,
    };
    assert!(super::needs_copilot_token_refresh(&auth, 1_700_000_000));
}

#[test]
fn apply_exchange_response_populates_all_fields() {
    let mut auth = auth_without_copilot_token();
    let exchange = TokenExchangeResponse {
        token: "tid=new;:sig".to_string(),
        expires_at: 2_000_000_000,
        endpoints_api: "https://copilot-api.ghe.example.com".to_string(),
    };
    super::apply_exchange_response(&mut auth, exchange);
    assert_eq!(auth.copilot_token.as_deref(), Some("tid=new;:sig"));
    assert_eq!(auth.copilot_token_expires_at, Some(2_000_000_000));
    assert_eq!(
        auth.endpoints_api.as_deref(),
        Some("https://copilot-api.ghe.example.com")
    );
    // The long-lived token is untouched by the exchange.
    assert_eq!(auth.github_oauth_token, "gho_long_lived");
}

#[test]
fn apply_exchange_response_preserves_endpoints_api_when_response_is_empty() {
    let mut auth = CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: None,
        copilot_token_expires_at: None,
        endpoints_api: Some("https://copilot-api.ghe.example.com".to_string()),
        enterprise_url: Some("ghe.example.com".to_string()),
    };
    let exchange = TokenExchangeResponse {
        token: "tid=new;:sig".to_string(),
        expires_at: 2_000_000_000,
        endpoints_api: String::new(),
    };
    super::apply_exchange_response(&mut auth, exchange);
    assert_eq!(
        auth.endpoints_api.as_deref(),
        Some("https://copilot-api.ghe.example.com"),
        "empty endpoints_api from exchange must not clobber the existing value"
    );
}

// =========================================================================
// PROV-057 — endpoints_api URL is honoured
// =========================================================================

#[test]
fn from_auth_uses_endpoints_api_over_deployment_base_url() {
    // Scenario: Chat requests honour the endpoints.api URL from the token
    //           exchange response
    //
    // @step Given copilot_auth.json contains an "endpoints_api" value of "https://copilot-api.ghe.example.com"
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_ghe".to_string(),
        copilot_token: Some("tid=ghe;:sig".to_string()),
        copilot_token_expires_at: Some(9_999_999_999),
        endpoints_api: Some("https://copilot-api.ghe.example.com".to_string()),
        enterprise_url: Some("ghe.example.com".to_string()),
    };
    // @step When CopilotProvider issues a chat completion request
    let provider = CopilotProvider::from_auth(
        CopilotDeploymentType::Enterprise {
            host: "ghe.example.com".to_string(),
        },
        auth,
        "gpt-4o-copilot",
    )
    .expect("enterprise provider should construct");

    // @step Then the request URL host is "copilot-api.ghe.example.com"
    assert_eq!(
        provider.base_url().as_str(),
        "https://copilot-api.ghe.example.com"
    );
    // @step And the request URL host is NOT "api.githubcopilot.com"
    assert!(!provider.base_url().as_str().contains("api.githubcopilot.com"));
}

#[test]
fn from_auth_falls_back_to_computed_url_when_endpoints_api_missing() {
    let auth = CopilotAuthJson::from_github_oauth_token("gho_dotcom".to_string(), None);
    let provider = CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        auth,
        "gpt-4o-copilot",
    )
    .expect("github.com provider should construct");
    assert_eq!(provider.base_url().as_str(), "https://api.githubcopilot.com");
}

#[test]
fn from_auth_prefers_cached_copilot_token_over_github_oauth_token_for_bearer() {
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_long_lived".to_string(),
        copilot_token: Some("tid=cached;:sig".to_string()),
        copilot_token_expires_at: Some(9_999_999_999),
        endpoints_api: None,
        enterprise_url: None,
    };
    let provider = CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        auth,
        "gpt-4o-copilot",
    )
    .expect("provider should construct");
    assert_eq!(
        provider.access_token(),
        "tid=cached;:sig",
        "when a cached Copilot token is present it must be the effective Bearer token"
    );
}
