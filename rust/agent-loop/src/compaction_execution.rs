use codelet_providers::request_admission::{admit_complete_request, AdmissionLimits};
use futures::StreamExt;
use rig::agent::{Agent, MultiTurnStreamItem, StreamingError};
use rig::completion::{
    Completion, CompletionModel, CompletionRequest, GetTokenUsage, Prompt, PromptError,
};
use rig::message::{AssistantContent, Message};
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};

#[path = "compaction_runway.rs"]
mod runway;
#[path = "compaction_settlement.rs"]
pub mod settlement;

enum Exploration {
    Complete(String),
    Failed(PromptError),
}

pub async fn run<M>(
    agent: Agent<M>,
    session_id: uuid::Uuid,
    task: &str,
    provider: &str,
    maximum_depth: usize,
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
            Exploration::Complete(response) => return Ok(response),
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
                streaming,
                provider,
                limits,
            )
            .await;
        }
    };
    if runway.stopped()? {
        tracing::info!(provider, "Compactor exploration reserved synthesis runway");
        return synthesize(
            agent,
            runway.synthesis_request(runway.history()?)?,
            streaming,
            provider,
            limits,
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
                streaming,
                provider,
                limits,
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
                streaming,
                provider,
                limits,
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
    let mut request = agent
        .completion(
            "Exploration is finished. Return the complete DAG now using only the gathered evidence. \
             No tools are available. Preserve the authoritative source boundary; do not invent missing evidence.",
            history,
        )
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
    streaming: bool,
    provider: &str,
    limits: AdmissionLimits,
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
    tokio::time::timeout_at(
        codelet_cli::compaction_timing::synthesis_deadline(),
        async {
            let bytes = agent
                .model
                .prepare_request(request.clone())
                .map_err(|error| error.to_string())?;
            admit_complete_request(&bytes, limits).map_err(|error| error.to_string())?;
            if streaming {
                let mut stream = agent
                    .model
                    .stream(request)
                    .await
                    .map_err(|error| error.to_string())?;
                let mut response = String::new();
                let mut snapshot = 0;
                while let Some(item) = stream.next().await {
                    match item.map_err(|error| error.to_string())? {
                        StreamedAssistantContent::Text(text) => response.push_str(&text.text),
                        StreamedAssistantContent::ResponseAttemptStarted => {
                            snapshot = response.len()
                        }
                        StreamedAssistantContent::ResponseAttemptAborted => {
                            response.truncate(snapshot)
                        }
                        StreamedAssistantContent::ToolCall(_)
                        | StreamedAssistantContent::ToolCallDelta { .. } => {
                            return Err("Compactor synthesis requested an unavailable tool".into());
                        }
                        _ => {}
                    }
                }
                Ok(response)
            } else {
                let response = agent
                    .model
                    .completion(request)
                    .await
                    .map_err(|error| error.to_string())?;
                let mut text = String::new();
                for content in response.choice {
                    match content {
                        AssistantContent::Text(fragment) => text.push_str(&fragment.text),
                        AssistantContent::ToolCall(_) => {
                            return Err("Compactor synthesis requested an unavailable tool".into());
                        }
                        _ => {}
                    }
                }
                Ok(text)
            }
        },
    )
    .await
    .map_err(|_| {
        tracing::warn!(operation_id = ?codelet_cli::compaction_timing::operation_id(),
            phase = "synthesis", provider, retryable = true,
            "Compaction phase deadline exceeded; fresh user input required for retry");
        "compaction synthesis deadline exceeded; submit fresh input to retry".to_string()
    })?
}
