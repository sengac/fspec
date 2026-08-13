#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/github-copilot-http-middleware-and-routing.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-055:
//! GitHub Copilot HTTP middleware, facades & endpoint routing.
//!
//! All tests are written to FAIL until the PROV-055 implementation lands.

use codelet_providers::copilot::oauth_types::CopilotDeploymentType;
use codelet_providers::copilot::{
    base_url::{base_url_for, CopilotBaseUrl},
    behavior_facade::{
        select_copilot_behavior_facade, CopilotBehaviorFacade, CopilotClaudeBehaviorFacade,
        CopilotGptBehaviorFacade,
    },
    classifier::{CopilotRequestClassifier, RequestClassification},
    endpoint::{CopilotEndpoint, CopilotEndpointFacade},
    header_facade::CopilotHeaderFacade,
    system_prompt_facade::system_prompt_facade_for_endpoint,
};
use serde_json::json;

// ============================================================================
// Scenario: Chat completion request to gpt-4o-copilot uses /chat/completions
//           endpoint with required Copilot headers
// ============================================================================
#[test]
fn scenario_chat_completion_gpt_4o_copilot_uses_chat_completions_with_required_headers() {
    // @step Given I am logged in to github-copilot with a github.com deployment credential
    let deployment = CopilotDeploymentType::GitHubCom;

    // @step And the credential contains a valid access_token
    let access_token = "ghu_test_access_token_abc123";

    // @step When I select the model "gpt-4o-copilot" in the TUI and send a text-only chat message
    let model_id = "gpt-4o-copilot";
    let body = json!({
        "model": model_id,
        "messages": [ { "role": "user", "content": "Hello, world" } ]
    });
    let classification = CopilotRequestClassifier::classify(&body);
    let endpoint = CopilotEndpointFacade::select(model_id);
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);
    let base_url = base_url_for(&deployment);
    let full_url = format!("{}/chat/completions", base_url.as_str());

    // @step Then CopilotEndpointFacade::select("gpt-4o-copilot") should return Endpoint::ChatCompletions
    assert_eq!(endpoint, CopilotEndpoint::ChatCompletions);

    // @step And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    assert_eq!(full_url, "https://api.githubcopilot.com/chat/completions");

    // @step And the request should include header "x-initiator: user"
    assert_eq!(
        headers.get("x-initiator").map(|v| v.to_str().unwrap()),
        Some("user"),
    );

    // @step And the request should include header "User-Agent" starting with "rust/"
    let ua = headers
        .get(http::header::USER_AGENT)
        .expect("User-Agent header must be set")
        .to_str()
        .unwrap();
    assert!(
        ua.starts_with("rust/"),
        "User-Agent must start with 'rust/', got: {ua}"
    );

    // @step And the request should include header "Authorization: Bearer <access_token>"
    let auth_header = headers
        .get(http::header::AUTHORIZATION)
        .expect("Authorization header must be set")
        .to_str()
        .unwrap();
    assert_eq!(auth_header, format!("Bearer {access_token}"));

    // @step And the request should include header "Openai-Intent: conversation-edits"
    assert_eq!(
        headers.get("openai-intent").map(|v| v.to_str().unwrap()),
        Some("conversation-edits"),
    );

    // @step And the request should NOT include the "Copilot-Vision-Request" header
    assert!(
        headers.get("copilot-vision-request").is_none(),
        "Copilot-Vision-Request header must NOT be present for text-only requests"
    );
}

