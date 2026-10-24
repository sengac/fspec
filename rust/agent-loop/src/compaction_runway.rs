use codelet_providers::request_admission::{
    admit_complete_request, AdmissionLimits, RequestAdmissionError,
};
use rig::agent::{Agent, CancelSignal, PromptHook, StreamingPromptHook};
use rig::completion::{Completion, CompletionError, CompletionModel, CompletionRequest};
use rig::message::{AssistantContent, Message, ToolCall, ToolResult, UserContent};
use rig::OneOrMany;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    history: Vec<Message>,
    partial: Option<(Message, Vec<ToolResult>)>,
    synthesis: Option<CompletionRequest>,
    previous_input: Option<u64>,
    stopped: bool,
    failure: Option<String>,
}

#[derive(Clone)]
pub(super) struct Runway<M: CompletionModel> {
    agent: Agent<M>,
    provider: String,
    limits: AdmissionLimits,
    state: Arc<Mutex<State>>,
}

impl<M: CompletionModel> Runway<M> {
    pub(super) fn new(agent: Agent<M>, provider: &str, limits: AdmissionLimits) -> Self {
        Self {
            agent,
            provider: provider.into(),
            limits,
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    fn pending_assistant(&self, assistant: &Message) {
        if self.provider != "codex" {
            return;
        }
        match self.state.lock() {
            Ok(mut state) => {
                let (scaffold, _) = state
                    .partial
                    .get_or_insert_with(|| (assistant.clone(), Vec::new()));
                *scaffold = assistant.clone();
            }
            Err(error) => tracing::error!(%error, "Compactor assistant capture unavailable"),
        }
    }

    pub(super) fn stopped(&self) -> Result<bool, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        match &state.failure {
            Some(error) => Err(error.clone()),
            None => Ok(state.stopped),
        }
    }

    pub(super) fn history(&self) -> Result<Vec<Message>, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        if state.history.is_empty() {
            return Err("Compactor capacity recovery has no captured request evidence".into());
        }
        let mut history = state.history.clone();
        if let Some((Message::Assistant { id, content }, results)) = &state.partial {
            let completed = content
                .iter()
                .filter(|content| match content {
                    AssistantContent::ToolCall(call) => {
                        self.provider == "codex"
                            || results.iter().any(|result| {
                                result.id == call.id && result.call_id == call.call_id
                            })
                    }
                    _ => true,
                })
                .cloned();
            history.push(Message::Assistant {
                id: id.clone(),
                content: OneOrMany::many(completed).map_err(|error| error.to_string())?,
            });
            if !results.is_empty() {
                history.push(Message::User {
                    content: OneOrMany::many(results.iter().cloned().map(UserContent::ToolResult))
                        .map_err(|error| error.to_string())?,
                });
            }
        }
        Ok(history)
    }

    pub(super) fn synthesis_request(
        &self,
        mut history: Vec<Message>,
    ) -> Result<CompletionRequest, String> {
        let mut request = self
            .state
            .lock()
            .map_err(|error| error.to_string())?
            .synthesis
            .clone()
            .ok_or("Compactor has no captured synthesis request")?;
        history.push(request.chat_history.last().clone());
        request.chat_history = rig::OneOrMany::many(history).map_err(|error| error.to_string())?;
        Ok(request)
    }

    fn completed_tool(&self, assistant: &Message, result: &ToolResult) {
        match self.state.lock() {
            Ok(mut state) => {
                let (scaffold, results) = state
                    .partial
                    .get_or_insert_with(|| (assistant.clone(), Vec::new()));
                *scaffold = assistant.clone();
                results.push(result.clone());
            }
            Err(error) => tracing::error!(%error, "Compactor completed-tool capture unavailable"),
        }
    }

    async fn assess(&self, prompt: &Message, history: &[Message]) -> Result<bool, String> {
        self.limits.validate().map_err(|error| error.to_string())?;
        let mut complete_history = history.to_vec();
        complete_history.push(prompt.clone());
        {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state.history = complete_history.clone();
            state.partial = None;
        }
        let exploration = self
            .agent
            .completion(prompt.clone(), history.to_vec())
            .await
            .map_err(|error| error.to_string())?
            .build();
        let bytes = self
            .agent
            .model
            .prepare_request(exploration)
            .map_err(|error| error.to_string())?;
        let exploration = admit_complete_request(&bytes, self.limits);
        let synthesis =
            super::synthesis_request(&self.agent, complete_history, &self.provider).await?;
        self.state
            .lock()
            .map_err(|error| error.to_string())?
            .synthesis = Some(synthesis.clone());
        let synthesis_bytes = self
            .agent
            .model
            .prepare_request(synthesis)
            .map_err(|error| error.to_string())?;
        let synthesis = admit_complete_request(&synthesis_bytes, self.limits).map_err(|error| {
            format!("Compactor synthesis cannot fit complete gathered evidence without truncation: {error}")
        })?;
        let input = match exploration {
            Ok(admitted) => admitted.input_tokens(),
            Err(RequestAdmissionError::BudgetExceeded { .. }) if !history.is_empty() => {
                return Ok(true)
            }
            Err(error) => return Err(error.to_string()),
        };
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        let growth = state
            .previous_input
            .map_or(0, |previous| input.saturating_sub(previous));
        state.previous_input = Some(input);
        let runway =
            (codelet_tools::session_search::originals::MAX_ORIGINAL_BYTES as u64).max(growth);
        let budget = self
            .limits
            .context_capacity
            .unwrap_or(0)
            .saturating_sub(self.limits.output_reservation.unwrap_or(0));
        Ok(!history.is_empty()
            && input.max(synthesis.input_tokens()).saturating_add(runway) > budget)
    }

    async fn before_request(&self, prompt: &Message, history: &[Message], cancel: CancelSignal) {
        let result = self.assess(prompt, history).await;
        if !matches!(result, Ok(false)) {
            if let Ok(mut state) = self.state.lock() {
                state.stopped = true;
                state.failure = result.err();
            }
            cancel.cancel();
        }
    }
}

impl<M: CompletionModel> PromptHook<M> for Runway<M> {
    async fn on_tool_result_message(
        &self,
        assistant: &Message,
        _call: &ToolCall,
        result: &ToolResult,
    ) {
        self.completed_tool(assistant, result);
    }

    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        cancel: CancelSignal,
    ) {
        self.before_request(prompt, history, cancel).await;
    }
}

impl<M: CompletionModel> StreamingPromptHook<M> for Runway<M> {
    async fn on_tool_call_message(&self, assistant: &Message) {
        self.pending_assistant(assistant);
    }

    async fn on_tool_result_message(
        &self,
        assistant: &Message,
        _call: &ToolCall,
        result: &ToolResult,
    ) {
        self.completed_tool(assistant, result);
    }

    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        cancel: CancelSignal,
    ) {
        self.before_request(prompt, history, cancel).await;
    }
}

pub(super) fn capacity_exhausted(error: &CompletionError) -> bool {
    matches!(error, CompletionError::RequestError(source)
        if matches!(source.downcast_ref::<RequestAdmissionError>(),
            Some(RequestAdmissionError::BudgetExceeded { .. })))
}
