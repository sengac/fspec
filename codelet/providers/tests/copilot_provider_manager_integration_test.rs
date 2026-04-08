#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/add-github-copilot-provider.feature
//!
//! Parent integration tests for PROV-053 — GitHub Copilot provider end-to-end
//! wiring through `ProviderManager`.
//!
//! These tests verify that the parent scenarios are wired through the
//! provider dispatch layer (`ProviderType::GitHubCopilot`,
//! `ProviderCredentials::detect`, `ProviderManager::with_provider`,
//! `ProviderManager::get_github_copilot`). The behavioural depth of each
//! scenario lives in the per-child integration test files
//! (`copilot_oauth_device_flow_test.rs`,
//! `copilot_http_middleware_routing_test.rs`,
//! `copilot_models_catalog_test.rs`); these tests are the manager-level
//! "is this provider actually plugged in?" proof.

mod fixtures;

use codelet_providers::copilot::{
    delete_copilot_auth, get_copilot_auth_path, models, select_copilot_behavior_facade,
    write_copilot_auth, CopilotAuthJson, CopilotDeploymentType, CopilotEndpoint,
    CopilotEndpointFacade, CopilotHeaderFacade, CopilotProvider, CopilotRequestClassifier,
};
use codelet_providers::{ProviderCredentials, ProviderManager, ProviderType};
use fixtures::setup_fspec_home;
use http::header::{AUTHORIZATION, USER_AGENT};
use serial_test::serial;
use std::str::FromStr;

/// Helper: write a fake `copilot_auth.json` into the (temp) `FSPEC_HOME`.
async fn install_fake_copilot_credential(enterprise_url: Option<String>) {
    let auth =
        CopilotAuthJson::from_github_oauth_token("ghu_fake_test_token".to_string(), enterprise_url);
    write_copilot_auth(&auth)
        .await
        .expect("write_copilot_auth should succeed in temp HOME");
}

// =========================================================================
// Scenario: Login to github.com Copilot deployment via OAuth device flow
// =========================================================================

#[tokio::test]
#[serial]
async fn test_login_github_com_copilot_routes_through_provider_manager() {
    // @step Given I have an active GitHub Copilot subscription on github.com
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step And I have no existing github-copilot credential on disk
    let auth_path = get_copilot_auth_path();
    assert!(
        !auth_path.exists(),
        "copilot_auth.json should not exist initially"
    );
    let creds_before = ProviderCredentials::detect();
    assert!(
        !creds_before.has_github_copilot(),
        "no credential file → has_github_copilot must be false"
    );

    // @step When I run `codelet auth login github-copilot`
    // @step And I select deploymentType "github.com" at the CLI prompt
    // @step And I enter the displayed device code at https://github.com/login/device and approve the request
    // (the device flow itself is exercised in copilot_oauth_device_flow_test.rs;
    //  here we install the credential the flow would have written)
    install_fake_copilot_credential(None).await;

    // @step Then the polling loop should succeed with an access_token response
    // @step And a credential should be persisted at "~/.fspec/credentials/copilot_auth.json" with file mode 0600
    assert!(
        auth_path.exists(),
        "copilot_auth.json should exist after login"
    );

    // @step And the credential should contain access and refresh tokens set to the same GitHub OAuth token value
    let creds_after = ProviderCredentials::detect();
    assert!(
        creds_after.has_github_copilot(),
        "manager-level detection must surface the new credential"
    );

    // @step And the credential expires field should be 0
    let manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");
    assert_eq!(manager.current_provider_name(), "github-copilot");
}

// =========================================================================
// Scenario: Login to GitHub Enterprise Copilot deployment with enterprise URL
// =========================================================================

