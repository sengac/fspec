//! COV-057 Esc-interrupt fixtures: a provider that never finishes and a
//! tool that takes 5 seconds.

use codelet_providers::stub_model::{StubCompletion, StubModel};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse};
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;

type Step = Result<RawStreamingChoice<StubCompletion>, CompletionError>;

/// Provider whose response never finishes. With `partial`, that text streams
/// first and then the stream hangs; without it, the request itself never
/// returns (the agent is waiting for the API response). `reached` fires once
/// the driver is blocked on the provider.
#[derive(Clone, Default)]
pub struct HangingModel {
    pub partial: Option<&'static str>,
    pub reached: Arc<Notify>,
}

impl CompletionModel for HangingModel {
    type Response = StubCompletion;
    type StreamingResponse = StubCompletion;
    type Client = ();

    fn make(_: &Self::Client, _: impl Into<String>) -> Self {
        Self::default()
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        StubModel::new().prepare_request(request)
    }

    async fn completion(
        &self,
        _: CompletionRequest,
    ) -> Result<CompletionResponse<StubCompletion>, CompletionError> {
        Err(CompletionError::ProviderError("streaming only".into()))
    }

    async fn stream(
        &self,
        _: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<StubCompletion>, CompletionError> {
        let Some(text) = self.partial else {
            self.reached.notify_one();
            return std::future::pending().await;
        };
        let reached = Arc::clone(&self.reached);
        let first: Step = Ok(RawStreamingChoice::Message(text.to_owned()));
        let hang = futures::stream::once(async move {
            // Polled only after the driver consumed `text` and asked for more.
            reached.notify_one();
            std::future::pending::<Step>().await
        });
        Ok(StreamingCompletionResponse::stream(Box::pin(
            futures::stream::iter([first]).chain(hang),
        )))
    }
}

/// A tool that takes 5 seconds; the Notify fires once it starts running.
pub struct SlowTool(pub Arc<Notify>);

impl rig::tool::Tool for SlowTool {
    const NAME: &'static str = "SlowCheck";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;
    async fn definition(&self, _: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.into(),
            description: "Takes 5 seconds".into(),
            parameters: serde_json::json!({"type":"object","properties":{}}),
        }
    }
    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        self.0.notify_one();
        // TEST-042: the check never finishes on its own; only Esc ends it.
        std::future::pending::<()>().await;
        Ok("slow check finished".into())
    }
}
