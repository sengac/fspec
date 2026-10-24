//! Feature: spec/features/mandatory-tool-work-unit-provider-contract.feature
use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_providers::LlmProvider;
use codelet_test_helpers::setup_temp_directory;
use codelet_tools::ToolDefinition;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn tools() -> Vec<ToolDefinition> {
    vec![ToolDefinition {
        name: "patch".into(),
        description: "Patch files".into(),
        input_schema: json!({"type":"object","properties":{"patch":{"type":"string"}},
            "required":["patch"],"additionalProperties":false}),
    }]
}

fn provider(script: &std::path::Path) -> Result<RhaiCustomProvider, Box<dyn std::error::Error>> {
    provider_at(script, "http://127.0.0.1:1")
}

fn provider_at(
    script: &std::path::Path,
    url: &str,
) -> Result<RhaiCustomProvider, Box<dyn std::error::Error>> {
    let config: ProviderConfig = serde_json::from_value(json!({
        "name":"attribution", "display_name":"Attribution", "base_url":url,
        "script":script, "auth":{"type":"bearer","env_var":"TOOL025_UNUSED"},
        "models":{"smart":{"id":"selected"}},
        "tool_style":"openai", "api_style":"openai_chat"
    }))?;
    Ok(RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "smart".into(),
    )?)
}

