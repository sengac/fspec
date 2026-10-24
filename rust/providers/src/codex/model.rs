use super::memory_policy;
use super::refreshing_client::RefreshingCodexClient;
use super::request_options::{CodexCapabilities, RequestOptionsError};
use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse};
use rig::http_client::HttpClientExt;
use rig::providers::openai::{responses_api::ResponsesCompletionModel, Client};
use rig::streaming::StreamingCompletionResponse;
use serde_json::Value;
use std::sync::{Arc, RwLock};

#[cfg(test)]
#[path = "manager_admission_tests.rs"]
mod manager_admission_tests;

#[path = "answers.rs"]
mod answers;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "native.rs"]
mod native;
#[path = "physical_admission.rs"]
mod physical_admission;
#[path = "reasoning.rs"]
mod reasoning;
#[path = "sections.rs"]
mod sections;
#[path = "settlement.rs"]
mod settlement;
#[path = "stream_events.rs"]
mod stream_events;
#[path = "streaming.rs"]
mod streaming;
#[path = "wire_request.rs"]
mod wire_request;

pub use native::NativeCapabilities;

type ResponsesModel = ResponsesCompletionModel<RefreshingCodexClient>;

#[derive(Clone)]
pub struct CodexResponsesModel {
    client: Client<RefreshingCodexClient>,
    model: String,
    history: super::history::CodexHistory,
    routing: Option<super::routing::CodexRoutingHandle>,
    capabilities: Arc<RwLock<Option<CodexCapabilities>>>,
    completed_reasoning: bool,
    streaming_completions: bool,
    metadata: super::metadata::CodexMetadataHandle,
    custom_tools: super::custom_tools::CustomToolRegistry,
    native_capabilities: NativeCapabilities,
    strict_attribution: bool,
    request_admission: Option<crate::request_admission::AdmissionLimits>,
    recovery: super::reconnect::CodexRecoveryHandle,
}

impl CodexResponsesModel {
    pub fn with_strict_attribution(mut self) -> Self {
        self.strict_attribution = true;
        self.custom_tools = self.custom_tools.attributed();
        self
    }

    pub fn with_recovery_handle(mut self, recovery: super::reconnect::CodexRecoveryHandle) -> Self {
        self.recovery = recovery;
        self
    }

    pub fn with_request_admission(
        mut self,
        limits: crate::request_admission::AdmissionLimits,
    ) -> Self {
        self.request_admission = Some(limits);
        self
    }

    pub fn with_streaming_completions(mut self) -> Self {
        self.streaming_completions = true;
        self
    }

    pub fn with_native_capabilities(mut self, capabilities: NativeCapabilities) -> Self {
        self.native_capabilities = capabilities;
        self
    }
    pub fn with_custom_tools(mut self, tools: super::custom_tools::CustomToolRegistry) -> Self {
        self.custom_tools = if self.strict_attribution {
            tools.attributed()
        } else {
            tools
        };
        self
    }

    pub fn with_metadata_handle(mut self, metadata: super::metadata::CodexMetadataHandle) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_routing_handle(mut self, routing: super::routing::CodexRoutingHandle) -> Self {
        self.routing = Some(routing);
        self
    }

    pub fn with_completed_reasoning(mut self, enabled: bool) -> Self {
        self.completed_reasoning = enabled;
        self
    }

    pub fn new(client: Client<RefreshingCodexClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            history: super::history::CodexHistory::default(),
            routing: None,
            capabilities: Arc::new(RwLock::new(None)),
            completed_reasoning: true,
            streaming_completions: false,
            metadata: super::metadata::CodexMetadataHandle::default(),
            custom_tools: super::custom_tools::CustomToolRegistry::default(),
            native_capabilities: NativeCapabilities::default(),
            strict_attribution: false,
            request_admission: None,
            recovery: super::reconnect::CodexRecoveryHandle::default(),
        }
    }

    pub fn with_history(mut self, history: super::history::CodexHistory) -> Self {
        self.history = history;
        self
    }

    pub fn set_capabilities(
        &self,
        capabilities: Option<CodexCapabilities>,
    ) -> Result<(), RequestOptionsError> {
        *self
            .capabilities
            .write()
            .map_err(|_| capability_lock_error())? = capabilities;
        Ok(())
    }
}

fn capability_lock_error() -> RequestOptionsError {
    RequestOptionsError::Invalid {
        option: "capabilities".into(),
        reason: "capability state unavailable; recreate the provider".into(),
    }
}

impl CompletionModel for CodexResponsesModel {
    type Response = <ResponsesModel as CompletionModel>::Response;
    type StreamingResponse = <ResponsesModel as CompletionModel>::StreamingResponse;
    type Client = Client<RefreshingCodexClient>;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model)
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        Ok(serde_json::to_vec(&self.wire_request(request, true)?)?)
    }

    fn settle_tool_results(
        &self,
        history: &[rig::message::Message],
    ) -> Result<Vec<rig::message::Message>, CompletionError> {
        settlement::settle(self, history)
    }

    fn prepare_request_for_generation(
        &self,
        request: CompletionRequest,
        generation: u64,
    ) -> Result<Vec<u8>, CompletionError> {
        let private = super::history::CodexHistory::default();
        private.restore(self.history.snapshot()?)?;
        private.transition_generation(generation)?;
        self.clone().with_history(private).prepare_request(request)
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        if self.streaming_completions {
            return streaming::stream(
                self.client.clone(),
                self.wire_request(request, true)?,
                self.history.clone(),
                self.routing.clone(),
                streaming::StreamPolicy {
                    completed_reasoning: self.completed_reasoning,
                    admission: self.request_admission,
                    recovery: self.recovery.clone(),
                },
                self.metadata.clone(),
                self.custom_tools.clone(),
            )
            .completion()
            .await;
        }
        let generation = self.history.generation()?;
        let body = self.wire_request(request, false)?;
        let bytes = serde_json::to_vec(&body)?;
        physical_admission::admit(&bytes, self.request_admission)?;
        let mut request = self
            .client
            .post("/responses")?
            .body(bytes)
            .map_err(|error| CompletionError::HttpError(error.into()))?;
        if let Some(routing) = &self.routing {
            request.extensions_mut().insert(routing.clone());
        }
        self.history.ensure_generation(generation)?;
        let response = self.client.send(request).await?;
        let success = response.status().is_success();
        let text = rig::http_client::text(response).await?;
        self.history.ensure_generation(generation)?;
        if !success {
            return Err(CompletionError::ProviderError(text));
        }
        let raw: Value = serde_json::from_str(&text)?;
        memory_policy::validate_response(&raw)?;
        let completed = raw["status"] == "completed";
        let output = raw["output"].as_array().cloned();
        let response = serde_json::from_value::<Self::Response>(raw)?.try_into()?;
        if completed {
            if let Some(output) = output {
                self.history
                    .commit_for_generation(&body, &output, generation)?;
            }
        }
        Ok(response)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        Ok(streaming::stream(
            self.client.clone(),
            self.wire_request(request, true)?,
            self.history.clone(),
            self.routing.clone(),
            streaming::StreamPolicy {
                completed_reasoning: self.completed_reasoning,
                admission: self.request_admission,
                recovery: self.recovery.clone(),
            },
            self.metadata.clone(),
            self.custom_tools.clone(),
        )
        .stream)
    }
}
