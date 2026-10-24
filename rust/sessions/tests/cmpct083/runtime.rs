use super::{fixture::TestResult, transport};
use codelet_cli::{
    interactive::{run_agent_stream, StreamEvent, StreamOutput},
    session::Session,
};
use codelet_core::RigAgent;
use codelet_providers::{
    codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel},
    copilot::CopilotHttpClient,
};
use codelet_sessions::background_session::BackgroundSession;
use parking_lot::Mutex;
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, GetTokenUsage},
};
use serde_json::Value;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use wiremock::MockServer;

#[derive(Default)]
pub struct Output {
    events: Mutex<Vec<StreamEvent>>,
    persistence: Option<codelet_agent_loop::BackgroundOutput>,
}

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().push(event.clone());
        if let Some(persistence) = &self.persistence {
            persistence.emit(event);
        }
    }
}

impl Output {
    pub fn production(background: Arc<BackgroundSession>, provider: String) -> Self {
        Self {
            events: Mutex::default(),
            persistence: Some(codelet_agent_loop::BackgroundOutput::with_provider(
                background, provider, None,
            )),
        }
    }

    pub fn assert_no_completion(&self) {
        assert!(!self
            .events
            .lock()
            .iter()
            .any(|event| matches!(event, StreamEvent::CompactionComplete(_))));
    }
}

pub async fn stream<M>(model: M, inner: &mut Session, id: uuid::Uuid, output: &Output) -> TestResult
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Send + GetTokenUsage,
{
    prepare_turn(inner, id, output)?;
    stream_parameters(model, inner, id, output, None).await
}

pub fn prepare_turn(inner: &mut Session, id: uuid::Uuid, output: &Output) -> TestResult {
    if output.persistence.is_some() && inner.ensure_compaction_inference_ready().is_ok() {
        codelet_agent_loop::persist::persist_user_message(&id, "next")?;
    }
    inner.bind_provider_persistence(id)?;
    inner.begin_provider_turn();
    Ok(())
}

pub async fn stream_parameters<M>(
    model: M,
    inner: &mut Session,
    id: uuid::Uuid,
    output: &Output,
    parameters: Option<Value>,
) -> TestResult
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Send + GetTokenUsage,
{
    let mut builder = rig::agent::AgentBuilder::new(model)
        .preamble("mandatory gate instructions")
        .max_tokens(4096)
        .tool(codelet_tools::inject_summary::InjectSummaryTool::new(id));
    if let Some(parameters) = parameters {
        builder = builder.additional_params(parameters);
    }
    let agent = RigAgent::new(builder.build(), 3);
    run_agent(agent, inner, id, output).await
}

pub async fn run_agent<M>(
    agent: RigAgent<M>,
    inner: &mut Session,
    id: uuid::Uuid,
    output: &Output,
) -> TestResult
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Send + GetTokenUsage,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        run_agent_stream(
            agent,
            "next",
            inner,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            output,
            id,
        ),
    )
    .await?
    .map_err(|error| error.into_boxed_dyn_error() as Box<dyn std::error::Error>)?;
    Ok(())
}

pub async fn send(
    row: &str,
    root: &Path,
    background: &Arc<BackgroundSession>,
) -> TestResult<Value> {
    let server = transport::server(row).await;
    let mut inner = background.inner.lock().await;
    let output = Output::production(background.clone(), inner.current_provider_name().to_owned());
    send_inner(row, root, &server, &mut inner, background.id, &output).await?;
    if row == "stub" {
        return Ok(serde_json::from_slice(
            &codelet_providers::stub_model::last_request_bytes(),
        )?);
    }
    transport::only(&server).await
}

pub async fn send_inner(
    row: &str,
    root: &Path,
    server: &MockServer,
    inner: &mut Session,
    id: uuid::Uuid,
    output: &Output,
) -> TestResult {
    match row {
        "zai" | "zai-plan" => super::zai::send(row, server, inner, id, output).await,
        "anthropic" => {
            let client: rig::providers::anthropic::Client =
                rig::providers::anthropic::Client::builder()
                    .api_key("offline-gate-key")
                    .base_url(server.uri())
                    .build()?;
            stream(
                client.completion_model("claude-sonnet-4"),
                inner,
                id,
                output,
            )
            .await
        }
        "openai" => {
            let client: rig::providers::openai::CompletionsClient =
                rig::providers::openai::CompletionsClient::builder()
                    .api_key("offline-gate-key")
                    .base_url(server.uri())
                    .build()?;
            stream(client.completion_model("o3"), inner, id, output).await
        }
        "gemini" => {
            let client: rig::providers::gemini::Client = rig::providers::gemini::Client::builder()
                .api_key("offline-gate-key")
                .base_url(server.uri())
                .build()?;
            stream(client.completion_model("gemini-2.5-pro"), inner, id, output).await
        }
        "copilot" => {
            let client = rig::providers::openai::CompletionsClient::<CopilotHttpClient>::builder()
                .api_key("offline-gate-key")
                .http_client(CopilotHttpClient::new("offline-gate-token".into()))
                .base_url(server.uri())
                .build()?;
            stream(client.completion_model("gpt-4o"), inner, id, output).await
        }
        "codex" => {
            let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
                .api_key("offline-gate-key")
                .http_client(RefreshingCodexClient::new_api_key())
                .base_url(server.uri())
                .build()?;
            let extension = inner
                .provider_session
                .extension::<codelet_providers::codex::session::CodexSession>("codex")
                .ok_or("selected Codex session extension missing")?;
            let model = CodexResponsesModel::new(client, "shared-model")
                .with_history(extension.history.clone())
                .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: Some(200_000),
                    output_reservation: Some(4096),
                });
            stream(model, inner, id, output).await
        }
        "custom" => {
            stream(
                super::custom::model(root, &server.uri())?,
                inner,
                id,
                output,
            )
            .await
        }
        "stub" => {
            stream(
                codelet_providers::stub_model::StubModel::new(),
                inner,
                id,
                output,
            )
            .await
        }
        _ => Err(format!("unknown provider gate row {row}").into()),
    }
}
