//! COV-057 offline provider turn helper.
//!
//! Generalises `cmpct060/support.rs::next_anthropic_wire`: runs ONE real
//! turn of a manager-owned session through `run_agent_stream` against a
//! wiremock Anthropic endpoint that streams a text `reply`, with the user
//! prompt persisted first exactly as the agent loop's admission does. Returns
//! the JSON body of the provider request (the API-format messages the model
//! actually received).

use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::prelude::CompletionClient;
use serde_json::{json, Value};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type WireResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn anthropic_text_stream(reply: &str, input_tokens: u64, cache_read: u64) -> String {
    let start = json!({"type":"message_start","message":{
        "id":"msg_test","type":"message","role":"assistant","model":"test",
        "content":[],"usage":{"input_tokens":input_tokens,
            "cache_read_input_tokens":cache_read,"output_tokens":0}
    }});
    let block = json!({"type":"content_block_start","index":0,
        "content_block":{"type":"text","text":""}});
    let delta = json!({"type":"content_block_delta","index":0,
        "delta":{"type":"text_delta","text":reply}});
    let stop = json!({"type":"content_block_stop","index":0});
    let end = json!({"type":"message_delta",
        "delta":{"stop_reason":"end_turn","stop_sequence":null},
        "usage":{"output_tokens":7}});
    format!(
        "event: message_start\ndata: {start}\n\n\
         event: content_block_start\ndata: {block}\n\n\
         event: content_block_delta\ndata: {delta}\n\n\
         event: content_block_stop\ndata: {stop}\n\n\
         event: message_delta\ndata: {end}\n\n\
         event: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"
    )
}

/// Run one turn: persist `prompt`, stream it to a mock Anthropic endpoint
/// answering `reply` (reporting `input_tokens` of context), and return the
/// captured request body.
pub async fn turn(
    manager: &SessionManager,
    session_id: &SessionId,
    prompt: &str,
    reply: &str,
    input_tokens: u64,
) -> WireResult<Value> {
    cached_turn(manager, session_id, prompt, reply, input_tokens, 0).await
}

/// [`turn`] whose response also reports `cache_read` cache-read input tokens.
pub async fn cached_turn(
    manager: &SessionManager,
    session_id: &SessionId,
    prompt: &str,
    reply: &str,
    input_tokens: u64,
    cache_read: u64,
) -> WireResult<Value> {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            anthropic_text_stream(reply, input_tokens, cache_read),
            "text/event-stream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let client: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("test")
        .base_url(server.uri())
        .build()?;
    let background = manager.get_session(&session_id.value)?;
    codelet_agent_loop::persist::persist_user_message(&background.id, prompt)?;
    let mut inner = background.inner.lock().await;
    let output = codelet_agent_loop::BackgroundOutput::with_provider(
        background.clone(),
        inner.current_provider_name().to_owned(),
        None,
    );
    let agent = codelet_core::RigAgent::new(
        rig::agent::AgentBuilder::new(client.completion_model("claude-sonnet-4"))
            .max_tokens(100)
            .build(),
        1,
    );
    inner.bind_provider_persistence(background.id)?;
    inner.begin_provider_turn();
    codelet_test_helpers::deadlock_guard(
        "codelet_cli::interactive::run_agent_stream( agent, prompt...",
        codelet_cli::interactive::run_agent_stream(
            agent,
            prompt,
            &mut inner,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(tokio::sync::Notify::new()),
            &output,
            background.id,
        ),
    )
    .await??;
    drop(inner);
    let requests = server
        .received_requests()
        .await
        .ok_or("capture unavailable")?;
    Ok(requests.first().ok_or("no provider request")?.body_json()?)
}
