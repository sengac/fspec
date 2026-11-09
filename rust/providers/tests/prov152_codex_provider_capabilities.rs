//! Feature: spec/features/codex-provider-capability-updates.feature

use codelet_providers::codex::request_options::{CodexCapabilities, RequestOptionsError};
use codelet_providers::request_admission::RequestAdmissionError;
use codelet_providers::CodexProvider;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use serde_json::json;

#[tokio::test]
// @step Given a Codex provider whose capability snapshot does not advertise xhigh has an agent with explicit experimental options
// @step When the trusted capability source updates the provider snapshot
// @step Then the existing agent uses the updated snapshot on its next request
// @step And clearing the snapshot leaves explicit options unvalidated, with no capability invented
async fn scenario_provider_capability_updates_reach_existing_agents(
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = CodexProvider::from_api_key("test-key", "not-a-catalog-model")?;
    let config = json!({"reasoning":{"effort":"xhigh"},"service_tier":"unsupported-tier"});
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, Some(config));
    let explicit_options = agent
        .additional_params
        .clone()
        .ok_or("explicit options missing")?;
    let request = || {
        CompletionRequestBuilder::new(agent.model.as_ref().clone(), "Hello")
            .additional_params(explicit_options.clone())
            .build()
    };
    let option_error = |error: CompletionError| match error {
        CompletionError::RequestError(source) => match source.downcast_ref::<RequestOptionsError>()
        {
            Some(RequestOptionsError::Unsupported { option, .. }) => Some(option.clone()),
            _ => None,
        },
        _ => None,
    };
    provider.set_capabilities(Some(serde_json::from_value::<CodexCapabilities>(
        json!({"efforts":["low"]}),
    )?))?;
    let initial = agent
        .model
        .completion(request())
        .await
        .err()
        .ok_or("a snapshot without xhigh must reject it")?;
    assert_eq!(option_error(initial).as_deref(), Some("reasoning.effort"));
    provider.set_capabilities(Some(serde_json::from_value::<CodexCapabilities>(json!({
        "efforts":["xhigh"],"service_tiers":["priority"]
    }))?))?;
    let refreshed = agent
        .model
        .completion(request())
        .await
        .err()
        .ok_or("tier must remain unsupported")?;
    assert_eq!(option_error(refreshed).as_deref(), Some("service_tier"));
    provider.set_capabilities(None)?;
    let cleared = agent
        .model
        .completion(request())
        .await
        .err()
        .ok_or("the provider has no admission limits, so the request is refused")?;
    // The options pass through unvalidated and the request reaches
    // admission, which refuses it before anything is sent.
    assert!(
        matches!(&cleared, CompletionError::RequestError(source)
            if matches!(source.downcast_ref::<RequestAdmissionError>(), Some(RequestAdmissionError::MissingContextCapacity))),
        "{cleared:?}"
    );
    Ok(())
}