// ============================================================================
// Scenario: gpt-5 model is routed to the /responses endpoint with
//           reasoning_opaque round-trip
// ============================================================================
#[test]
fn scenario_gpt_5_routed_to_responses_with_reasoning_opaque_roundtrip() {
    // @step Given I am logged in to github-copilot with a github.com deployment credential
    let deployment = CopilotDeploymentType::GitHubCom;
    let _access_token = "ghu_test_access_token_gpt5";

    // @step When I select the model "gpt-5" in the TUI and send a chat message
    let model_id = "gpt-5";
    let endpoint = CopilotEndpointFacade::select(model_id);
    let behavior = select_copilot_behavior_facade(model_id);
    let base_url = base_url_for(&deployment);

    // @step Then CopilotEndpointFacade::select("gpt-5") should return Endpoint::Responses
    assert_eq!(endpoint, CopilotEndpoint::Responses);

    // @step And the outgoing request URL should be "https://api.githubcopilot.com/responses"
    assert_eq!(
        format!("{}/responses", base_url.as_str()),
        "https://api.githubcopilot.com/responses"
    );

    // @step And the selected behavior facade should be CopilotGptBehaviorFacade
    assert_eq!(behavior.family(), "gpt");

    // @step And the system prompt facade should be CopilotResponsesSystemPromptFacade
    let system_prompt_facade = system_prompt_facade_for_endpoint(endpoint);
    assert_eq!(
        system_prompt_facade.provider(),
        "copilot-responses",
        "gpt-5 must use CopilotResponsesSystemPromptFacade for /responses endpoint"
    );

    // @step When the Copilot API returns a response containing a "reasoning_opaque" field
    let api_response = json!({
        "id": "resp_test_reasoning_123",
        "output": [],
        "reasoning_opaque": "opaque_blob_xyz_987"
    });
    let extracted = behavior
        .extract_reasoning_opaque(&api_response)
        .expect("reasoning_opaque must be extracted from response");

    // @step Then the next turn request should include the previous "reasoning_opaque" value unchanged
    let mut next_request = json!({
        "model": "gpt-5",
        "input": [{ "role": "user", "content": "Follow up" }]
    });
    behavior.inject_reasoning_opaque(&mut next_request, &extracted);
    assert_eq!(
        next_request.get("reasoning_opaque"),
        Some(&json!("opaque_blob_xyz_987")),
        "reasoning_opaque must be echoed unchanged on next turn"
    );
}

// ============================================================================
// Scenario: gpt-5-mini is excluded from the Responses API rule and
//           uses /chat/completions
// ============================================================================
#[test]
fn scenario_gpt_5_mini_excluded_from_responses_api_rule() {
    // @step Given I am logged in to github-copilot with a github.com deployment credential
    let deployment = CopilotDeploymentType::GitHubCom;
    let access_token = "ghu_test_access_token_mini";

    // @step When I select the model "gpt-5-mini" in the TUI and send a chat message
    let model_id = "gpt-5-mini";
    let endpoint = CopilotEndpointFacade::select(model_id);
    let base_url = base_url_for(&deployment);
    let body = json!({
        "model": model_id,
        "messages": [{ "role": "user", "content": "Quick summary please" }]
    });
    let classification = CopilotRequestClassifier::classify(&body);
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);

    // @step Then CopilotEndpointFacade::select("gpt-5-mini") should return Endpoint::ChatCompletions
    assert_eq!(
        endpoint,
        CopilotEndpoint::ChatCompletions,
        "gpt-5-mini MUST be explicitly excluded from the Responses API routing rule"
    );

    // @step And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    assert_eq!(
        format!("{}/chat/completions", base_url.as_str()),
        "https://api.githubcopilot.com/chat/completions"
    );

    // @step And the system prompt facade should be OpenAISystemPromptFacade
    let system_prompt_facade = system_prompt_facade_for_endpoint(endpoint);
    assert_eq!(
        system_prompt_facade.provider(),
        "openai",
        "gpt-5-mini uses /chat/completions which must use the existing OpenAISystemPromptFacade"
    );

    // @step And the request should include header "Openai-Intent: conversation-edits"
    assert_eq!(
        headers.get("openai-intent").map(|v| v.to_str().unwrap()),
        Some("conversation-edits"),
    );
}

// ============================================================================
// Scenario: Image attachment triggers Copilot-Vision-Request header on
//           claude-sonnet-4.5
// ============================================================================
#[test]
fn scenario_image_attachment_triggers_vision_request_header_on_claude_sonnet() {
    // @step Given I am logged in to github-copilot with a github.com deployment credential
    let deployment = CopilotDeploymentType::GitHubCom;
    let access_token = "ghu_test_access_token_vision";

    // @step When I select the model "claude-sonnet-4.5" in the TUI
    let model_id = "claude-sonnet-4.5";
    let endpoint = CopilotEndpointFacade::select(model_id);
    let base_url = base_url_for(&deployment);

    // @step And I attach an image and send a message asking to describe the image
    let body = json!({
        "model": model_id,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": "Describe this image" },
                {
                    "type": "image_url",
                    "image_url": { "url": "data:image/png;base64,iVBORw0KGgo=" }
                }
            ]
        }]
    });
    let classification = CopilotRequestClassifier::classify(&body);
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);
    let behavior = select_copilot_behavior_facade(model_id);

    // @step Then CopilotRequestClassifier::classify should detect is_vision = true on the request body
    assert!(
        classification.is_vision,
        "CopilotRequestClassifier must detect image_url in messages as is_vision = true"
    );

    // @step And the outgoing request should include header "Copilot-Vision-Request: true"
    assert_eq!(
        headers
            .get("copilot-vision-request")
            .map(|v| v.to_str().unwrap()),
        Some("true"),
    );

    // @step And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    assert_eq!(
        format!("{}/chat/completions", base_url.as_str()),
        "https://api.githubcopilot.com/chat/completions"
    );
    assert_eq!(endpoint, CopilotEndpoint::ChatCompletions);

    // @step And the selected behavior facade should be CopilotClaudeBehaviorFacade
    assert_eq!(behavior.family(), "claude");
}

