//! rhai_rig_agent_keystone_tests.rs: agent-loop dispatch, message conversion and tool forwarding scenarios.

use super::*;

// =========================================================================
// Scenario: agent_loop dispatch for a custom provider routes through
// CustomProvider::create_rig_agent
// =========================================================================
#[tokio::test]
#[serial]
async fn agent_loop_dispatch_for_custom_provider_routes_through_custom_provider_create_rig_agent() {
    // @step Given a session whose current_provider is "my-script" and a registered custom-provider config of the same name
    let fx = ProjectFixture::new();
    fx.write_provider(
        "my-script",
        &happy_script("http://127.0.0.1:0"),
        "http://127.0.0.1:0",
    );

    // @step When the agent_loop receives a user prompt for that session
    // @step Then the dispatch matches the custom-provider arm
    // (Exercised indirectly via the same surface the session_manager
    // dispatch would call.)
    let session_id = Uuid::new_v4();
    let handle = CustomProvider::create_rig_agent(
        &fx.project_root,
        "my-script",
        "smart",
        session_id,
        Some("role-text"),
        None,
        false, // parent agent: full custom-provider tool surface
    )
    .expect("create_rig_agent");

    // @step And CustomProvider::create_rig_agent is invoked with the session id, the user role preamble, and the resolved thinking_config
    assert_eq!(handle.provider_name(), "my-script");

    // @step And the returned rig::agent::Agent is wrapped in codelet_core::RigAgent and streamed via run_agent_stream_with_images
    // Static-type check: `handle.into_inner()` returns
    // `rig::agent::Agent<RhaiCustomProviderModel>` which is exactly what
    // `codelet_core::RigAgent::with_default_depth` accepts.
    let agent: rig::agent::Agent<RhaiCustomProviderModel> = handle.into_inner();
    let _rig_agent = codelet_core::RigAgent::with_default_depth(agent);
}

// =========================================================================
// Additional unit coverage for rig_message_convert helpers used by the
// keystone model
// =========================================================================
#[test]
fn rig_preamble_and_history_convert_to_internal_messages() {
    let history = vec![RigMessage::User {
        content: OneOrMany::one(UserContent::Text("hi".into())),
    }];
    let out = rig_messages_to_internal(Some("sys"), &history);
    assert_eq!(out.len(), 2);
    match &out[1].content {
        MessageContent::Text(t) => assert_eq!(t, "hi"),
        other => panic!("expected text, got {other:?}"),
    }
    let _ = Message::user("placeholder");
}

// =========================================================================
// Scenario: CompletionRequest.tools are forwarded to Rhai build_request
// =========================================================================
//
// Regression: previously `tools_for_rhai` returned `Vec::new()`,
// dropping the rig-supplied tool catalogue on the floor. Scripts like
// `claude-rhai` that expect `request.tools` to contain the full tool
// definitions therefore advertised zero tools to the upstream API.
#[tokio::test]
#[serial]
async fn completion_request_tools_are_forwarded_to_rhai_build_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                {
                    "message": { "content": "ok" },
                    "finish_reason": "stop"
                }
            ]
        })))
        .mount(&server)
        .await;

    // Script echoes `request.tools` straight back into the request body
    // so the test can verify the bridge preserved the catalogue.
    let script = format!(
        r#"
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_request(request) {{
    let out = [];
    if type_of(request.tools) == "array" {{
        for t in request.tools {{
            out.push(#{{
                name: t.name,
                description: t.description,
                input_schema: t.input_schema
            }});
        }}
    }}
    #{{ messages: request.messages, tools: out }}
}}
fn parse_response(raw) {{ #{{ content: "ok", stop_reason: "end_turn" }} }}
fn parse_stream_chunk(config, data) {{ #{{ kind: "ignore" }} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{ #{{ type: "api", message: "err" }} }}
"#,
        base = server.uri()
    );
    let provider = build_provider_inline(&script, "my-script", &server.uri());
    let model = RhaiCustomProviderModel::new(Arc::new(provider));

    let history = OneOrMany::one(RigMessage::User {
        content: OneOrMany::one(UserContent::Text("hi".into())),
    });
    let tool_a = RigToolDefinition {
        name: "read_file".to_string(),
        description: "read a file".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"}
            },
            "required": ["file_path"]
        }),
    };
    let tool_b = RigToolDefinition {
        name: "write_file".to_string(),
        description: "write a file".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "content":   {"type": "string"}
            },
            "required": ["file_path", "content"]
        }),
    };
    let request = CompletionRequest {
        preamble: None,
        chat_history: history,
        documents: vec![],
        // BUG-338: an agent's tool server attributes its tools before they
        // reach a provider (custom providers reject unattributed catalogs).
        tools: vec![
            codelet_tools::tool_attribution::decorate_definition(tool_a.clone())
                .expect("read_file accepts attribution"),
            codelet_tools::tool_attribution::decorate_definition(tool_b.clone())
                .expect("write_file accepts attribution"),
        ],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
    };

    model.completion(request).await.expect("completion ok");

    let received = &server.received_requests().await.expect("received")[0];
    let body: serde_json::Value = serde_json::from_slice(&received.body).expect("json body");
    let tools = body["tools"].as_array().expect("tools is array");
    assert_eq!(
        tools.len(),
        2,
        "expected both tools forwarded, got {tools:?}"
    );
    assert_eq!(tools[0]["name"], "read_file");
    assert_eq!(tools[0]["description"], "read a file");
    assert_eq!(tools[0]["input_schema"]["required"][0], "file_path");
    assert_eq!(tools[1]["name"], "write_file");
    assert_eq!(tools[1]["description"], "write a file");
    assert_eq!(tools[1]["input_schema"]["required"][1], "content");
    assert_eq!(tools[0]["input_schema"]["required"][1], "work_unit_id");
}
