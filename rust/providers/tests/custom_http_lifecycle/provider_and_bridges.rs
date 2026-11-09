//! custom_http_lifecycle_tests.rs: provider name, context window and bridge round-trip scenarios.

use super::*;

// =========================================================================
// Scenario: Provider name reflects config
// =========================================================================
#[tokio::test]
async fn provider_name_reflects_config() {
    // @step Given a ProviderConfig with name "my-llm"
    let (_tmp, cfg) = config_with_full_script(
        "my-llm",
        "https://api.example.com",
        "model-smart-v2",
        FULL_HAPPY_SCRIPT,
    );

    // @step When I construct a RhaiCustomProvider from that config
    let provider = build_provider(cfg, "smart");

    // @step Then provider.name() returns "my-llm"
    assert_eq!(provider.name(), "my-llm");
}

// =========================================================================
// Scenario: Provider context window reflects selected model
// =========================================================================
#[tokio::test]
async fn provider_context_window_reflects_selected_model() {
    // @step Given a config with a model "big" defining context_window 200000 and max_output_tokens 8192
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let script_path = helpers::write_script(tmp.path(), "p.rhai", FULL_HAPPY_SCRIPT);

    let mut models = std::collections::HashMap::new();
    models.insert(
        "big".to_string(),
        codelet_providers::custom::ModelDef {
            id: "model-big".to_string(),
            context_window: Some(200_000),
            max_output_tokens: Some(8_192),
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    let cfg = ProviderConfig {
        name: "my-llm".to_string(),
        display_name: "My LLM".to_string(),
        base_url: "https://api.example.com".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: codelet_providers::custom::AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: std::collections::HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };

    // @step When I construct a RhaiCustomProvider selecting model "big"
    let provider = build_provider(cfg, "big");

    // @step Then provider.context_window() equals 200000 and provider.max_output_tokens() equals 8192
    assert_eq!(provider.context_window(), Some(200_000));
    assert_eq!(provider.max_output_tokens(), Some(8_192));
}

// =========================================================================
// Scenario: Request bridge preserves multi-turn message structure
// =========================================================================
#[tokio::test]
async fn request_bridge_preserves_multi_turn_message_structure() {
    // @step Given a conversation with user then assistant then user turns
    let messages = vec![
        Message::user("first"),
        Message::assistant("second"),
        Message::user("third"),
    ];

    // @step When I convert the messages through the request_bridge
    let dyn_value: Dynamic = messages_to_rhai(&messages).expect("messages_to_rhai");
    let array = dyn_value
        .into_typed_array::<Dynamic>()
        .expect("resulting Dynamic is a Rhai Array");

    // @step Then the resulting Rhai array has three entries in the correct order with matching roles and contents
    assert_eq!(array.len(), 3);

    let expect = [
        ("user", "first"),
        ("assistant", "second"),
        ("user", "third"),
    ];
    for (i, (role, content)) in expect.iter().enumerate() {
        let m = array[i]
            .clone()
            .try_cast::<Map>()
            .expect("each array entry is a Map");
        let actual_role = m
            .get("role")
            .cloned()
            .unwrap_or(Dynamic::UNIT)
            .into_string()
            .expect("role is string");
        assert_eq!(&actual_role, role);
        let actual_content = m
            .get("content")
            .cloned()
            .unwrap_or(Dynamic::UNIT)
            .into_string()
            .expect("content is string");
        assert_eq!(&actual_content, content);
    }
}

// =========================================================================
// Scenario: Response bridge preserves structured tool call input
// =========================================================================
#[tokio::test]
async fn response_bridge_preserves_structured_tool_call_input() {
    // @step Given a Rhai response map with tool_call input {path:"a.txt", mode:"read"}
    let mut input_map = Map::new();
    input_map.insert("path".into(), Dynamic::from("a.txt".to_string()));
    input_map.insert("mode".into(), Dynamic::from("read".to_string()));

    let mut tool_use_part = Map::new();
    tool_use_part.insert("type".into(), Dynamic::from("tool_use".to_string()));
    tool_use_part.insert("id".into(), Dynamic::from("call_1".to_string()));
    tool_use_part.insert("name".into(), Dynamic::from("read_file".to_string()));
    tool_use_part.insert("input".into(), Dynamic::from_map(input_map));

    let content_array: rhai::Array = vec![Dynamic::from_map(tool_use_part)];

    let mut response_map = Map::new();
    response_map.insert("content".into(), Dynamic::from_array(content_array));
    response_map.insert("stop_reason".into(), Dynamic::from("tool_use".to_string()));

    let response_dynamic = Dynamic::from_map(response_map);

    // @step When I convert it through the response_bridge
    let (response, _usage) =
        rhai_to_completion_response(response_dynamic).expect("response_bridge succeeds");

    // @step Then the ToolUseContent input is a serde_json::Value object with fields path="a.txt" and mode="read"
    let parts = match &response.content {
        MessageContent::Parts(p) => p,
        MessageContent::Text(_) => panic!("expected Parts, got Text"),
    };
    assert_eq!(parts.len(), 1);
    match &parts[0] {
        ContentPart::ToolUse { id, name, input } => {
            assert_eq!(id, "call_1");
            assert_eq!(name, "read_file");
            let obj = input.as_object().expect("input is JSON object");
            assert_eq!(obj.get("path").and_then(|v| v.as_str()), Some("a.txt"));
            assert_eq!(obj.get("mode").and_then(|v| v.as_str()), Some("read"));
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
    assert_eq!(response.stop_reason, StopReason::ToolUse);
}
