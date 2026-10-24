use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::CompletionModel;
use rig::message::Message;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexToolInputUpdate {
    pub item_id: String,
    pub name: Option<String>,
    pub arguments_delta: String,
}

pub type CodexToolInputSink = Arc<dyn Fn(CodexToolInputUpdate) + Send + Sync>;

#[derive(Clone)]
pub struct CodexPresentationHook<Hook> {
    inner: Hook,
    sink: Option<CodexToolInputSink>,
}

impl<Hook> CodexPresentationHook<Hook> {
    pub fn new(inner: Hook, sink: Option<CodexToolInputSink>) -> Self {
        Self { inner, sink }
    }
}

impl<Model, Hook> StreamingPromptHook<Model> for CodexPresentationHook<Hook>
where
    Model: CompletionModel,
    Hook: StreamingPromptHook<Model>,
{
    fn request_usage_observer(&self) -> Option<rig::completion::RequestUsageObserver> {
        self.inner.request_usage_observer()
    }

    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        signal: CancelSignal,
    ) {
        self.inner.on_completion_call(prompt, history, signal).await;
    }

    async fn on_text_delta(&self, delta: &str, text: &str, signal: CancelSignal) {
        self.inner.on_text_delta(delta, text, signal).await;
    }

    async fn on_tool_call_delta(
        &self,
        id: &str,
        name: Option<&str>,
        delta: &str,
        signal: CancelSignal,
    ) {
        self.inner
            .on_tool_call_delta(id, name, delta, signal.clone())
            .await;
        if !signal.is_cancelled() {
            if let Some(sink) = &self.sink {
                sink(CodexToolInputUpdate {
                    item_id: id.to_owned(),
                    name: name.map(str::to_owned),
                    arguments_delta: delta.to_owned(),
                });
            }
        }
    }

    async fn on_stream_completion_response_finish(
        &self,
        prompt: &Message,
        response: &Model::StreamingResponse,
        signal: CancelSignal,
    ) {
        self.inner
            .on_stream_completion_response_finish(prompt, response, signal)
            .await;
    }

    async fn on_tool_call(&self, name: &str, id: Option<String>, args: &str, signal: CancelSignal) {
        self.inner.on_tool_call(name, id, args, signal).await;
    }

    async fn on_tool_result(
        &self,
        name: &str,
        id: Option<String>,
        args: &str,
        result: &str,
        signal: CancelSignal,
    ) {
        self.inner
            .on_tool_result(name, id, args, result, signal)
            .await;
    }
}
