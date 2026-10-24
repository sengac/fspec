use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use futures::StreamExt;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionRequestBuilder,
};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use serde_json::Value;
use wiremock::MockServer;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct PersistedHistory {
    pub history: CodexHistory,
    directory: tempfile::TempDir,
}

impl PersistedHistory {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let path = directory.path().join("checkpoint.json");
        let history = CodexHistory::default();
        history.set_writer(std::sync::Arc::new(move |checkpoint| {
            let bytes = serde_json::to_vec(&checkpoint).map_err(|error| error.to_string())?;
            std::fs::write(&path, bytes).map_err(|error| error.to_string())
        }))?;
        Ok(Self { history, directory })
    }

    pub fn bytes(&self) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.directory.path().join("checkpoint.json"))
    }

    pub fn transition(&self) -> Result<(), codelet_providers::session::ExtensionError> {
        let mut provider = codelet_providers::session::ProviderSession::default();
        provider.register(
            "codex",
            std::sync::Arc::new(codelet_providers::codex::session::CodexSession {
                history: self.history.clone(),
                ..Default::default()
            }),
        );
        provider.reset()
    }
}

pub fn model(
    server: &MockServer,
    history: CodexHistory,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

pub fn request(model: &CodexResponsesModel) -> CompletionRequest {
    CompletionRequestBuilder::new(model.clone(), "Hello").build()
}

pub async fn consume(
    model: &CodexResponsesModel,
    streaming: bool,
) -> Result<CompletionError, Box<dyn std::error::Error>> {
    consume_request(model, streaming, request(model)).await
}

pub async fn consume_request(
    model: &CodexResponsesModel,
    streaming: bool,
    request: CompletionRequest,
) -> Result<CompletionError, Box<dyn std::error::Error>> {
    if !streaming {
        return model
            .completion(request)
            .await
            .err()
            .ok_or_else(|| "native memory unexpectedly completed".into());
    }
    let mut stream = match model.stream(request).await {
        Ok(stream) => stream,
        Err(error) => return Ok(error),
    };
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => return Ok(error),
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_)) => {
                return Err("native memory completed or dispatched a tool".into());
            }
            _ => {}
        }
    }
    Err("native memory did not return an error".into())
}

pub fn unsupported(error: &CompletionError) {
    assert!(
        matches!(error, CompletionError::RequestError(source)
        if source.downcast_ref::<codelet_providers::codex::memory_policy::NativeMemoryUnsupported>().is_some()),
        "{error}"
    );
    assert!(
        error.to_string().contains("native memory unsupported"),
        "{error}"
    );
    assert!(!format!("{error:?}").contains("private-native"));
}

pub fn terminal(output: Vec<Value>) -> Value {
    let mut raw = super::baseline::response();
    raw["output"] = output.into();
    raw["end_turn"] = true.into();
    raw
}
