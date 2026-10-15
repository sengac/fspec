#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/codex-provider-live-completion.feature
//! Feature: spec/features/codex-models-silently-terminate-rig-swallows-sse-error-events-and-astgreprefactor-tool-schema-is-rejected.feature
//! (Scenario: "Live gpt-6-astra agent conversation with all production tools produces a response")
//!
//! Live integration test: uses the REAL OAuth credentials from ~/.codex/auth.json
//! (the same token system the CodexProvider uses in production) and performs an
//! actual completion against chatgpt.com/backend-api/codex/responses with
//! gpt-6-astra — the model that silently terminates conversations in the TUI.
//!
//! Run with:
//!   cargo test -p codelet-providers --test codex_gpt6_astra_live_test -- --nocapture

use codelet_common::Message;
use codelet_providers::codex::CodexProvider;
use codelet_providers::LlmProvider;

/// Skip (return None) when no usable Codex OAuth credentials exist on this machine.
fn real_oauth_creds() -> Option<(String, String, String)> {
    let auth = codelet_providers::codex::codex_auth::read_codex_auth().ok()??;
    let tokens = auth.tokens?;
    if tokens.access_token.is_empty() || tokens.account_id.is_empty() {
        return None;
    }
    Some((tokens.access_token, tokens.refresh_token, tokens.account_id))
}

/// Build a CodexProvider from real OAuth tokens (forcing a refresh, since the
/// tokens on disk are of unknown age).
fn provider_with_tokens(
    access: &str,
    refresh: &str,
    account_id: &str,
    model: &str,
) -> CodexProvider {
    CodexProvider::from_oauth_tokens(
        access,
        refresh,
        account_id,
        Some(0),
        codelet_providers::codex::codex_oauth::CODEX_ISSUER,
        model,
    )
    .expect("provider should initialize with real OAuth tokens")
}

/// Scenario: the Codex backend REQUIRES streaming — the non-streaming path is
/// rejected with 400 "Stream must be set to true". This documents why the TUI
/// must use the streaming path (and why a silently-empty stream ends a
/// conversation with no error surfaced).
#[tokio::test]
async fn live_gpt6_astra_non_streaming_is_rejected() {
    let (access, refresh, account_id) = match real_oauth_creds() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: no Codex OAuth credentials in ~/.codex/auth.json");
            return;
        }
    };

    let model = std::env::var("CODEX_TEST_MODEL")
        .unwrap_or_else(|_| "gpt-6-astra".to_string());

    let provider = provider_with_tokens(&access, &refresh, &account_id, &model);

    let messages = vec![Message::user("Reply with exactly the single word: pong")];

    let result = provider.complete(&messages).await;

    match &result {
        Ok(text) => println!("OK response: {text:?}"),
        Err(e) => eprintln!("ERROR: {e:?}"),
    }

    let err = result.expect_err("Codex backend rejects non-streaming requests");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("Stream must be set to true") || msg.contains("stream"),
        "expected a stream-required 400, got: {msg}"
    );
}

