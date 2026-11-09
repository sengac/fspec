//! rhai_rig_agent_keystone_tests.rs: thinking config, Rhai tool wrapper and system-prompt facade scenarios.

use super::*;

// =========================================================================
// Scenario: thinking_config supplied to create_rig_agent reaches Rhai
// build_request via request.thinking_config
// =========================================================================
#[tokio::test]
#[serial]
async fn thinking_config_reaches_rhai_build_request_via_request_thinking_config() {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    // @step Given a custom provider script whose build_request echoes its input back as the JSON body
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
    let script = format!(
        r#"
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_request(request) {{
    #{{
        messages: request.messages,
        thinking_config: request.thinking_config
    }}
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

    // @step When CustomProvider::create_rig_agent is called with thinking_config = Some({"thinking":{"type":"enabled","budget_tokens":8000}})
    // (We exercise the equivalent rig CompletionRequest that
    // create_rig_agent's `additional_params` plumbing produces — rig
    // attaches additional_params directly to the CompletionRequest.)
    let thinking = json!({"thinking": {"type": "enabled", "budget_tokens": 8000}});
    let history = OneOrMany::one(RigMessage::User {
        content: OneOrMany::one(UserContent::Text("hello".into())),
    });
    let request = CompletionRequest {
        preamble: None,
        chat_history: history,
        documents: vec![],
        tools: vec![],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: Some(thinking),
    };

    // @step And the agent processes a user prompt
    model.completion(request).await.expect("completion ok");

    // @step Then the JSON body sent to the script HTTP endpoint contains a thinking_config field with budget_tokens 8000
    let received = &server.received_requests().await.expect("received")[0];
    let body: serde_json::Value = serde_json::from_slice(&received.body).expect("json body");
    assert_eq!(body["thinking_config"]["thinking"]["budget_tokens"], 8000);
}

// =========================================================================
// Scenario: RhaiToolWrapper exposes Rhai-defined tool names and dispatches
// through default_to_internal
// =========================================================================
#[tokio::test]
#[serial]
async fn rhai_tool_wrapper_exposes_rhai_defined_name_and_dispatches_through_default_to_internal() {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    // @step Given a custom provider script that defines a tool name "read_file" with maps_to "file:read"
    // @step When CustomProvider::create_rig_agent is invoked
    // (We simulate the wrapper construction directly — CustomProvider's
    // full create_rig_agent path is exercised by the earlier scenario.)
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("p.rhai");
    fs::write(
        &script_path,
        r#"
fn build_url(config) { "" }
fn build_headers(config) { #{} }
fn build_request(request) { #{} }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { #{ kind: "ignore" } }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#,
    )
    .expect("write rhai");
    let def = RhaiToolDef {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: json!({"type":"object"}),
        maps_to: "file:read".to_string(),
    };
    let mut models = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: "model".to_string(),
            context_window: Some(1000),
            max_output_tokens: Some(100),
            supports_tools: true,
            supports_streaming: false,
            supports_thinking: false,
            supports_vision: false,
        },
    );
    let cfg = ProviderConfig {
        name: "test".to_string(),
        display_name: "t".to_string(),
        base_url: "http://x".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "K".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    let loader = Arc::new(ScriptLoader::with_default_engine());
    let adapter =
        RhaiToolFacadeAdapter::new(Arc::new(def), Arc::new(cfg), loader).expect("adapter");
    let session_id = Uuid::new_v4();
    let wrapper = RhaiToolWrapper::new(adapter, session_id);

    // @step Then the resulting agent's tool set contains a tool whose name() returns "read_file"
    assert_eq!(<RhaiToolWrapper as Tool>::name(&wrapper), "read_file");

    // @step And calling that tool with {"file_path":"/tmp/example.txt"} routes through apply_map_tool_params and then default_to_internal returning a DispatchedToolParams::File(InternalFileParams::Read{file_path:"/tmp/example.txt", ..})
    // @step And the dispatched ReadTool execution result is returned as the rig tool output
    // Set up a real file so the read actually succeeds.
    let data_tmp = TempDir::new().expect("data tmp");
    let target = data_tmp.path().join("example.txt");
    fs::write(&target, "hello world").expect("write target");
    let args = RhaiToolArgs(json!({
        "file_path": target.to_string_lossy(),
    }));
    // BUG-470: this test drives the real ReadTool without a session manager
    // (no isolation callback), so it declares unrestricted tool access the
    // way the tools crate's tests do; production keeps isolation enforced.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "the Rhai keystone test drives tools without a session manager",
    );
    let output = wrapper.call(args).await.expect("tool call ok");
    assert!(
        output.contains("hello world"),
        "expected read output to contain file contents, got: {output}"
    );
    // Keep the tmp data alive for the async call above.
    drop(data_tmp);
    drop(tmp);
}

// =========================================================================
// Scenario: System prompt facade transform_preamble is wired in as the
// agent preamble
// =========================================================================
#[tokio::test]
#[serial]
async fn system_prompt_facade_transform_preamble_is_wired_as_agent_preamble() {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    // @step Given a custom provider script that defines transform_preamble returning "PREFIX\\n${preamble}"
    let fx = ProjectFixture::new();
    let script = r#"
fn build_url(config) { "http://x" }
fn build_headers(config) { #{} }
fn build_request(request) { #{} }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { #{ kind: "ignore" } }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
fn transform_preamble(config, preamble, fspec_guidance) { "PREFIX\n" + preamble }
"#;
    fx.write_provider("my-script", script, "http://x");

    // @step When CustomProvider::create_rig_agent is called with preamble "user role text"
    let handle = CustomProvider::create_rig_agent(
        &fx.project_root,
        "my-script",
        "smart",
        Uuid::new_v4(),
        Some("user role text"),
        None,
        false, // parent agent: full custom-provider tool surface
    )
    .expect("create_rig_agent");

    // @step Then the rig::agent::Agent's preamble equals "PREFIX\\nuser role text"
    let rendered = handle
        .system_prompt_facade()
        .transform_preamble("user role text");
    assert_eq!(rendered, "PREFIX\nuser role text");
}
