//! PROV-163: the GitHub Copilot HTTP backend is verified against current
//! Copilot behaviour.
//!
//! Feature: spec/features/copilot-http-backend-verification.feature

use codelet_providers::copilot::models::{build_catalog_from_response, CopilotModelsResponse};
use codelet_providers::copilot::{read_copilot_auth, CopilotDeploymentType, CopilotProvider};
use codelet_providers::LlmProvider;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn entry(id: &str, policy: Option<&str>) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": id,
        "name": id,
        "version": format!("{id}-2026-01-01"),
        "model_picker_enabled": true,
        "capabilities": {
            "family": "any-family",
            "limits": {
                "max_context_window_tokens": 100,
                "max_output_tokens": 50,
                "max_prompt_tokens": 100
            },
            "supports": {"streaming": true, "tool_calls": true, "vision": false}
        }
    });
    if let Some(state) = policy {
        value["policy"] = serde_json::json!({ "state": state, "terms": "terms" });
    }
    value
}

#[test]
fn policy_disabled_models_are_hidden_from_the_copilot_catalog() -> TestResult {
    // @step Given a Copilot models response with one enabled, one policy-disabled and one policy-less picker model
    let response: CopilotModelsResponse = serde_json::from_value(serde_json::json!({
        "data": [
            entry("gpt-x", Some("enabled")),
            entry("gpt-y", Some("disabled")),
            entry("gpt-z", None),
        ]
    }))?;

    // @step When the catalog is built
    let ids: Vec<String> = build_catalog_from_response(&response)?
        .into_iter()
        .map(|model| model.id)
        .collect();

    // @step Then the enabled and the policy-less models are listed
    assert_eq!(ids, ["gpt-x", "gpt-z"]);

    // @step And the policy-disabled model is not listed
    assert!(!ids.iter().any(|id| id == "gpt-y"));
    Ok(())
}

#[tokio::test]
#[ignore = "live: needs a GitHub Copilot subscription; log in with /login, then run with FSPEC_COPILOT_LIVE=1 and --ignored"]
async fn the_live_copilot_backend_completes_a_request_with_stored_credentials() -> TestResult {
    // @step Given stored fspec GitHub Copilot credentials and FSPEC_COPILOT_LIVE set
    if std::env::var_os("FSPEC_COPILOT_LIVE").is_none() {
        return Err("set FSPEC_COPILOT_LIVE=1 to run the live Copilot smoke test".into());
    }
    let auth = read_copilot_auth()
        .await?
        .ok_or("no stored GitHub Copilot credentials (log in with /login first)")?;
    let deployment = match auth.enterprise_url.clone() {
        Some(host) if !host.is_empty() => CopilotDeploymentType::Enterprise { host },
        _ => CopilotDeploymentType::GitHubCom,
    };
    let provider = CopilotProvider::from_auth(deployment, auth, "gpt-4o")?;

    // @step When the backend exchanges its token, lists models and sends one short completion
    provider.ensure_fresh_copilot_token().await?;
    let token = provider
        .auth_snapshot()
        .await
        .copilot_token
        .ok_or("token exchange returned no Copilot token")?;
    let models =
        codelet_providers::copilot::models::fetch_models(provider.base_url().as_str(), &token)
            .await?;
    let reply = provider
        .complete(&[codelet_common::Message::user("Reply with the word pong.")])
        .await?;

    // @step Then the token exchange succeeds, at least one model is listed and the completion returns text
    assert!(!token.is_empty());
    assert!(
        !models.is_empty(),
        "the Copilot models endpoint listed nothing"
    );
    assert!(!reply.trim().is_empty(), "the completion returned no text");
    Ok(())
}
