//! Feature: spec/features/scripted-codex-subscription.feature

use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
use codelet_providers::custom::{ProviderConfig, RhaiCustomProvider, ScriptLoader};
use codelet_providers::oauth::building_blocks::{dynamic_to_json_value, json_value_to_dynamic};
use codelet_providers::oauth::custom_oauth_device::persist_on_success;
use codelet_providers::oauth::engine::build_provider_engine;
use codelet_providers::oauth::script_provider::{ScriptProviderConfig, ScriptedOAuthProvider};
use codelet_providers::oauth::scripted_refreshing_client::ScriptedRefreshingClient;
use codelet_tools::ToolDefinition;
use rhai::{Dynamic, Map, Scope};
use serde_json::json;
use serial_test::serial;
use std::path::PathBuf;
use std::sync::Arc;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "fixtures/mod.rs"]
mod fixtures;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/codex")
}

fn backend() -> Result<RhaiCustomProvider, Box<dyn std::error::Error>> {
    let mut config = ProviderConfig::from_file(&bundle().join("codex.json"))?;
    config.script = bundle().join("codex.rhai").to_string_lossy().into_owned();
    Ok(RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::new(build_provider_engine("codex"))),
        "default".into(),
    )?)
}

#[tokio::test]
#[serial]
async fn authorize_and_refresh_a_codex_subscription_with_file_credentials() -> TestResult {
    // @step Given the bundled Codex script and a local OAuth issuer
    let (_home, _guard) = fixtures::setup_fspec_home()?;
    let server = MockServer::start().await;
    let jwt = fixtures::build_test_jwt("account-test");
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .and(body_string_contains(
            "client_id=app_EMoamEEZ73f0CkXaXp7hrann",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id":"device", "user_code":"CODE", "interval":5
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"error":"authorization_pending"})),
        )
        .mount(&server)
        .await;
    let config = ScriptProviderConfig {
        name: "codex".into(),
        display_name: "Codex".into(),
        script: "codex.rhai".into(),
        auth_url: Some(server.uri()),
        token_url: Some(format!("{}/oauth/token", server.uri())),
        client_id: Some("app_EMoamEEZ73f0CkXaXp7hrann".into()),
        redirect_uri: None,
        scopes: None,
        flow: Some("device_code".into()),
        credential_file: Some("codex.json".into()),
    };
    let provider = ScriptedOAuthProvider::load(&bundle().join("codex.rhai"), config)?;
    // @step When I request a device code authorize it and refresh the resulting tokens
    let device = provider.auth_start().await?;
    assert_eq!(device["user_code"].clone().into_string()?, "CODE");
    let pending = provider.poll_for_token(device.clone()).await?;
    assert_eq!(pending["status"].clone().into_string()?, "pending");
    server.reset().await;
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_code":"auth-code", "code_verifier":"verifier"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token":"access", "refresh_token":"refresh", "id_token":jwt, "expires_in":3600
        })))
        .expect(1)
        .mount(&server)
        .await;
    let tokens = provider.poll_for_token(device).await?;
    // @step Then pending authorization and successful token exchange are distinguished
    assert_eq!(tokens["status"].clone().into_string()?, "success");
    assert!(!provider.auth_needs_refresh(tokens.clone()).await?);
    persist_on_success("codex", &tokens)?;
    server.verify().await;
    server.reset().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token":"fresh", "refresh_token":"rotated", "id_token":jwt, "expires_in":3600
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut expired = tokens;
    expired.insert("expires_at".into(), Dynamic::from(0_i64));
    codelet_providers::oauth::custom_oauth::write_stored_tokens("codex", &expired)?;
    ScriptedRefreshingClient::new(&provider, "codex")
        .ensure_fresh_if_needed()
        .await?;
    // @step And form encoded tokens account identity and file credentials are preserved
    let stored = codelet_providers::oauth::custom_oauth::read_stored_tokens("codex")?
        .ok_or("tokens missing")?;
    assert_eq!(stored["account_id"].clone().into_string()?, "account-test");
    assert_eq!(stored["access_token"].clone().into_string()?, "fresh");
    let requests = server
        .received_requests()
        .await
        .ok_or("recording disabled")?;
    assert_eq!(
        requests[0].headers["content-type"],
        "application/x-www-form-urlencoded"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn build_subscription_requests_at_built_in_provider_parity() -> TestResult {
    // @step Given a bundled Codex provider with text tool history and stored subscription tokens
    let (_home, _guard) = fixtures::setup_fspec_home()?;
    let tokens = json_value_to_dynamic(&json!({"access_token":"access", "account_id":"account"}))
        .cast::<Map>();
    codelet_providers::oauth::custom_oauth::write_stored_tokens("codex", &tokens)?;
    let backend = backend()?;
    let messages = vec![
        Message::system("Instructions"),
        Message::user("Hello"),
        Message {
            role: MessageRole::Assistant,
            content: MessageContent::Parts(vec![ContentPart::ToolUse {
                id: "call_one".into(),
                name: "shell_command".into(),
                input: json!({"command":"pwd"}),
            }]),
        },
        Message {
            role: MessageRole::User,
            content: MessageContent::Parts(vec![ContentPart::tool_result_text(
                "call_one", "/project", false,
            )]),
        },
    ];
    let tools = vec![ToolDefinition {
        name: "shell_command".into(),
        description: "Shell".into(),
        input_schema: json!({"type":"object", "properties":{"command":{"type":"string"}}}),
    }];
    // @step When I build a Responses request and its headers
    let body = backend
        .invoke_build_stream_request(
            &messages,
            &tools,
            Some(json!({
                "reasoning":{"effort":"high","summary":"auto"}, "store":false
            })),
        )
        .await?;
    let headers = backend.invoke_build_headers().await?;
    // @step Then the body and subscription headers match the built-in provider
    let expected = json!({"model":"gpt-5.3-codex", "instructions":"Instructions", "stream":true,
        "store":false, "reasoning":{"effort":"high","summary":"auto"},
        "input":[{"role":"user","content":[{"type":"input_text","text":"Hello"}]},
        {"type":"function_call","call_id":"call_one","name":"shell_command","arguments":"{\"command\":\"pwd\"}"},
        {"type":"function_call_output","call_id":"call_one","output":"/project"}],
        "tools":[{"type":"function", "name":"shell_command","description":"Shell", "parameters":tools[0].input_schema}]
    });
    assert_eq!(body, expected);
    for (name, value) in
        codelet_providers::codex::codex_oauth::build_codex_headers("access", "account")
    {
        assert_eq!(headers.get(&name).ok_or("missing header")?, value.as_str());
    }
    assert_eq!(
        backend.invoke_build_url().await?,
        "https://chatgpt.com/backend-api/codex/responses"
    );
    // @step And the bundle exposes Codex native tools and base instructions
    let config = ProviderConfig::from_file(&bundle().join("codex.json"))?;
    assert_eq!(
        config
            .system_prompt
            .ok_or("missing prefix")?
            .prefix
            .as_deref(),
        Some(codelet_providers::codex::CODEX_BASE_INSTRUCTIONS)
    );
    assert_eq!(
        config.tool_style,
        codelet_providers::custom::ToolStyle::Codex
    );
    Ok(())
}

#[tokio::test]
async fn stream_subscription_output_and_reject_failures() -> TestResult {
    // @step Given the bundled script processes Responses events
    let loader = Arc::new(ScriptLoader::new(build_provider_engine("codex")));
    let ast = loader.load(&bundle().join("codex.rhai"))?;
    let mut processor = codelet_providers::custom::stream::RhaiStreamProcessor::new(
        loader.engine_arc(),
        ast.clone(),
        "codex".into(),
        Dynamic::from_map(Map::new()),
    );
    // @step When text reasoning completed tools usage and failed events arrive
    use codelet_providers::custom::stream::StreamChunk;
    let text = processor
        .process_event(&json!({"type":"response.output_text.delta","delta":"hello"}).to_string())
        .await?;
    let reasoning = processor
        .process_event(
            &json!({"type":"response.reasoning_summary_text.delta","delta":"think"}).to_string(),
        )
        .await?;
    let tool = processor.process_event(&json!({"type":"response.output_item.done", "item":{
        "type":"function_call","id":"fc_one","call_id":"call_one","name":"shell_command","arguments":"{\"command\":\"pwd\"}"
    }}).to_string()).await?;
    let completed = processor
        .process_event(
            &json!({"type":"response.completed","response":{"usage":{
                "input_tokens":10,"output_tokens":5,"output_tokens_details":{"reasoning_tokens":3},
                "input_tokens_details":{"cached_tokens":2}
            }}})
            .to_string(),
        )
        .await?;
    let failed = processor
        .process_event(
            &json!({"type":"response.failed","response":{"error":{"message":"denied"}}})
                .to_string(),
        )
        .await;
    // @step Then the common stream bridge receives output and token counts without swallowing errors
    assert!(matches!(&text[0], StreamChunk::TextDelta(value) if value=="hello"));
    assert!(matches!(&reasoning[0], StreamChunk::ReasoningDelta(value) if value=="think"));
    assert!(tool.iter().any(|chunk| matches!(chunk, StreamChunk::ToolCallComplete { id, input, .. } if id=="call_one" && input["command"]=="pwd")));
    assert!(completed.iter().any(
        |chunk| matches!(chunk, StreamChunk::UsageDelta(usage) if usage.reasoning_tokens==Some(3))
    ));
    assert!(failed.is_err());
    let parsed: Dynamic = loader.engine().call_fn(&mut Scope::new(), &ast, "parse_response", (json_value_to_dynamic(&json!({
        "status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"answer"}]}]
    })),))?;
    assert_eq!(
        dynamic_to_json_value(&parsed)["content"][0]["text"],
        "answer"
    );
    Ok(())
}