/// Scenario: gpt-6-astra answers a simple prompt via the STREAMING path — the
/// exact path the TUI agent loop uses (rig responses_api /responses SSE).
/// If this stream yields no text, that is the silent-termination bug.
#[tokio::test]
async fn live_gpt6_astra_streaming_completion() {
    // Surface rig's debug lines about SSE events it fails to deserialize —
    // silently-skipped events are the prime suspect for empty streams.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rig=debug,warn".parse().unwrap()),
        )
        .with_test_writer()
        .try_init()
        .ok();

    let (access, refresh, account_id) = match real_oauth_creds() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: no Codex OAuth credentials in ~/.codex/auth.json");
            return;
        }
    };

    let model_name = std::env::var("CODEX_TEST_MODEL")
        .unwrap_or_else(|_| "gpt-6-astra".to_string());

    let provider = provider_with_tokens(&access, &refresh, &account_id, &model_name);

    use rig::client::CompletionClient;
    let model = provider.client().completion_model(&model_name);

    // Mirror production request params (build_reasoning_params is private, so
    // inline the same JSON the agent path sends).
    let resp = rig::completion::CompletionRequestBuilder::new(
        model,
        "Reply with exactly the single word: pong",
    )
    .preamble(codelet_providers::codex::CODEX_BASE_INSTRUCTIONS.to_string())
    .additional_params(serde_json::json!({
        "store": false,
        "include": ["reasoning.encrypted_content"],
        "reasoning": { "effort": "low", "summary": "auto" },
    }))
    .stream()
    .await
    .expect("stream request should be accepted (stream:true)");

    use futures::StreamExt;
    use rig::streaming::StreamedAssistantContent;

    let mut stream = resp;
    let mut text = String::new();
    let mut chunk_count = 0usize;
    while let Some(item) = stream.next().await {
        chunk_count += 1;
        match item {
            Ok(StreamedAssistantContent::Text(t)) => {
                println!("text delta: {:?}", t.text);
                text.push_str(&t.text);
            }
            Ok(other) => println!("other chunk: {other:?}"),
            Err(e) => eprintln!("stream error: {e:?}"),
        }
    }

    println!("total chunks received: {chunk_count}");

    assert!(
        chunk_count > 0,
        "stream produced ZERO chunks — backend sent events rig could not parse, or the SSE stream ended immediately (silent-termination bug)"
    );
    assert!(
        !text.trim().is_empty(),
        "stream produced chunks but no text — this is the silent-termination bug"
    );
}

/// Scenario: reproduce the TUI failure — same streaming path but WITH tool
/// definitions in the request (the TUI registers ~25 tools). gpt-6-astra has
/// tool_mode:code_mode_only in codex's models.json, so with tools present the
/// backend is expected to emit `custom_tool_call` items /
/// `response.custom_tool_call_input.delta` events that rig's responses_api
/// cannot parse → silently skipped → empty stream → conversation terminates.
#[tokio::test]
async fn live_gpt6_astra_streaming_with_tools() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rig=debug,warn".parse().unwrap()),
        )
        .with_test_writer()
        .try_init()
        .ok();

    let (access, refresh, account_id) = match real_oauth_creds() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: no Codex OAuth credentials in ~/.codex/auth.json");
            return;
        }
    };

    let model_name = std::env::var("CODEX_TEST_MODEL")
        .unwrap_or_else(|_| "gpt-6-astra".to_string());

    let provider = provider_with_tokens(&access, &refresh, &account_id, &model_name);

    use rig::client::CompletionClient;
    let model = provider.client().completion_model(&model_name);

    // A dummy tool mirroring the shape of production tool definitions.
    let tool = rig::completion::ToolDefinition {
        name: "shell_command".to_string(),
        description: "Run a shell command and return its output".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Command to run" }
            },
            "required": ["command"]
        }),
    };

    let resp = rig::completion::CompletionRequestBuilder::new(
        model,
        "Use the shell_command tool to run: echo pong. Then tell me the output.",
    )
    .preamble(codelet_providers::codex::CODEX_BASE_INSTRUCTIONS.to_string())
    .tool(tool)
    .additional_params(serde_json::json!({
        "store": false,
        "include": ["reasoning.encrypted_content"],
        "reasoning": { "effort": "low", "summary": "auto" },
    }))
    .stream()
    .await
    .expect("stream request with tools should be accepted (stream:true)");

    use futures::StreamExt;
    use rig::streaming::StreamedAssistantContent;

    let mut stream = resp;
    let mut text = String::new();
    let mut tool_calls = 0usize;
    let mut chunk_count = 0usize;
    while let Some(item) = stream.next().await {
        chunk_count += 1;
        match item {
            Ok(StreamedAssistantContent::Text(t)) => {
                println!("text delta: {:?}", t.text);
                text.push_str(&t.text);
            }
            Ok(StreamedAssistantContent::ToolCall(tc)) => {
                tool_calls += 1;
                println!("tool call chunk: {tc:?}");
            }
            Ok(other) => println!("other chunk: {other:?}"),
            Err(e) => eprintln!("stream error: {e:?}"),
        }
    }

    println!(
        "WITH-TOOLS RESULT: chunks={chunk_count} text_len={} tool_calls={tool_calls}",
        text.trim().len()
    );

    // Diagnostic (not yet an assertion on text): the bug is reproduced when the
    // model was asked to use a tool but we observe NEITHER text NOR tool calls.
    assert!(
        chunk_count > 0,
        "stream produced ZERO chunks with tools — silent-termination bug reproduced at chunk level"
    );
    assert!(
        !text.trim().is_empty() || tool_calls > 0,
        "SILENT-TERMINATION BUG REPRODUCED: model was asked to use a tool but stream yielded no text and no parseable tool calls (code-mode custom_tool_call events likely dropped by rig)"
    );
}

