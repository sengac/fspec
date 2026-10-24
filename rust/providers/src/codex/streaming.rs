use super::lifecycle::{backend_error, invalid, Terminal};
use super::stream_events::{Choice, OutputState};
use crate::codex::refreshing_client::RefreshingCodexClient;
use futures::StreamExt;
use rig::completion::{CompletionError, GetTokenUsage};
use rig::http_client::sse::{Event, GenericEventSource};
use rig::providers::openai::{
    responses_api::streaming::StreamingCompletionResponse as Final, Client,
};
use rig::streaming::StreamingCompletionResponse;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub(super) struct CodexStream {
    pub stream: StreamingCompletionResponse<Final>,
    terminal: Arc<Mutex<Option<Value>>>,
    validate_completion: Arc<AtomicBool>,
}

pub(super) struct StreamPolicy {
    pub completed_reasoning: bool,
    pub admission: Option<crate::request_admission::AdmissionLimits>,
    pub recovery: crate::codex::reconnect::CodexRecoveryHandle,
}

impl CodexStream {
    pub async fn completion(
        mut self,
    ) -> Result<
        rig::completion::CompletionResponse<
            rig::providers::openai::responses_api::CompletionResponse,
        >,
        CompletionError,
    > {
        self.validate_completion.store(true, Ordering::Relaxed);
        while let Some(event) = self.stream.next().await {
            event?;
        }
        let terminal = self
            .terminal
            .lock()
            .map_err(|_| invalid("terminal state unavailable"))?
            .take()
            .ok_or_else(|| invalid("missing terminal response"))?;
        let mut raw: rig::providers::openai::responses_api::CompletionResponse =
            serde_json::from_value(terminal)?;
        raw.usage = self.stream.response.map(|response| response.usage);
        let mut response: rig::completion::CompletionResponse<_> = raw.try_into()?;
        response.choice = self.stream.choice;
        Ok(response)
    }
}

const MAX_CONTINUATIONS: usize = 3;

fn capture_terminal(
    captured: &Mutex<Option<Value>>,
    response: Value,
) -> Result<(), CompletionError> {
    *captured
        .lock()
        .map_err(|_| invalid("terminal state unavailable"))? = Some(response);
    Ok(())
}

fn validate_terminal(captured: &Mutex<Option<Value>>) -> Result<(), CompletionError> {
    let raw = captured
        .lock()
        .map_err(|_| invalid("terminal state unavailable"))?
        .clone()
        .ok_or_else(|| invalid("missing terminal response"))?;
    let _: rig::completion::CompletionResponse<
        rig::providers::openai::responses_api::CompletionResponse,
    > = serde_json::from_value::<rig::providers::openai::responses_api::CompletionResponse>(raw)?
        .try_into()?;
    Ok(())
}