// ============================================================================
// Scenario: Enterprise deployment routes requests to the copilot-api
//           enterprise subdomain
// ============================================================================
#[test]
fn scenario_enterprise_deployment_routes_to_copilot_api_enterprise_subdomain() {
    // @step Given I am logged in to github-copilot with an enterprise deployment credential
    // @step And the credential has enterpriseUrl set to "ghe.example.com"
    let deployment = CopilotDeploymentType::Enterprise {
        host: "ghe.example.com".to_string(),
    };
    let access_token = "ghu_enterprise_access_token_xyz";

    // @step When I select the model "gpt-4o" in the TUI and send a chat message
    let model_id = "gpt-4o";
    let base_url = base_url_for(&deployment);
    let body = json!({
        "model": model_id,
        "messages": [{ "role": "user", "content": "Hello enterprise" }]
    });
    let classification = CopilotRequestClassifier::classify(&body);
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);
    let full_url = format!("{}/chat/completions", base_url.as_str());

    // @step Then the outgoing request URL should be "https://copilot-api.ghe.example.com/chat/completions"
    assert_eq!(
        full_url, "https://copilot-api.ghe.example.com/chat/completions",
        "Enterprise deployment must route to copilot-api.<enterprise-host>"
    );

    // @step And the request should include header "Authorization: Bearer <access_token>"
    assert_eq!(
        headers
            .get(http::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(),
        format!("Bearer {access_token}")
    );

    // @step And the request should include header "Openai-Intent: conversation-edits"
    assert_eq!(
        headers.get("openai-intent").map(|v| v.to_str().unwrap()),
        Some("conversation-edits"),
    );
}

// ============================================================================
// Scenario: Agent-mode workflow sets x-initiator header to agent instead of user
// ============================================================================
#[test]
fn scenario_agent_mode_sets_x_initiator_to_agent() {
    // @step Given I am logged in to github-copilot with a github.com deployment credential
    let _deployment = CopilotDeploymentType::GitHubCom;
    let access_token = "ghu_agent_mode_token";

    // @step And codelet is running in autonomous agent mode
    // (signaled via body metadata.mode = "agent" — slice2 memo §6)

    // @step When codelet sends a chat request to the model "gpt-5-codex"
    let model_id = "gpt-5-codex";
    let body = json!({
        "model": model_id,
        "messages": [{ "role": "user", "content": "Run this task" }],
        "metadata": { "mode": "agent" }
    });
    let classification = CopilotRequestClassifier::classify(&body);
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);

    // @step Then CopilotRequestClassifier::classify should detect is_agent = true on the request body
    assert!(
        classification.is_agent,
        "CopilotRequestClassifier must detect metadata.mode == 'agent' as is_agent = true"
    );

    // @step And the outgoing request should include header "x-initiator: agent"
    assert_eq!(
        headers.get("x-initiator").map(|v| v.to_str().unwrap()),
        Some("agent"),
    );

    // @step And the outgoing request should NOT include header "x-initiator: user"
    let x_init_values: Vec<_> = headers
        .get_all("x-initiator")
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect();
    assert!(
        !x_init_values.contains(&"user"),
        "x-initiator must NOT include 'user' when running in agent mode, got: {x_init_values:?}"
    );
    assert_eq!(x_init_values.len(), 1, "x-initiator must be single-valued");
}