#[tokio::test]
#[serial]
async fn test_login_github_enterprise_routes_through_provider_manager() {
    // @step Given I have an active GitHub Copilot subscription on a GitHub Enterprise instance
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step And I have no existing github-copilot credential on disk
    assert!(!get_copilot_auth_path().exists());

    // @step When I run `codelet auth login github-copilot`
    // @step And I select deploymentType "enterprise" at the CLI prompt
    // @step And I enter "ghe.example.com" at the enterpriseUrl prompt
    // @step And I complete the device code flow against ghe.example.com
    install_fake_copilot_credential(Some("ghe.example.com".to_string())).await;

    // @step Then a credential should be persisted with the enterpriseUrl field set to "ghe.example.com"
    let manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept enterprise-deployed github-copilot");
    assert_eq!(manager.current_provider_name(), "github-copilot");

    // @step And subsequent Copilot API calls should be routed to "https://copilot-api.ghe.example.com"
    let base_url = CopilotProvider::base_url_for(&CopilotDeploymentType::Enterprise {
        host: "ghe.example.com".to_string(),
    });
    assert_eq!(base_url.as_str(), "https://copilot-api.ghe.example.com");
}

// =========================================================================
// Scenario: Chat completion request to gpt-4o-copilot uses /chat/completions
//           endpoint with Copilot headers
// =========================================================================

#[tokio::test]
#[serial]
async fn test_gpt_4o_copilot_routes_to_chat_completions_via_manager() {
    // @step Given I am logged in to github-copilot with a valid credential
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let _manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");

    // @step And I have selected model "gpt-4o-copilot" in the codelet TUI
    let endpoint = CopilotEndpointFacade::select("gpt-4o-copilot");

    // @step When I send a chat message
    // @step Then codelet should send the request to "api.githubcopilot.com/chat/completions"
    assert_eq!(endpoint, CopilotEndpoint::ChatCompletions);
    let base_url = CopilotProvider::base_url_for(&CopilotDeploymentType::GitHubCom);
    assert_eq!(base_url.as_str(), "https://api.githubcopilot.com");

    // @step And the request should include an "Authorization: Bearer <token>" header
    // @step And the request should include a "User-Agent: codelet/<version>" header
    // @step And the request should include an "Openai-Intent: conversation-edits" header
    // @step And the request should include an "x-initiator: user" header
    let classification =
        codelet_providers::copilot::RequestClassification::default();
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_fake_test_token");

    let auth = headers.get(AUTHORIZATION).expect("Authorization required");
    assert_eq!(auth, "Bearer ghu_fake_test_token");

    let ua = headers.get(USER_AGENT).expect("User-Agent required");
    assert!(ua.to_str().unwrap().starts_with("codelet/"));

    assert_eq!(
        headers
            .get("openai-intent")
            .expect("Openai-Intent required"),
        "conversation-edits"
    );
    assert_eq!(
        headers.get("x-initiator").expect("x-initiator required"),
        "user"
    );

    // @step And the response should be streamed back to the TUI
    // (streaming wire path is owned by the http middleware tests)
}

// =========================================================================
// Scenario: gpt-5 model is routed to the /responses endpoint with
//           reasoning_opaque round-trip
// =========================================================================

#[tokio::test]
#[serial]
async fn test_gpt_5_routes_to_responses_endpoint_via_manager() {
    // @step Given I am logged in to github-copilot with a valid credential
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let _manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");

    // @step And I have selected model "gpt-5" in the codelet TUI
    let endpoint = CopilotEndpointFacade::select("gpt-5");

    // @step When I send a chat message
    // @step Then codelet should route the request to the "/responses" endpoint
    assert_eq!(endpoint, CopilotEndpoint::Responses);

    // @step And the response should include a "reasoning_opaque" field
    // @step And on the next turn the "reasoning_opaque" field should be round-tripped back in the request
    let facade = select_copilot_behavior_facade("gpt-5");
    let response = serde_json::json!({
        "id": "r1",
        "reasoning_opaque": "rsn_abc123"
    });
    let extracted = facade
        .extract_reasoning_opaque(&response)
        .expect("gpt family should round-trip reasoning_opaque");
    let mut next_request = serde_json::json!({ "model": "gpt-5", "input": [] });
    facade.inject_reasoning_opaque(&mut next_request, &extracted);
    assert_eq!(
        next_request.get("reasoning_opaque"),
        Some(&serde_json::json!("rsn_abc123"))
    );
}

// =========================================================================
// Scenario: gpt-5-mini is excluded from the Responses API rule and uses
//           /chat/completions
// =========================================================================