pub(super) fn stream(
    client: Client<RefreshingCodexClient>,
    mut request: Value,
    history: crate::codex::history::CodexHistory,
    routing: Option<crate::codex::routing::CodexRoutingHandle>,
    policy: StreamPolicy,
    metadata: crate::codex::metadata::CodexMetadataHandle,
    custom_tools: crate::codex::custom_tools::CustomToolRegistry,
) -> CodexStream {
    let metadata = metadata.scoped_stream();
    let buffering = metadata.guard();
    let generation = history.generation();
    let captured = Arc::new(Mutex::new(None));
    let terminal_response = captured.clone();
    let validate_completion = Arc::new(AtomicBool::new(false));
    let completion_mode = validate_completion.clone();
    let recovery = policy.recovery.clone();
    let stream = async_stream::try_stream! {
        let _buffering = buffering;
        let mut recovery_policy = crate::codex::reconnect::RecoveryPolicy::new(policy.recovery);
        let mut total_usage: Option<rig::providers::openai::responses_api::ResponsesUsage> = None;
        let mut request_usage = Vec::new();
        let generation = generation?;
        for continuation in 0..=MAX_CONTINUATIONS {
            let (mut terminal, mut state, mut native, request_id) = loop {
            metadata.begin_response();
            let request_id = uuid::Uuid::new_v4().to_string();
            history.ensure_generation(generation)?;
            let bytes = serde_json::to_vec(&request)?;
            super::physical_admission::admit(&bytes, policy.admission)?;
            let mut http_request = client.post("/responses")?.body(bytes)
                .map_err(|error| CompletionError::HttpError(error.into()))?;
            if let Some(routing) = &routing {
                http_request.extensions_mut().insert(routing.clone());
            }
            http_request.extensions_mut().insert(metadata.clone());
            http_request.extensions_mut().insert(crate::codex::recovery::AttemptAuthority {
                history: history.clone(), generation,
            });
            yield Choice::ResponseAttemptStarted;
            let mut source = GenericEventSource::new(client.clone(), http_request);
            let mut native = super::native::NativeOutput::from_request(&request);
            let mut state = OutputState::new(policy.completed_reasoning, custom_tools.clone());
            let terminal = loop {
                match source.next().await {
                    Some(Ok(Event::Open)) => recovery_policy.connected(),
                    Some(Ok(Event::Message(event))) if event.data.trim().is_empty() => {},
                    Some(Ok(Event::Message(event))) => {
                        let data: Value = serde_json::from_str(&event.data).map_err(invalid)?;
                        super::memory_policy::validate_response(&data["response"])?;
                        metadata.apply_event(&data);
                        if let Some(routing) = &routing {
                            if let Some(value) = metadata.turn_state() {
                                routing.observe_event_turn_state(&value);
                            }
                        }
                        let kind = data["type"].as_str().ok_or_else(|| invalid("missing event type"))?;
                        if let Some(notice) = native.event(kind, &data)? {
                            yield Choice::Reasoning { id: None, reasoning: notice, signature: None };
                        }
                        match kind {
                            "error" => {
                                source.close();
                                let redactor = client.http_client().redact_diagnostics(client.headers()).await;
                                Err(backend_error(data.get("error").unwrap_or(&data).clone(), &redactor))?;
                            }
                            "response.completed" | "response.failed" | "response.incomplete" => {
                                source.close();
                                let mut response = data["response"].clone();
                                let mut terminal = Terminal::parse(kind, &mut response)?;
                                state.merge_terminal(&mut terminal.output);
                                response["output"] = Value::Array(terminal.output.clone());
                                capture_terminal(&terminal_response, response)?;
                                break Ok(terminal);
                            }
                            _ => for choice in state.event(kind, &data)? { yield choice; },
                        }
                    }
                    Some(Err(error)) => {
                        source.close();
                        break Err(CompletionError::HttpError(error));
                    }
                    None => {
                        source.close();
                        break Err(CompletionError::HttpError(crate::codex::reconnect::NetworkFailure::Interrupted.into_http_error()));
                    }
                }
            };
            drop(source);
            match terminal {
                Ok(terminal) => break (terminal, state, native, request_id),
                Err(error) => {
                    yield Choice::ResponseAttemptAborted;
                    recovery_policy.retry(error).await?;
                },
            }
            };
            history.ensure_generation(generation)?;
            if terminal.status == "failed" && terminal.usage.is_none() {
                let redactor = client.http_client().redact_diagnostics(client.headers()).await;
                if let Some(error) = terminal.error(&redactor) { Err(error)?; }
            }
            for notice in native.terminal(&terminal.output)? {
                yield Choice::Reasoning { id: None, reasoning: notice, signature: None };
            }
            for (index, item) in terminal.output.iter().enumerate() {
                for choice in state.completed_item(item, Some(index as u64))? { yield choice; }
            }
            let usage = terminal.usage.take().ok_or_else(|| invalid("missing physical usage"))?.into_usage();
            let tokens = usage.token_usage().ok_or_else(|| invalid("invalid physical usage"))?;
            let aggregate_usage = match total_usage.take() {
                Some(total) => total.checked_add(usage)?,
                None => usage,
            };
            rig::completion::report_request_usage(&request_id, &tokens)?;
            request_usage.push((request_id.clone(), tokens));
            yield Choice::Usage(tokens);
            state.saw_tool |= terminal.output.iter().any(|item| matches!(item["type"].as_str(), Some("function_call" | "custom_tool_call")));
            if terminal.should_continue(state.saw_tool) {
                if continuation == MAX_CONTINUATIONS {
                    Err(CompletionError::ProviderError("Codex continuation limit reached".to_string()))?;
                }
                terminal.replay(&mut request)?;
                total_usage = Some(aggregate_usage);
                continue;
            }
            if terminal.status == "completed" {
                for item in &terminal.output {
                    if matches!(item["type"].as_str(), Some("function_call" | "custom_tool_call")) { state.tool(item)?; }
                }
            }
            let output = terminal.output.clone();
            let redactor = client.http_client().redact_diagnostics(client.headers()).await;
            if let Some(error) = terminal.error(&redactor) { Err(error)?; }
            if completion_mode.load(Ordering::Relaxed) {
                validate_terminal(&terminal_response)?;
            }
            history.commit_for_generation(&request, &output, generation)?;
            for tool in state.tools { yield Choice::ToolCall(tool); }
            yield Choice::FinalResponse(Final {
                usage: aggregate_usage,
                request_context: Some(rig::completion::RequestContext {
                    model: request["model"].as_str().ok_or_else(|| invalid("missing model"))?.to_owned(),
                    generation,
                    request: request_id,
                    usage: Some(tokens),
                }),
                request_usage,
            });
            break;
        }
    };
    CodexStream {
        stream: StreamingCompletionResponse::stream(Box::pin(recovery.track(stream))),
        terminal: captured,
        validate_completion,
    }
}
