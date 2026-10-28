//! Feature: spec/features/mandatory-tool-work-unit-provider-contract.feature
use codelet_providers::codex::custom_tools::{CustomToolRegistry, ToolRegistration};
use codelet_providers::codex::{CodexProvider, NativeCapabilities};
use codelet_providers::convert_tools_to_rig;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::{anthropic, gemini, openai};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "tool025_provider_transports/catalog.rs"]
mod catalog;
#[path = "tool025_provider_transports/custom_catalog.rs"]
mod custom_catalog;
#[path = "tool025_provider_transports/custom_transport.rs"]
mod custom_transport;
#[path = "tool025_provider_transports/merged_catalogs.rs"]
mod merged_catalogs;

fn definitions() -> Vec<codelet_tools::ToolDefinition> {
    vec![codelet_tools::ToolDefinition {
        name: "apply_patch".into(),
        description: "Apply a patch".into(),
        input_schema: json!({"type":"object", "additionalProperties":false,
            "properties":{"patch":{"type":"string"},"target_work_unit_id":{"type":"string"}},
            "required":["patch","target_work_unit_id"]}),
    }]
}

fn serialized<M: CompletionModel>(model: M) -> Result<Value, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "Prepare patch")
        .preamble("Test instructions".into())
        .max_tokens(4096)
        .tools(convert_tools_to_rig(&definitions())?)
        .build();
    Ok(serde_json::from_slice(&model.prepare_request(request)?)?)
}

fn schema(value: &Value) -> Option<&Value> {
    if value["properties"]["patch"]["type"] == "string" {
        return Some(value);
    }
    match value {
        Value::Object(object) => object.values().find_map(schema),
        Value::Array(array) => array.iter().find_map(schema),
        _ => None,
    }
}

fn assert_contract(body: &Value) -> TestResult {
    let parameters = schema(body).ok_or("missing patch schema")?;
    assert!(
        parameters["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("work_unit_id"))),
        "{body}"
    );
    assert_eq!(parameters["type"], "object");
    assert_eq!(parameters["additionalProperties"], false);
    assert_eq!(
        parameters["properties"]["target_work_unit_id"]["type"],
        "string"
    );
    assert_eq!(parameters["properties"]["work_unit_id"]["type"], "string");
    assert_eq!(parameters["properties"]["work_unit_id"]["minLength"], 1);
    assert_eq!(
        parameters["properties"]
            .as_object()
            .map(serde_json::Map::len),
        Some(3)
    );
    assert!(parameters["required"]
        .as_array()
        .is_some_and(|required| required.contains(&json!("patch"))));
    assert!(parameters["required"]
        .as_array()
        .is_some_and(|required| required.contains(&json!("target_work_unit_id"))));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn final_provider_requests_retain_the_attribution_contract() -> TestResult {
    // @step Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    // @step When the final tool-bearing provider requests are serialized
    // @step Then required harness attribution survives each provider's schema conversion
    // @step And representative strict business argument schemas remain valid
    catalog::builtins().await?;
    custom_catalog::custom_builders().await?;
    custom_transport::final_custom_provider_requests_retain_the_attribution_contract().await?;
    custom_transport::final_custom_requests_reject_destructive_or_unsupported_tool_catalogs()
        .await?;
    custom_transport::final_custom_rig_preparation_rejects_a_dropped_attribution_contract()?;
    merged_catalogs::final_provider_options_cannot_replace_attributed_tools()?;
    let openai = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model")
        .with_strict_tools();
    assert_contract(&serialized(openai)?)?;
    let responses = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    assert_contract(&serialized(responses)?)?;
    let anthropic = anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    assert_contract(&serialized(anthropic)?)?;
    let gemini = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    assert_contract(&serialized(gemini)?)?;
    let copilot = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .http_client(codelet_providers::copilot::CopilotHttpClient::new(
            "test-token".into(),
        ))
        .build()?
        .completion_model("claude-sonnet-4.5");
    assert_contract(&serialized(copilot)?)?;
    let copilot_responses = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .http_client(codelet_providers::copilot::CopilotHttpClient::new(
            "test-token".into(),
        ))
        .build()?
        .completion_model("gpt-5");
    assert_contract(&serialized(copilot_responses)?)?;
    let codex = CodexProvider::from_api_key("test-key", "test-model")?.completion_model();
    assert_contract(&serialized(codex)?)?;
    Ok(())
}

#[test]
fn raw_and_hosted_native_tools_cannot_bypass_strict_attribution() -> TestResult {
    // @step Given a Codex agent configured with raw patch and hosted search capabilities
    let registry = CustomToolRegistry::new(vec![ToolRegistration {
        namespace: None,
        name: "apply_patch".into(),
        executor: "apply_patch".into(),
        grammar: None,
        raw_input: true,
    }])?;
    let model = CodexProvider::from_api_key("test-key", "test-model")?
        .with_custom_tools(registry)
        .completion_model()
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        });
    // @step When strict attributed tools are prepared for the model
    let body = serialized(model.clone())?;
    let tools = body["tools"].as_array().ok_or("missing tools")?;
    // @step Then patch execution is exposed through a JSON function with mandatory attribution
    assert!(
        tools.iter().all(|tool| tool["type"] == "function"),
        "{body}"
    );
    // @step And hosted execution lacking an attribution channel is not exposed
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "apply_patch");
    assert_contract(&body)?;
    // @step And the harness does not silently synthesize "NONE" for model calls
    assert!(!body.to_string().contains("\"const\":\"NONE\""));
    for tools in [
        json!([{ "type":"web_search" }]),
        json!([{ "type":"custom", "name":"apply_patch", "format":{"type":"text"} }]),
        json!([{ "type":"function", "name":"unregistered", "parameters":{"type":"object"} }]),
        json!([]),
    ] {
        let request = CompletionRequestBuilder::new(model.clone(), "Prepare patch")
            .preamble("Test instructions".into())
            .tools(convert_tools_to_rig(&definitions())?)
            .additional_params(json!({"tools":tools}))
            .build();
        assert!(
            model.prepare_request(request).is_err(),
            "merged options bypassed strict catalog"
        );
    }
    let plain = CodexProvider::from_api_key("test-key", "test-model")?.completion_model();
    let mut weakened_required = body["tools"].clone();
    weakened_required[0]["parameters"]["required"] = json!(["work_unit_id"]);
    let mut changed_type = body["tools"].clone();
    changed_type[0]["parameters"]["properties"]["patch"]["type"] = json!("integer");
    let mut changed_attribution = body["tools"].clone();
    changed_attribution[0]["parameters"]["properties"]["work_unit_id"]["type"] = json!("integer");
    let mut relaxed_properties = body["tools"].clone();
    relaxed_properties[0]["parameters"]
        .as_object_mut()
        .ok_or("missing strict patch schema")?
        .remove("additionalProperties");
    for tools in [
        json!([]),
        json!([{"type":"web_search"}]),
        weakened_required,
        changed_type,
        changed_attribution,
        relaxed_properties,
    ] {
        let request = CompletionRequestBuilder::new(plain.clone(), "Prepare patch")
            .preamble("Test instructions".into())
            .tools(convert_tools_to_rig(&definitions())?)
            .additional_params(json!({"tools":tools}))
            .build();
        assert!(
            plain.prepare_request(request).is_err(),
            "unregistered options replaced attributed tools"
        );
    }
    Ok(())
}