/// Scenario: reproduce the TUI failure via the FULL production agent path —
/// `create_rig_agent` (all ~25 Codex-native tools) + `stream_prompt(...).multi_turn()`,
/// exactly what `RigAgent::prompt_streaming_with_history` does in the TUI.
#[tokio::test]
async fn live_gpt6_astra_full_agent_path() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rig=debug,warn,codelet_providers=debug".parse().unwrap()),
        )
        .with_test_writer()
        .try_init()
        .ok();

    // @step Given real Codex OAuth credentials and the gpt-6-astra model
    let (access, refresh, account_id) = match real_oauth_creds() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: no Codex OAuth credentials in ~/.codex/auth.json");
            return;
        }
    };

    let model_name = std::env::var("CODEX_TEST_MODEL")
        .unwrap_or_else(|_| "gpt-6-astra".to_string());

    let provider = provider_with_tokens(&access, &refresh, &account_id, &model_name);

    // @step And the full production agent with all Codex-native tools registered (including AstGrepRefactor)
    // Full production agent: all Codex-native tool facades registered.
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);

    use futures::StreamExt;
    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::StreamingPrompt;

    // @step When the agent is prompted "Use the shell_command tool to run: echo pong. Then tell me the output." with multi-turn enabled
    let mut stream = agent
        .stream_prompt("Use the shell_command tool to run: echo pong. Then tell me the output.")
        .multi_turn(8)
        .await;

    let mut text = String::new();
    let mut tool_call_events = 0usize;
    let mut item_count = 0usize;
    let mut final_text_len: Option<usize> = None;
    let mut seen_errors: Vec<String> = Vec::new();

    while let Some(item) = stream.next().await {
        item_count += 1;
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(chunk)) => {
                use rig::streaming::StreamedAssistantContent;
                match chunk {
                    StreamedAssistantContent::Text(t) => {
                        println!("text delta: {:?}", t.text);
                        text.push_str(&t.text);
                    }
                    StreamedAssistantContent::ToolCall(tc) => {
                        tool_call_events += 1;
                        println!("agent tool call: {:?}", tc.function.name);
                    }
                    other => println!("other chunk: {other:?}"),
                }
            }
            Ok(MultiTurnStreamItem::StreamUserItem(_)) => {
                println!("tool result fed back to model (multi-turn step)");
            }
            Ok(MultiTurnStreamItem::FinalResponse(fr)) => {
                final_text_len = Some(fr.response().len());
                println!("FINAL RESPONSE len={}: {:?}", fr.response().len(), fr.response());
            }
            Ok(other) => println!("other multi-turn item: {other:?}"),
            Err(e) => {
                let msg = format!("{e}");
                println!("agent stream error: {msg}");
                seen_errors.push(msg);
            }
        }
    }

    println!(
        "FULL-AGENT RESULT: items={item_count} streamed_text_len={} tool_call_events={tool_call_events} final_len={final_text_len:?}",
        text.trim().len()
    );

    // @step Then the agent stream produces tool calls and/or streamed text
    // @step And the final response is non-empty
    assert!(
        item_count > 0,
        "agent stream produced ZERO items — silent-termination bug reproduced on the full agent path"
    );
    assert!(
        final_text_len.is_some_and(|l| l > 0) || !text.trim().is_empty(),
        "SILENT-TERMINATION BUG REPRODUCED on full agent path: no final text and no streamed text"
    );
    // @step And no invalid_function_parameters error occurs
    assert!(
        !seen_errors.iter().any(|e| e.contains("invalid_function_parameters")),
        "backend rejected a tool schema: invalid_function_parameters must not occur after BUG-208 fix"
    );
}
