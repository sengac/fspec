use axum::{http::StatusCode, routing::post, Json, Router};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive::stream_loop::run_agent_stream;
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::Notify;
use tracing::Instrument;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Output;
impl StreamOutput for Output {
    fn emit(&self, _event: StreamEvent) {}
}

pub async fn fail_in_real_stream_loop(stream: bool) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let body = json!({"error": {"message": "quota exceeded", "code": "quota_rejected"},
        "system_instructions": "APP_SYSTEM_PRIVATE", "prompt": "APP_PROMPT_PRIVATE\u{200b}\u{e0001}\\u{200b}",
        "completion": "APP_COMPLETION_PRIVATE", "tool": {"arguments": "APP_ARGS_PRIVATE", "result": "APP_RESULT_PRIVATE"}});
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let router = Router::new().route(
        "/chat/completions",
        post(move || async move { (StatusCode::BAD_REQUEST, Json(body)) }),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    let client: rig::providers::openai::Client = rig::providers::openai::Client::builder()
        .api_key("synthetic")
        .base_url(format!("http://{address}"))
        .build()?;
    let model = client
        .completions_api()
        .completion_model("fixture")
        .with_stream(stream);
    let agent = rig::agent::AgentBuilder::new(model).build();
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    ));
    let span = tracing::info_span!(
        "provider_failure",
        session_id = "provider-fixture-239",
        request_id = "request-fixture-239"
    );
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "synthetic input",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(Notify::new()),
            &Output,
            uuid::Uuid::new_v4(),
        )
        .instrument(span),
    )
    .await;
    task.abort();
    assert!(
        outcome?.is_err(),
        "fixture must fail at the real provider boundary"
    );
    Ok(())
}
