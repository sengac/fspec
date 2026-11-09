use codelet_providers::request_admission::{admit_complete_request, AdmissionLimits};
use futures::StreamExt;
use rig::agent::{Agent, MultiTurnStreamItem, StreamingError};
use rig::completion::{
    Completion, CompletionModel, CompletionRequest, GetTokenUsage, Prompt, PromptError,
};
use rig::message::{AssistantContent, Message};
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};

#[path = "compaction_chunking.rs"]
mod chunking;
#[path = "compaction_runway.rs"]
mod runway;
#[path = "compaction_settlement.rs"]
pub mod settlement;

enum Exploration {
    Complete(String),
    Failed(PromptError),
}

/// BUG-614 (Q7): the conversation that generated the latest candidate, so a
/// correction continues it instead of starting a new synthesis.
pub struct Conversation {
    messages: Vec<Message>,
    candidate: String,
}

type Shared<'conversation> = &'conversation std::sync::Mutex<Option<Conversation>>;

#[allow(clippy::too_many_arguments)]
pub async fn run<M>(
    agent: Agent<M>,
    session_id: uuid::Uuid,
    task: &str,
    provider: &str,
    maximum_depth: usize,
    request: codelet_cli::compaction_operation::BuildRequest,
    conversation: Shared<'_>,
    limits: AdmissionLimits,
) -> Result<String, String>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Clone + Unpin + GetTokenUsage,
{
    codelet_tools::tool_attribution::configure_agent_tools(&agent, session_id.to_string())
        .await
        .map_err(|error| format!("Compaction attribution configuration failed: {error}"))?;
    let streaming = crate::deep_search_handler::provider_uses_streaming_execution(provider);
    if let codelet_cli::compaction_operation::BuildRequest::Correction(correction) = request {
        // A correction answers inside the generating conversation: the
        // rejected candidate is the assistant turn, the exact error the next
        // user turn. Exploration is not repeated.
        let previous = conversation
            .lock()
            .map_err(|error| error.to_string())?
            .take()
            .ok_or("compactor correction has no generating conversation")?;
        let mut history = previous.messages;
        history.push(Message::assistant(previous.candidate));
        let request =
            conversation_request(&agent, Message::user(correction), history, provider).await?;
        return synthesize(
            agent,
            request,
            None,
            streaming,
            provider,
            limits,
            conversation,
        )
        .await;
    }
    let runway = runway::Runway::new(agent.clone(), provider, limits);
    let exploration = Box::pin(async {
        let error = if streaming {
            let stream = agent
                .stream_prompt(task)
                .multi_turn(maximum_depth)
                .with_hook(runway.clone())
                .await;
            futures::pin_mut!(stream);
            let mut response = None;
            let mut failure = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(MultiTurnStreamItem::FinalResponse(final_response)) => {
                        response = Some(final_response.response().to_string());
                    }
                    Err(StreamingError::Completion(error)) => {
                        failure = Some(PromptError::CompletionError(error));
                        break;
                    }
                    Err(StreamingError::Prompt(error)) => {
                        failure = Some(*error);
                        break;
                    }
                    Err(error) => return Err(format!("Compactor sub-agent failed: {error}")),
                    _ => {}
                }
            }
            match failure {
                Some(error) => error,
                None => {
                    return response
                        .map(Exploration::Complete)
                        .ok_or_else(|| "Compactor returned no final response".to_string())
                }
            }
        } else {
            match agent
                .prompt(task)
                .multi_turn(maximum_depth)
                .with_hook(runway.clone())
                .await
            {
                Ok(response) => return Ok(Exploration::Complete(response)),
                Err(error) => error,
            }
        };
        Ok(Exploration::Failed(error))
    });
    let phase = codelet_cli::compaction_timing::Phase::new("exploration");
    let result = tokio::time::timeout_at(
        codelet_cli::compaction_timing::exploration_deadline(),
        exploration,
    )
    .await;
    drop(phase);
    let error = match result {
        Ok(result) => match result? {
            // BUG-614 (Q8): the exploration's final answer is a candidate
            // chunk too; it is checked and continued like a synthesis chunk.
            Exploration::Complete(response) => {
                let mut history = runway.history()?;
                let prompt = history
                    .pop()
                    .ok_or("Compactor has no final exploration request")?;
                let request = conversation_request(&agent, prompt, history, provider).await?;
                let first = chunking::Chunk {
                    text: response,
                    stop_reason: None,
                };
                return synthesize(
                    agent,
                    request,
                    Some(first),
                    streaming,
                    provider,
                    limits,
                    conversation,
                )
                .await;
            }
            Exploration::Failed(error) => error,
        },
        Err(_) => {
            tracing::info!(
                provider,
                "Compactor elapsed exploration budget reserved synthesis runway"
            );
            return synthesize(
                agent,
                runway.synthesis_request(runway.history()?)?,
                None,
                streaming,
                provider,
                limits,
                conversation,
            )
            .await;
        }
    };
    if runway.stopped()? {
        tracing::info!(provider, "Compactor exploration reserved synthesis runway");
        return synthesize(
            agent,
            runway.synthesis_request(runway.history()?)?,
            None,
            streaming,
            provider,
            limits,
            conversation,
        )
        .await;
    }
    match error {
        PromptError::CompletionError(ref error) if runway::capacity_exhausted(error) => {
            tracing::info!(
                provider,
                "Compactor physical request exhausted capacity; synthesizing gathered evidence"
            );
            synthesize(
                agent,
                runway.synthesis_request(runway.history()?)?,
                None,
                streaming,
                provider,
                limits,
                conversation,
            )
            .await
        }
        PromptError::MaxDepthError {
            chat_history,
            prompt,
            ..
        } => {
            let mut history = *chat_history;
            if history.last() != Some(prompt.as_ref()) {
                history.push(*prompt);
            }
            synthesize(
                agent,
                runway.synthesis_request(history)?,
                None,
                streaming,
                provider,
                limits,
                conversation,
            )
            .await
        }
        error => Err(format!("Compactor sub-agent failed: {error}")),
    }
}