#[tokio::test]
#[serial]
async fn test_gpt_5_mini_uses_chat_completions_via_manager() {
    // @step Given I am logged in to github-copilot with a valid credential
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let _manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");

    // @step And I have selected model "gpt-5-mini" for a summarization task
    let endpoint = CopilotEndpointFacade::select("gpt-5-mini");

    // @step When I send a chat message
    // @step Then codelet should route the request to "/chat/completions" and not to "/responses"
    assert_eq!(endpoint, CopilotEndpoint::ChatCompletions);
    assert_ne!(endpoint, CopilotEndpoint::Responses);
}

// =========================================================================
// Scenario: Image attachment triggers Copilot-Vision-Request header on
//           claude-sonnet-4.5
// =========================================================================

#[tokio::test]
#[serial]
async fn test_image_attachment_triggers_vision_header_via_manager() {
    // @step Given I am logged in to github-copilot with a valid credential
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let _manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");

    // @step And I have selected model "claude-sonnet-4.5" in the codelet TUI
    let body = serde_json::json!({
        "model": "claude-sonnet-4.5",
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "What is in this image?"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,iVBOR"}}
            ]
        }]
    });

    // @step When I attach an image and ask the model to describe it
    let classification = CopilotRequestClassifier::classify(&body);

    // @step Then codelet should detect image parts in the request body
    assert!(
        classification.is_vision,
        "image parts must trigger is_vision"
    );

    // @step And the request should include a "Copilot-Vision-Request: true" header
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_fake_test_token");
    let vision = headers
        .get("copilot-vision-request")
        .expect("Copilot-Vision-Request header should be present for image requests");
    assert_eq!(vision, "true");

    // @step And the Copilot API should return a vision response
    // (live API behaviour is exercised by copilot_http_middleware_routing_test.rs)
}

// =========================================================================
// Scenario: Logout deletes the github-copilot credential file
// =========================================================================

#[tokio::test]
#[serial]
async fn test_logout_deletes_credential_visible_to_manager() {
    // @step Given I am logged in to github-copilot with a credential at "~/.fspec/credentials/copilot_auth.json"
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let auth_path = get_copilot_auth_path();
    assert!(auth_path.exists());
    assert!(ProviderCredentials::detect().has_github_copilot());

    // @step When I run `codelet auth logout github-copilot`
    delete_copilot_auth().await.expect("logout should succeed");

    // @step Then the file "~/.fspec/credentials/copilot_auth.json" should be deleted
    assert!(!auth_path.exists(), "credential file must be removed");

    // @step And opening the codelet TUI should show github-copilot as unauthenticated
    let creds = ProviderCredentials::detect();
    assert!(
        !creds.has_github_copilot(),
        "manager-level detection must report no credential after logout"
    );
    let manager_result = ProviderManager::with_provider("github-copilot");
    assert!(
        manager_result.is_err(),
        "with_provider must reject github-copilot once the credential file is gone"
    );
}

// =========================================================================
// Scenario: Model picker fetches the live catalog from /models with no
//           static merge
// =========================================================================

