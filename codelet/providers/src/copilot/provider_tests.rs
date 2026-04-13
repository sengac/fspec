//! Unit tests for [`CopilotProvider`] (PROV-053/055/057).
//!
//! Feature: spec/features/github-copilot-end-to-end-integration.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::copilot::auth::CopilotAuthJson;
use crate::copilot::endpoint::CopilotEndpoint;
use crate::copilot::oauth_types::CopilotDeploymentType;

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