// ============================================================================
// Edge-case guard tests — not Gherkin scenarios, but pin pure-function
// behavior so regressions are caught.
// ============================================================================
#[test]
fn endpoint_selector_edge_cases() {
    // gpt-4 family → ChatCompletions
    assert_eq!(
        CopilotEndpointFacade::select("gpt-4"),
        CopilotEndpoint::ChatCompletions
    );
    assert_eq!(
        CopilotEndpointFacade::select("gpt-4o"),
        CopilotEndpoint::ChatCompletions
    );
    // gpt-5 family → Responses
    assert_eq!(
        CopilotEndpointFacade::select("gpt-5"),
        CopilotEndpoint::Responses
    );
    assert_eq!(
        CopilotEndpointFacade::select("gpt-5-codex"),
        CopilotEndpoint::Responses
    );
    // gpt-5-mini is the explicit exception
    assert_eq!(
        CopilotEndpointFacade::select("gpt-5-mini"),
        CopilotEndpoint::ChatCompletions
    );
    // Non-gpt → ChatCompletions
    assert_eq!(
        CopilotEndpointFacade::select("claude-sonnet-4.5"),
        CopilotEndpoint::ChatCompletions
    );
    assert_eq!(
        CopilotEndpointFacade::select("gemini-2.5-pro"),
        CopilotEndpoint::ChatCompletions
    );
}

#[test]
fn behavior_selector_dispatches_by_prefix() {
    assert_eq!(select_copilot_behavior_facade("gpt-4").family(), "gpt");
    assert_eq!(
        select_copilot_behavior_facade("gpt-5-codex").family(),
        "gpt"
    );
    assert_eq!(
        select_copilot_behavior_facade("claude-sonnet-4.5").family(),
        "claude"
    );
    assert_eq!(
        select_copilot_behavior_facade("gemini-2.5-pro").family(),
        "gemini"
    );
    // Unknown prefix defaults to gpt
    assert_eq!(select_copilot_behavior_facade("mistral-8b").family(), "gpt");
}

#[test]
fn classifier_text_only_request() {
    let body = json!({
        "model": "gpt-4",
        "messages": [{ "role": "user", "content": "plain text" }]
    });
    let c = CopilotRequestClassifier::classify(&body);
    assert!(!c.is_vision);
    assert!(!c.is_agent);
}

#[test]
fn classifier_detects_responses_api_input_image() {
    let body = json!({
        "model": "gpt-5",
        "input": [
            { "type": "input_text", "text": "What is this?" },
            { "type": "input_image", "image_url": "data:image/png;base64,abc" }
        ]
    });
    let c = CopilotRequestClassifier::classify(&body);
    assert!(
        c.is_vision,
        "input_image in /responses body must be detected"
    );
}

#[test]
fn github_dotcom_base_url_is_api_githubcopilot_com() {
    let url = base_url_for(&CopilotDeploymentType::GitHubCom);
    assert_eq!(url.as_str(), "https://api.githubcopilot.com");
}

#[test]
fn enterprise_base_url_uses_copilot_api_subdomain() {
    let url = base_url_for(&CopilotDeploymentType::Enterprise {
        host: "ghe.example.com".to_string(),
    });
    assert_eq!(url.as_str(), "https://copilot-api.ghe.example.com");
}

// ============================================================================
// Type-plumbing guard tests — these pin the exported API surface so callers
// can depend on the concrete types. Previously these lived in a
// `_shield_unused_imports` hack that muted unused-import warnings while the
// implementation was being built; with the implementation now in place they
// are real assertions.
// ============================================================================
#[test]
fn type_surface_gpt_behavior_facade_is_a_unit_struct() {
    let facade = CopilotGptBehaviorFacade;
    assert_eq!(facade.family(), "gpt");
}

#[test]
fn type_surface_claude_behavior_facade_is_a_unit_struct() {
    let facade = CopilotClaudeBehaviorFacade;
    assert_eq!(facade.family(), "claude");
}

#[test]
fn type_surface_base_url_is_constructible_through_provider() {
    let url: CopilotBaseUrl = base_url_for(&CopilotDeploymentType::GitHubCom);
    assert!(url.as_str().starts_with("https://"));
}

#[test]
fn type_surface_request_classification_default_is_user_text() {
    let c: RequestClassification = RequestClassification::default();
    assert!(!c.is_vision);
    assert!(!c.is_agent);
}

#[test]
fn type_surface_boxed_behavior_facade_is_dyn_dispatchable() {
    let boxed: Box<dyn CopilotBehaviorFacade> = Box::new(CopilotGptBehaviorFacade);
    assert_eq!(boxed.family(), "gpt");
}
