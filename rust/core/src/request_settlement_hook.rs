use crate::persistence::request_settlement::{
    replace_request_context, settle_request, SettlementError,
};
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::Message;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestSettlementHook<Hook> {
    inner: Hook,
    session_id: Option<Uuid>,
    request_id: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
    reported: Arc<AtomicBool>,
}

impl<Hook> RequestSettlementHook<Hook> {
    pub fn new(inner: Hook, session_id: Uuid) -> Self {
        Self::optional(inner, Some(session_id))
    }

    pub fn optional(inner: Hook, session_id: Option<Uuid>) -> Self {
        Self {
            inner,
            session_id,
            request_id: Arc::new(Mutex::new(None)),
            error: Arc::new(Mutex::new(None)),
            reported: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn check(&self) -> Result<(), SettlementError> {
        let error = self
            .error
            .lock()
            .map_err(|error| SettlementError::Failed(error.to_string()))?;
        match error.as_ref() {
            Some(error) => Err(SettlementError::Failed(error.clone())),
            None => Ok(()),
        }
    }

    fn settle(&self, usage: &rig::completion::Usage) -> Result<(), SettlementError> {
        let request_id = self
            .request_id
            .lock()
            .map_err(|error| SettlementError::Failed(error.to_string()))?;
        let request_id = request_id
            .as_ref()
            .ok_or_else(|| SettlementError::Failed("missing physical request start".into()))?;
        if let Some(session_id) = self.session_id {
            settle_request(session_id, request_id, usage)?;
        }
        Ok(())
    }
}

impl<Model, Hook> StreamingPromptHook<Model> for RequestSettlementHook<Hook>
where
    Model: CompletionModel,
    Model::StreamingResponse: GetTokenUsage,
    Hook: StreamingPromptHook<Model>,
{
    fn request_usage_observer(&self) -> Option<rig::completion::RequestUsageObserver> {
        let Some(session_id) = self.session_id else {
            return self.inner.request_usage_observer();
        };
        let reported = self.reported.clone();
        let error = self.error.clone();
        Some(Arc::new(move |identity, usage| {
            reported.store(true, Ordering::Release);
            settle_request(session_id, identity, usage)
                .map(|_| ())
                .map_err(|failure| {
                    let message = failure.to_string();
                    if let Ok(mut stored) = error.lock() {
                        *stored = Some(message.clone());
                    }
                    message
                })
        }))
    }

    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        signal: CancelSignal,
    ) {
        if self.check().is_err() {
            signal.cancel();
            return;
        }
        match self.request_id.lock() {
            Ok(mut request_id) => *request_id = Some(Uuid::new_v4().to_string()),
            Err(_) => {
                signal.cancel();
                return;
            }
        }
        self.reported.store(false, Ordering::Release);
        self.inner.on_completion_call(prompt, history, signal).await;
    }

    async fn on_stream_completion_response_finish(
        &self,
        prompt: &Message,
        response: &Model::StreamingResponse,
        signal: CancelSignal,
    ) {
        if let Some(usage) = response
            .token_usage()
            .filter(|_| !self.reported.load(Ordering::Acquire))
        {
            if let Err(error) = self.settle(&usage) {
                tracing::error!(%error, "physical request settlement failed");
                if let Ok(mut stored) = self.error.lock() {
                    *stored = Some(error.to_string());
                }
                signal.cancel();
                return;
            }
        }
        if let Some(session_id) = self.session_id {
            if let Err(error) = replace_request_context(session_id, response.context_usage()) {
                if let Ok(mut stored) = self.error.lock() {
                    *stored = Some(error.to_string());
                }
                signal.cancel();
                return;
            }
        }
        self.inner
            .on_stream_completion_response_finish(prompt, response, signal)
            .await;
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
        self.inner.on_tool_call_delta(id, name, delta, signal).await;
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
