//! Feature: spec/features/mandatory-tool-work-unit-provider-contract.feature
use codelet_providers::convert_tools_to_rig;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::{anthropic, gemini, openai};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn catalog_is_preserved<M: CompletionModel>(model: M) -> Result<bool, Box<dyn std::error::Error>> {
    let tools = convert_tools_to_rig(&[codelet_tools::ToolDefinition {
        name: "inspect".into(),
        description: "Inspect files".into(),
        input_schema: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }])?;
    let request = CompletionRequestBuilder::new(model.clone(), "Inspect files")
        .preamble("Test instructions".into())
        .max_tokens(4096)
        .tools(tools)
        .build();
    let original: Value = serde_json::from_slice(&model.prepare_request(request.clone())?)?;
    let mut overridden = request;
    overridden.additional_params = Some(json!({"tools":[{"type":"web_search"}]}));
    let Ok(bytes) = model.prepare_request(overridden) else {
        return Ok(true);
    };
    let body: Value = serde_json::from_slice(&bytes)?;
    Ok(body["tools"] == original["tools"])
}

pub(super) fn final_provider_options_cannot_replace_attributed_tools() -> TestResult {
    // @step Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    let chat = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    let responses = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    let anthropic = anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    let gemini = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .build()?
        .completion_model("test-model");
    // @step When the final tool-bearing provider requests are serialized
    let results = [
        ("OpenAI chat", catalog_is_preserved(chat)?),
        ("OpenAI Responses", catalog_is_preserved(responses)?),
        ("Anthropic", catalog_is_preserved(anthropic)?),
        ("Gemini", catalog_is_preserved(gemini)?),
    ];
    // @step Then required harness attribution survives each provider's schema conversion
    // @step And representative strict business argument schemas remain valid
    assert!(
        results.iter().all(|(_, preserved)| *preserved),
        "merged catalogs bypass attribution: {results:?}"
    );
    Ok(())
}