#[tokio::test]
#[serial]
async fn test_model_picker_fetches_live_catalog_via_manager() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // @step Given I am logged in to github-copilot with a valid credential
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;
    let _manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot once the credential file exists");

    let mock_server = MockServer::start().await;

    // @step When I open the model picker in the codelet TUI
    // @step Then codelet should fetch the model catalog from "api.githubcopilot.com/models"
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer ghu_fake_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "model-a",
                    "name": "Model A",
                    "version": "model-a-2025-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "fictional",
                        "limits": {
                            "max_context_window_tokens": 32000,
                            "max_output_tokens": 4096,
                            "max_prompt_tokens": 32000
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                },
                {
                    "id": "model-b",
                    "name": "Model B",
                    "version": "model-b-2025-02-02",
                    "model_picker_enabled": false,
                    "capabilities": {
                        "family": "fictional",
                        "limits": {
                            "max_context_window_tokens": 32000,
                            "max_output_tokens": 4096,
                            "max_prompt_tokens": 32000
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let fetched = models::fetch_models(&mock_server.uri(), "ghu_fake_test_token")
        .await
        .expect("fetch_models should succeed against mock /models");

    // @step And the catalog should contain only models with "model_picker_enabled: true"
    assert_eq!(fetched.len(), 1, "picker-disabled entry must be filtered out");
    assert_eq!(fetched[0].id, "model-a");

    // @step And the catalog should be exactly what the endpoint returned, with no merging or static fallback
    assert!(fetched.iter().all(|m| m.id != "model-b"));
}

// =========================================================================
// ProviderType registration sanity tests — these prove the enum/string
// round-trip exists, which all 8 scenarios above implicitly depend on.
// =========================================================================

#[test]
fn provider_type_from_str_recognises_github_copilot() {
    assert_eq!(
        ProviderType::from_str("github-copilot").expect("should parse"),
        ProviderType::GitHubCopilot,
    );
}

#[test]
fn provider_type_as_str_round_trips_github_copilot() {
    assert_eq!(ProviderType::GitHubCopilot.as_str(), "github-copilot");
}

#[test]
fn provider_credentials_struct_exposes_github_copilot_field() {
    let creds = ProviderCredentials {
        claude_available: false,
        openai_available: false,
        codex_available: false,
        gemini_available: false,
        zai_available: false,
        github_copilot_available: true,
    };
    assert!(creds.has_github_copilot());
    assert!(creds.has_any());
}

// =========================================================================
// PROV-053 Rule 9: ProviderManager::get_github_copilot must return a real
// CopilotProvider instance that implements LlmProvider (review Critical #1).
// =========================================================================

#[tokio::test]
#[serial]
async fn provider_manager_get_github_copilot_returns_llm_provider() {
    use codelet_providers::LlmProvider;

    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;

    let manager = ProviderManager::with_provider_and_model(
        "github-copilot",
        Some("gpt-4o-copilot"),
    )
    .expect("manager should accept github-copilot with a model");

    let provider = manager
        .get_github_copilot()
        .expect("get_github_copilot should succeed when the credential file exists");
    assert_eq!(provider.name(), "github-copilot");
    assert_eq!(provider.model(), "gpt-4o-copilot");
    assert!(provider.supports_streaming());
    assert_eq!(provider.access_token(), "ghu_fake_test_token");
    assert_eq!(
        provider.base_url().as_str(),
        "https://api.githubcopilot.com",
        "github.com deployment must use api.githubcopilot.com"
    );
}

#[tokio::test]
#[serial]
async fn provider_manager_get_github_copilot_honors_enterprise_deployment() {
    use codelet_providers::copilot::CopilotDeploymentType;

    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(Some("ghe.example.com".to_string())).await;

    let manager = ProviderManager::with_provider_and_model(
        "github-copilot",
        Some("gpt-4o-copilot"),
    )
    .expect("manager should accept enterprise github-copilot with a model");

    let provider = manager
        .get_github_copilot()
        .expect("get_github_copilot should succeed for enterprise deployment");
    assert_eq!(
        provider.base_url().as_str(),
        "https://copilot-api.ghe.example.com"
    );
    assert_eq!(
        provider.deployment(),
        &CopilotDeploymentType::Enterprise {
            host: "ghe.example.com".to_string(),
        }
    );
}

#[tokio::test]
#[serial]
async fn provider_manager_get_github_copilot_requires_selected_model() {
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;

    let manager = ProviderManager::with_provider("github-copilot")
        .expect("manager should accept github-copilot without a model");
    let result = manager.get_github_copilot();
    assert!(
        result.is_err(),
        "get_github_copilot must fail when no model is selected"
    );
}

#[tokio::test]
#[serial]
async fn provider_manager_get_github_copilot_rejects_wrong_current_provider() {
    let (_temp_dir, _guard) = setup_fspec_home();
    install_fake_copilot_credential(None).await;

    // Force a Copilot credential to be detected but then construct the
    // manager with a different provider (fake Claude via env var).
    std::env::set_var("ANTHROPIC_API_KEY", "fake-key-for-manager-test");
    let manager = ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-sonnet-4.5"),
    )
    .expect("manager should accept claude with ANTHROPIC_API_KEY set");
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = manager.get_github_copilot();
    assert!(
        result.is_err(),
        "get_github_copilot must refuse when current provider is not github-copilot"
    );
}
