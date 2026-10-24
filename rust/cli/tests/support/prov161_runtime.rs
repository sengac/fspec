use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::{ProviderManager, ProviderType};
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::wasm_compat::WasmCompatSend;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub const PROMPT: &str = "Describe the selected runtime settings.";
pub const BUDGET: u64 = 12_288;

pub struct RuntimeFixture {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
}

impl RuntimeFixture {
    pub fn new() -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        Ok(Self {
            directory,
            environment: Vec::new(),
        })
    }

    pub fn environment(&mut self, name: &'static str, value: Option<&str>) {
        self.environment.push((name, std::env::var_os(name)));
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    pub fn manager(&self, provider: ProviderType, settings: Value) -> TestResult<ProviderManager> {
        std::fs::write(
            self.directory.path().join("fspec-config.json"),
            serde_json::to_vec(&json!({"providers": {
                provider.as_str(): {"settings": settings}
            }}))?,
        )?;
        let mut manager = ProviderManager::for_testing(provider.clone(), None, None);
        manager.set_model_direct(
            provider.as_str(),
            "runtime-test-model",
            Some(200_000),
            None,
            None,
        )?;
        Ok(manager)
    }

    pub fn session_id(&self, provider: &str) -> TestResult<uuid::Uuid> {
        Ok(codelet_core::persistence::create_session_with_provider(
            "PROV-161 runtime",
            self.directory.path(),
            provider,
        )?
        .id)
    }
}

impl Drop for RuntimeFixture {
    fn drop(&mut self) {
        codelet_core::persistence::reset_stores_for_tests();
        for (name, value) in self.environment.iter().rev() {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

pub fn limits(manager: &ProviderManager) -> TestResult<AdmissionLimits> {
    Ok(AdmissionLimits {
        context_capacity: Some(u64::try_from(manager.context_window())?),
        output_reservation: Some(u64::try_from(manager.max_output_tokens())?),
    })
}

pub async fn codex_model(
    server_url: &str,
    limits: AdmissionLimits,
) -> TestResult<codelet_providers::codex::CodexResponsesModel> {
    use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("offline-runtime-key")
        .base_url(server_url)
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(
        codelet_providers::codex::CodexResponsesModel::new(client, "runtime-test-model")
            .with_streaming_completions()
            .with_request_admission(limits),
    )
}

pub fn codex_events() -> Vec<String> {
    [
        json!({"type":"response.output_text.delta", "item_id":"answer", "output_index":0,
            "content_index":0, "delta":"Hello "}),
        json!({"type":"response.output_text.delta", "item_id":"answer", "output_index":0,
            "content_index":0, "delta":"world"}),
        json!({"type":"response.completed", "response": {
            "id":"runtime-response", "status":"completed", "end_turn":true,
            "output":[{"id":"answer", "type":"message", "role":"assistant",
                "status":"completed", "content":[{"type":"output_text", "text":"Hello world", "annotations":[]}]}],
            "usage":{"input_tokens":20,"output_tokens":2,"total_tokens":22}
        }}),
    ]
    .iter()
    .map(|event| format!("data: {event}\n\n"))
    .collect()
}

pub async fn codex_response(server: &MockServer) {
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(codex_events().concat()),
        )
        .expect(1)
        .mount(server)
        .await;
}

#[derive(Default)]
pub struct Output(pub Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

pub async fn run<M: CompletionModel + 'static>(
    agent: RigAgent<M>,
    session: &mut Session,
    session_id: uuid::Uuid,
    output: &Output,
) -> TestResult
where
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            PROMPT,
            session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            output,
            session_id,
        ),
    )
    .await??;
    Ok(())
}
