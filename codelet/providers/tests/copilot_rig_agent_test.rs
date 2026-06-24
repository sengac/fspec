#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/github-copilot-end-to-end-integration.feature
//!
//! PROV-057 Layer 1/2 integration seam tests — these tests verify that
//! `CopilotProvider` exposes the two methods required by the NAPI agent
//! loop dispatch macro (`run_with_provider!`) and the DeepSearch sub-agent
//! build path (`build_and_run!`):
//!
//! | Method                  | Consumer                                                          |
//! |-------------------------|-------------------------------------------------------------------|
//! | `client()`              | `codelet/napi/src/deep_search_handler.rs` `build_and_run!` macro  |
//! | `create_rig_agent(...)` | `codelet/napi/src/session_manager.rs` `run_with_provider!` macro  |
//!
//! These are infrastructure seams, not scenarios — the end-to-end scenarios
//! that exercise them live under Agent B's scope:
//!
//! - "Agent loop dispatches github-copilot to CopilotProvider"
//! - "DeepSearch sub-agents can use github-copilot as their provider"
//!
//! Both of those scenarios call through to these methods, so this file
//! pins down the single-responsibility contract each method must satisfy.

use codelet_providers::copilot::{
    CopilotAuthJson, CopilotDeploymentType, CopilotHttpClient, CopilotProvider,
};
use codelet_providers::LlmProvider;
use rig::providers::openai;
use uuid::Uuid;

/// Build a `CopilotProvider` with an already-cached (non-expiring) Copilot
/// token so tests never touch the network.
fn test_provider() -> CopilotProvider {
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_test_long_lived".to_string(),
        copilot_token: Some("tid=test_cached;:sig".to_string()),
        copilot_token_expires_at: Some(9_999_999_999),
        endpoints_api: None,
        enterprise_url: None,
    };
    CopilotProvider::from_auth(CopilotDeploymentType::GitHubCom, auth, "gpt-4o-copilot")
        .expect("provider should construct with valid test auth")
}

// =========================================================================
// client() accessor
// =========================================================================

#[test]
fn client_accessor_returns_rig_completions_client_parameterised_by_copilot_http_client() {
    // @step Given a constructed CopilotProvider
    let provider = test_provider();

    // @step When I call provider.client()
    // Type-level assertion: the returned reference must be of type
    // `&openai::CompletionsClient<CopilotHttpClient>` so the
    // `build_and_run!` macro's `.agent(...)` call dispatches through the
    // Copilot HTTP middleware.
    let _client: &openai::CompletionsClient<CopilotHttpClient> = provider.client();

    // @step Then the accessor is callable without side effects
    // (Type-check above is the primary assertion — if this compiles, the
    // contract is satisfied.)
}

#[test]
fn client_accessor_supports_building_an_agent_builder_with_the_providers_model_name() {
    use rig::client::CompletionClient;

    // @step Given a constructed CopilotProvider
    let provider = test_provider();

    // @step When I call provider.client().agent(provider.model())
    // This is the exact call chain used by the NAPI DeepSearch build_and_run!
    // macro at codelet/napi/src/deep_search_handler.rs:258.
    let _agent_builder = provider.client().agent(provider.model());

    // @step Then the agent builder is constructed successfully without
    //       panicking or taking a generic that conflicts with
    //       `CompletionModel<CopilotHttpClient>`.
}

// =========================================================================
// create_rig_agent() method
// =========================================================================

#[tokio::test]
async fn create_rig_agent_returns_an_agent_specialised_to_copilot_http_client() {
    // @step Given a constructed CopilotProvider
    let provider = test_provider();
    let session_id = Uuid::new_v4();

    // @step When I call create_rig_agent with no preamble and no thinking config
    // Type-level assertion: the returned agent must be specialised to
    // `CompletionModel<CopilotHttpClient>` so the `run_with_provider!`
    // macro can dispatch it without an "Unsupported provider" error.
    let _agent: rig::agent::Agent<openai::completion::CompletionModel<CopilotHttpClient>> =
        provider.create_rig_agent(session_id, None, None);

    // @step Then the method is callable without panicking
}

#[tokio::test]
async fn create_rig_agent_accepts_preamble_text() {
    // @step Given a constructed CopilotProvider
    let provider = test_provider();
    let session_id = Uuid::new_v4();

    // @step When I call create_rig_agent with a preamble
    let _agent = provider.create_rig_agent(
        session_id,
        Some("You are a helpful assistant for testing."),
        None,
    );

    // @step Then the method accepts the preamble and returns successfully
}

#[tokio::test]
async fn create_rig_agent_accepts_thinking_config_parameter() {
    // @step Given a constructed CopilotProvider
    let provider = test_provider();
    let session_id = Uuid::new_v4();

    // @step When I call create_rig_agent with a thinking config JSON value
    // Thinking config is currently unused for Copilot's OpenAI-compatible
    // endpoint but the signature must accept it for parity with the
    // `run_with_provider!` macro's call site.
    let thinking = serde_json::json!({
        "thinking": { "type": "enabled", "budget_tokens": 1024 }
    });
    let _agent = provider.create_rig_agent(session_id, None, Some(thinking));

    // @step Then the method accepts the parameter without erroring
}

#[tokio::test]
async fn create_rig_agent_is_callable_for_an_enterprise_provider() {
    // @step Given a CopilotProvider constructed from an enterprise deployment
    let auth = CopilotAuthJson {
        github_oauth_token: "gho_enterprise".to_string(),
        copilot_token: Some("tid=enterprise_cached;:sig".to_string()),
        copilot_token_expires_at: Some(9_999_999_999),
        endpoints_api: Some("https://copilot-api.ghe.example.com".to_string()),
        enterprise_url: Some("ghe.example.com".to_string()),
    };
    let provider = CopilotProvider::from_auth(
        CopilotDeploymentType::Enterprise {
            host: "ghe.example.com".to_string(),
        },
        auth,
        "gpt-4o-copilot",
    )
    .expect("enterprise provider should construct");
    let session_id = Uuid::new_v4();

    // @step When I call create_rig_agent
    let _agent = provider.create_rig_agent(session_id, Some("test preamble"), None);

    // @step Then the agent is built against the endpoints_api URL carried
    //       in the auth file (copilot-api.ghe.example.com), not the
    //       statically computed deployment URL.
    assert_eq!(
        provider.base_url().as_str(),
        "https://copilot-api.ghe.example.com",
        "enterprise provider must honour endpoints_api so create_rig_agent \
         wires requests through the correct host"
    );
}