async fn synthesis_request<M: CompletionModel>(
    agent: &Agent<M>,
    history: Vec<Message>,
    provider: &str,
) -> Result<CompletionRequest, String> {
    conversation_request(
        agent,
        Message::user(
            "Exploration is finished. Return the complete DAG now using only the gathered evidence. \
             No tools are available. Preserve the authoritative source boundary; do not invent missing evidence.",
        ),
        history,
        provider,
    )
    .await
}

/// A tool-free request that continues `history` with `prompt`.
async fn conversation_request<M: CompletionModel>(
    agent: &Agent<M>,
    prompt: Message,
    history: Vec<Message>,
    provider: &str,
) -> Result<CompletionRequest, String> {
    let mut request = agent
        .completion(prompt, history)
        .await
        .map_err(|error| error.to_string())?
        .build();
    if provider == "claude" {
        request.tool_choice = Some(rig::message::ToolChoice::None);
    } else {
        request.tools.clear();
        request.tool_choice = None;
    }
    Ok(request)
}

async fn synthesize<M>(
    agent: Agent<M>,
    mut request: CompletionRequest,
    first: Option<chunking::Chunk>,
    streaming: bool,
    provider: &str,
    limits: AdmissionLimits,
    conversation: Shared<'_>,
) -> Result<String, String>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Clone + Unpin + GetTokenUsage,
{
    {
        let _phase = codelet_cli::compaction_timing::Phase::new("settlement");
        let mut history: Vec<_> = request.chat_history.iter().cloned().collect();
        let handoff = settlement::reconcile(agent.model.as_ref(), &history)
            .map_err(|error| error.to_string())?;
        if !handoff.is_empty() {
            let instruction = history
                .pop()
                .ok_or("Compactor has no synthesis instruction")?;
            history.extend(handoff);
            history.push(instruction);
            request.chat_history =
                rig::OneOrMany::many(history).map_err(|error| error.to_string())?;
        }
    }
    let _phase = codelet_cli::compaction_timing::Phase::new("synthesis");
    let messages: Vec<Message> = request.chat_history.iter().cloned().collect();
    let candidate = tokio::time::timeout_at(
        codelet_cli::compaction_timing::synthesis_deadline(),
        chunking::synthesize_in_chunks(request, first, |request| {
            synthesis_chunk(&agent, request, streaming, limits)
        }),
    )
    .await
    .map_err(|_| {
        tracing::warn!(operation_id = ?codelet_cli::compaction_timing::operation_id(),
            phase = "synthesis", provider, retryable = true,
            "Compaction phase deadline exceeded; the next input retries it");
        "compaction synthesis deadline exceeded; submit fresh input to retry".to_string()
    })??;
    *conversation.lock().map_err(|error| error.to_string())? = Some(Conversation {
        messages,
        candidate: candidate.clone(),
    });
    Ok(candidate)
}

async fn synthesis_chunk<M>(
    agent: &Agent<M>,
    request: CompletionRequest,
    streaming: bool,
    limits: AdmissionLimits,
) -> Result<chunking::Chunk, String>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: Clone + Unpin + GetTokenUsage,
{
    let bytes = agent
        .model
        .prepare_request(request.clone())
        .map_err(|error| error.to_string())?;
    admit_complete_request(&bytes, limits).map_err(|error| error.to_string())?;
    let unavailable_tool = || "Compactor synthesis requested an unavailable tool".to_string();
    if streaming {
        let mut stream = agent
            .model
            .stream(request)
            .await
            .map_err(|error| error.to_string())?;
        let mut text = String::new();
        let mut snapshot = 0;
        let mut stop_reason = None;
        while let Some(item) = stream.next().await {
            match item.map_err(|error| error.to_string())? {
                StreamedAssistantContent::Text(fragment) => text.push_str(&fragment.text),
                StreamedAssistantContent::ResponseAttemptStarted => snapshot = text.len(),
                StreamedAssistantContent::ResponseAttemptAborted => text.truncate(snapshot),
                StreamedAssistantContent::Final(response) => {
                    stop_reason = response.stop_reason().map(str::to_owned);
                }
                StreamedAssistantContent::ToolCall(_)
                | StreamedAssistantContent::ToolCallDelta { .. } => return Err(unavailable_tool()),
                _ => {}
            }
        }
        Ok(chunking::Chunk { text, stop_reason })
    } else {
        let response = agent
            .model
            .completion(request)
            .await
            .map_err(|error| error.to_string())?;
        let stop_reason = serde_json::to_value(&response.raw_response)
            .ok()
            .as_ref()
            .and_then(chunking::raw_stop_reason);
        let mut text = String::new();
        for content in response.choice {
            match content {
                AssistantContent::Text(fragment) => text.push_str(&fragment.text),
                AssistantContent::ToolCall(_) => return Err(unavailable_tool()),
                _ => {}
            }
        }
        Ok(chunking::Chunk { text, stop_reason })
    }
}
