//! Feature: spec/features/copilot-agent-loop-dispatch.feature
//!
//! COV-057: drives the REAL agent loop for a `github-copilot/gpt-4o`
//! session against a loopback Copilot chat-completions endpoint. The
//! only way the request can reach that endpoint, authenticated with the
//! on-disk Copilot token, is through the `"github-copilot"` arm of
//! `run_with_provider!` (`provider_manager.get_github_copilot()`); any
//! other arm would fall through to the custom-provider path and surface
//! an "Unsupported provider" error.

#[path = "support/bug268_environment.rs"]
mod environment;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_cli::session::Session;
use codelet_providers::copilot::{get_copilot_auth_path, write_copilot_auth, CopilotAuthJson};
use codelet_providers::ProviderManager;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use environment::{Environment, TestResult};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

const ANSWER: &str = "copilot dispatch answered";

fn copilot_reply(request: &Request) -> ResponseTemplate {
    let body = serde_json::from_slice::<Value>(&request.body).unwrap_or_default();
    let usage = json!({"prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15});
    if body["stream"] == true {
        let text = json!({"id": "cov057", "object": "chat.completion.chunk", "created": 1,
            "model": "gpt-4o", "choices": [{"index": 0,
            "delta": {"role": "assistant", "content": ANSWER}, "finish_reason": null}]});
        let stop = json!({"id": "cov057", "object": "chat.completion.chunk", "created": 1,
            "model": "gpt-4o", "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            "usage": usage});
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {text}\n\ndata: {stop}\n\ndata: [DONE]\n\n"))
    } else {
        ResponseTemplate::new(200).set_body_json(json!({"id": "cov057",
            "object": "chat.completion", "created": 1, "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": ANSWER},
            "finish_reason": "stop"}], "usage": usage}))
    }
}

/// Scenario: Agent loop dispatches github-copilot to CopilotProvider
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_agent_loop_dispatches_github_copilot_to_copilot_provider() -> TestResult {
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(copilot_reply)
        .mount(&server)
        .await;

    // @step Given a session has selected a "github-copilot/gpt-4o" model
    let (provider, model) = "github-copilot/gpt-4o"
        .split_once('/')
        .ok_or("model string must be provider/model")?;
    let manifest =
        codelet_core::persistence::create_session_with_provider("dispatch", root, provider)?;

    // @step And valid Copilot credentials exist on disk
    assert!(get_copilot_auth_path().starts_with(root));
    write_copilot_auth(&CopilotAuthJson {
        github_oauth_token: "synthetic-dispatch-account".into(),
        copilot_token: Some("synthetic-dispatch-token".into()),
        copilot_token_expires_at: Some(u64::MAX),
        endpoints_api: Some(server.uri()),
        enterprise_url: None,
    })
    .await?;

    // @step When the agent loop processes a chat message
    let mut manager = ProviderManager::with_provider(provider)?;
    manager.set_model_direct(provider, model, Some(128000), Some(4096), None)?;
    assert_eq!(manager.current_provider_name(), "github-copilot");
    let (input_tx, input_rx) = mpsc::channel(4);
    let (chunks_tx, mut chunks) = broadcast::channel(256);
    let background = Arc::new(BackgroundSession::new(
        manifest.id,
        "dispatch".into(),
        root.display().to_string(),
        None,
        None,
        Session::from_provider_manager(manager),
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        broadcast::channel(32).0,
        broadcast::channel(4).0,
    ));
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(background.id, root.to_owned())?;
    let (_injections, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(background.clone(), input_rx, injection_rx));
    background.send_input("say hello through copilot".into(), None)?;
    let events = codelet_test_helpers::deadlock_guard("the awaited block", async {
        let mut events = Vec::new();
        loop {
            let (_, chunk) = chunks.recv().await?;
            let done = matches!(chunk, StreamChunk::Done);
            events.push(chunk);
            if done {
                return Ok::<_, broadcast::error::RecvError>(events);
            }
        }
    })
    .await??;
    task.abort();

    // @step Then the run_with_provider macro matches the "github-copilot" arm
    let requests = server.received_requests().await.ok_or("missing capture")?;
    let chat = requests
        .iter()
        .find(|request| request.url.path().ends_with("/chat/completions"))
        .ok_or("the Copilot chat-completions endpoint saw no request")?;

    // @step And it constructs a CopilotProvider via provider_manager.get_github_copilot()
    assert_eq!(
        chat.headers["authorization"],
        "Bearer synthetic-dispatch-token"
    );
    assert_eq!(chat.body_json::<Value>()?["model"], "gpt-4o");

    // @step And the response stream completes without an "Unsupported provider" error
    let errors = events
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::Error { error } => Some(error.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "turn surfaced errors: {errors:?}");
    let text = events
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains(ANSWER), "{events:?}");
    Ok(())
}