fn script(builder: &str) -> String {
    format!(
        r#"
fn build_request(request) {{ {builder} }}
fn build_stream_request(request) {{ {builder} }}
fn build_headers(config) {{ #{{}} }}
fn build_url(config) {{ config.base_url }}
fn parse_response(raw) {{ #{{ content: "done", stop_reason: "end_turn" }} }}
fn parse_stream_chunk(config, data) {{ #{{ kind: "text", text: "done" }} }}
fn map_error(status, body) {{ #{{ type: "api", message: body }} }}
"#
    )
}

pub(super) async fn final_custom_provider_requests_retain_the_attribution_contract() -> TestResult {
    // @step Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    let directory = setup_temp_directory()?;
    let path = directory.path().join("valid.rhai");
    std::fs::write(&path, script("#{ tools: request.tools }"))?;
    let provider = provider(&path)?;
    // @step When the final tool-bearing provider requests are serialized
    for body in [
        provider.invoke_build_request(&[], &tools(), None).await?,
        provider
            .invoke_build_stream_request(&[], &tools(), None)
            .await?,
    ] {
        let serialized: Value = serde_json::from_slice(&serde_json::to_vec(&body)?)?;
        let schema = &serialized["tools"][0]["input_schema"];
        // @step Then required harness attribution survives each provider's schema conversion
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!("work_unit_id"))),
            "{body}"
        );
        assert_eq!(schema["properties"]["work_unit_id"]["type"], "string");
        assert_eq!(schema["properties"]["work_unit_id"]["minLength"], 1);
        assert!(schema["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("patch"))));
        assert_eq!(
            schema["properties"]
                .as_object()
                .map(|properties| properties.len()),
            Some(2)
        );
        // @step And representative strict business argument schemas remain valid
        assert_eq!(schema["additionalProperties"], false);
    }
    for (index, (builder, pointer)) in [
        ("let tools = []; for tool in request.tools { tools.push(#{ type: \"function\", function: #{ name: tool.name, parameters: tool.input_schema } }); } #{ tools: tools }", "/tools/0/function/parameters/required"),
        ("let functions = []; for tool in request.tools { functions.push(#{ name: tool.name, parameters: tool.input_schema }); } #{ tools: [#{ functionDeclarations: functions }] }", "/tools/0/functionDeclarations/0/parameters/required"),
        ("let functions = []; for tool in request.tools { functions.push(#{ name: tool.name, parametersJsonSchema: tool.input_schema }); } #{ tools: [#{ functionDeclarations: functions }] }", "/tools/0/functionDeclarations/0/parametersJsonSchema/required"),
    ].into_iter().enumerate() {
        let dialect_path = directory.path().join(format!("dialect-{index}.rhai"));
        std::fs::write(&dialect_path, script(builder))?;
        let dialect = provider_at(&dialect_path, "http://127.0.0.1:1")?;
        let body = dialect.invoke_build_stream_request(&[], &tools(), None).await?;
        assert!(body.pointer(pointer).and_then(Value::as_array)
            .is_some_and(|required| required.contains(&json!("work_unit_id"))), "{body}");
    }
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;
    let provider = provider_at(&path, &server.uri())?;
    provider.complete_with_tools(&[], &tools()).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("missing transport capture")?;
    assert_eq!(requests.len(), 1);
    let body: Value = serde_json::from_slice(&requests[0].body)?;
    // @step When the final tool-bearing provider requests are serialized
    // @step Then required harness attribution survives each provider's schema conversion
    assert!(
        body["tools"][0]["input_schema"]["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("work_unit_id"))),
        "{body}"
    );
    let model = RhaiCustomProviderModel::new(Arc::new(provider.clone()));
    let request = CompletionRequestBuilder::new(model.clone(), "Patch")
        .tools(codelet_providers::convert_tools_to_rig(&tools())?)
        .build();
    let prepared: Value = serde_json::from_slice(&model.prepare_request(request)?)?;
    assert_eq!(prepared["tools"], body["tools"]);
    let mut collisions = vec![
        body["tools"][0]["input_schema"].clone(),
        json!({"type":"object","properties":{"patch":{"type":"string"}},
            "required":["patch","work_unit_id"]}),
    ];
    for attribution in [
        body["tools"][0]["input_schema"]["properties"]["work_unit_id"].clone(),
        json!({"type":"string"}),
        json!({"type":"integer"}),
        json!({"type":"object"}),
        json!(true),
        json!(false),
    ] {
        let mut collision = tools();
        collision[0].input_schema["properties"]["work_unit_id"] = attribution;
        collisions.push(collision[0].input_schema.clone());
    }
    for input_schema in collisions {
        let mut collision = tools();
        collision[0].input_schema = input_schema;
        assert!(codelet_providers::convert_tools_to_rig(&collision).is_err());
        assert!(provider
            .invoke_build_request(&[], &collision, None)
            .await
            .is_err());
        assert!(provider
            .invoke_build_stream_request(&[], &collision, None)
            .await
            .is_err());
    }
    Ok(())
}

pub(super) async fn final_custom_requests_reject_destructive_or_unsupported_tool_catalogs(
) -> TestResult {
    // @step Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    let directory = setup_temp_directory()?;
    let mut accepted = Vec::new();
    // @step When the final tool-bearing provider requests are serialized
    for (index, mutation) in [
        "tools[0].input_schema.required = [\"work_unit_id\"];",
        "tools[0].input_schema.properties.patch.type = \"integer\";",
        "tools[0].input_schema.remove(\"additionalProperties\");",
    ]
    .into_iter()
    .enumerate()
    {
        let path = directory.path().join(format!("business-{index}.rhai"));
        let builder = format!("let tools = request.tools; {mutation} #{{ tools: tools }}");
        std::fs::write(&path, script(&builder))?;
        let provider = provider(&path)?;
        if provider
            .invoke_build_request(&[], &tools(), None)
            .await
            .is_ok()
        {
            accepted.push(format!("ordinary mutation {index}"));
        }
        if provider
            .invoke_build_stream_request(&[], &tools(), None)
            .await
            .is_ok()
        {
            accepted.push(format!("streaming mutation {index}"));
        }
        let model = RhaiCustomProviderModel::new(Arc::new(provider));
        let request = CompletionRequestBuilder::new(model.clone(), "Patch")
            .tools(codelet_providers::convert_tools_to_rig(&tools())?)
            .build();
        if model.prepare_request(request).is_ok() {
            accepted.push(format!("Rig mutation {index}"));
        }
    }
    // @step And representative strict business argument schemas remain valid
    assert!(
        accepted.is_empty(),
        "accepted destructive business schemas: {accepted:?}"
    );
    // @step Then required harness attribution survives each provider's schema conversion
    for (index, builder) in [
        "#{ tools: [] }",
        "let tools = request.tools; tools[0].input_schema.required = [\"patch\"]; #{ tools: tools }",
        "let tools = request.tools; tools[0].input_schema.properties.work_unit_id = #{ type: \"integer\" }; #{ tools: tools }",
        "let tools = request.tools; tools[0].input_schema.nullable = true; #{ tools: tools }",
        "#{ tools: [#{ type: \"custom\", name: \"patch\", format: #{ type: \"text\" } }] }",
        "#{ tools: [#{ type: \"web_search\" }] }",
        "let tools = request.tools; tools.push(#{ type: \"web_search\" }); #{ tools: tools }",
        "#{ tools: request.tools, functions: [#{ name: \"unattributed\", parameters: #{ type: \"object\" } }] }",
        "let functions = []; for tool in request.tools { functions.push(#{ name: tool.name, parameters: tool.input_schema }); } #{ tools: [#{ functionDeclarations: functions, googleSearch: #{} }] }",
        "let tools = request.tools; tools[0].parameters = tools[0].input_schema; tools[0].input_schema = #{ type: \"object\" }; #{ tools: tools }",
    ].into_iter().enumerate() {
        let path = directory.path().join(format!("invalid-{index}.rhai"));
        std::fs::write(&path, script(builder))?;
        let provider = provider(&path)?;
        assert!(provider.invoke_build_request(&[], &tools(), None).await.is_err(), "accepted destructive builder {index}");
        assert!(provider.invoke_build_stream_request(&[], &tools(), None).await.is_err(), "accepted destructive streaming builder {index}");
    }
    Ok(())
}

pub(super) fn final_custom_rig_preparation_rejects_a_dropped_attribution_contract() -> TestResult {
    // @step Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    let directory = setup_temp_directory()?;
    let path = directory.path().join("rig-invalid.rhai");
    std::fs::write(&path, script("#{ tools: [] }"))?;
    let model = RhaiCustomProviderModel::new(Arc::new(provider(&path)?));
    let request = CompletionRequestBuilder::new(model.clone(), "Patch")
        .tools(vec![rig::completion::ToolDefinition {
            name: "patch".into(),
            description: "Patch files".into(),
            parameters: json!({"type":"object","properties":{
                "patch":{"type":"string"},"work_unit_id":{"type":"string","minLength":1}},
                "required":["patch","work_unit_id"],"additionalProperties":false}),
        }])
        .build();
    assert!(
        model.prepare_request(request).is_err(),
        "Rig preparation accepted missing contract"
    );
    // @step And representative strict business argument schemas remain valid
    Ok(())
}
