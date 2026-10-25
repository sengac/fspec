use futures::StreamExt;
use rig::completion::{GetTokenUsage, RequestContext, Usage};
use rig::message::AssistantContent;
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamedAssistantContent, StreamingCompletionResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct CopilotStreamResponse {
    usage: Option<Usage>,
    context_usage: Option<Usage>,
    request_context: Option<RequestContext>,
    request_usage: Vec<(String, Usage)>,
    stop_reason: Option<String>,
}

impl GetTokenUsage for CopilotStreamResponse {
    fn token_usage(&self) -> Option<Usage> {
        self.usage
    }
    fn context_usage(&self) -> Option<Usage> {
        self.context_usage
    }
    fn request_context(&self) -> Option<&RequestContext> {
        self.request_context.as_ref()
    }
    fn request_usage_records(&self) -> Vec<(String, Usage)> {
        self.request_usage.clone()
    }
    fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }
}

pub(super) fn adapt<Response>(
    mut source: StreamingCompletionResponse<Response>,
    mut attempt: super::super::session::Attempt,
) -> StreamingCompletionResponse<CopilotStreamResponse>
where
    Response: Clone + Unpin + GetTokenUsage + Send + Sync + 'static,
{
    StreamingCompletionResponse::stream(Box::pin(async_stream::stream! {
        let mut text = String::new();
        let mut reasoning_blocks = Vec::new();
        let mut tools = Vec::new();
        let mut terminal = None;
        while let Some(item) = source.next().await {
            let item = match item {
                Ok(item) => item,
                Err(error) => { yield Err(error); return; }
            };
            let mapped = match item {
                StreamedAssistantContent::ResponseAttemptStarted => RawStreamingChoice::ResponseAttemptStarted,
                StreamedAssistantContent::ResponseAttemptAborted => RawStreamingChoice::ResponseAttemptAborted,
                StreamedAssistantContent::Text(delta) => {
                    text.push_str(&delta.text);
                    RawStreamingChoice::Message(delta.text)
                }
                StreamedAssistantContent::ToolCall(call) => { tools.push(call); continue; }
                StreamedAssistantContent::ToolCallDelta {id,content} => RawStreamingChoice::ToolCallDelta {id,content},
                StreamedAssistantContent::Reasoning(reasoning) => {
                    if reasoning.signature.is_some() {
                        reasoning_blocks.push(AssistantContent::Reasoning(reasoning.clone()));
                    }
                    for text in reasoning.reasoning {
                        yield Ok(RawStreamingChoice::Reasoning {
                            id:reasoning.id.clone(), reasoning:text, signature:reasoning.signature.clone(),
                        });
                    }
                    continue;
                }
                StreamedAssistantContent::ReasoningDelta {id,reasoning} => RawStreamingChoice::ReasoningDelta {id,reasoning},
                StreamedAssistantContent::Usage(usage) => RawStreamingChoice::Usage(usage),
                StreamedAssistantContent::Final(response) => {
                    terminal = Some(CopilotStreamResponse {
                        usage:response.token_usage(), context_usage:response.context_usage(),
                        request_context:response.request_context().cloned(), request_usage:response.request_usage_records(),
                        stop_reason:response.stop_reason().map(str::to_owned),
                    });
                    continue;
                }
            };
            yield Ok(mapped);
        }
        let Some(terminal) = terminal else {
            yield Err(rig::completion::CompletionError::ResponseError("Copilot stream missing terminal evidence".into()));
            return;
        };
        if !text.is_empty() || tools.is_empty() {
            reasoning_blocks.push(AssistantContent::text(&text));
        }
        reasoning_blocks.extend(tools.iter().cloned().map(AssistantContent::ToolCall));
        if let Err(error) = attempt.settle(reasoning_blocks) { yield Err(error); return; }
        yield Ok(RawStreamingChoice::FinalResponse(terminal));
        for call in tools {
            yield Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall {
                id:call.id, call_id:call.call_id, name:call.function.name,
                arguments:call.function.arguments, signature:call.signature, additional_params:call.additional_params,
            }));
        }
    }))
}
