use super::super::completion::Content;
use super::{
    ContentDelta, StreamingCompletionResponse, StreamingEvent, ThinkingState, ToolCallState,
};
use crate::completion::CompletionError;
use crate::streaming::{RawStreamingChoice, RawStreamingToolCall, ToolCallDeltaContent};

pub(super) fn handle_event(
    event: &StreamingEvent,
    current_tool_call: &mut Option<ToolCallState>,
    current_thinking: &mut Option<ThinkingState>,
) -> Option<Result<RawStreamingChoice<StreamingCompletionResponse>, CompletionError>> {
    match event {
        StreamingEvent::ContentBlockDelta { delta, .. } => match delta {
            ContentDelta::TextDelta { text } => {
                if current_tool_call.is_none() {
                    return Some(Ok(RawStreamingChoice::Message(text.clone())));
                }
                None
            }
            ContentDelta::InputJsonDelta { partial_json } => {
                if let Some(tool_call) = current_tool_call {
                    tool_call.input_json.push_str(partial_json);
                    // Emit the delta so UI can show progress
                    return Some(Ok(RawStreamingChoice::ToolCallDelta {
                        id: tool_call.id.clone(),
                        content: ToolCallDeltaContent::Delta(partial_json.clone()),
                    }));
                }
                None
            }
            ContentDelta::ThinkingDelta { thinking } => {
                if current_thinking.is_none() {
                    *current_thinking = Some(ThinkingState::default());
                }
                if let Some(state) = current_thinking {
                    state.thinking.push_str(thinking);
                }
                Some(Ok(RawStreamingChoice::ReasoningDelta {
                    id: None,
                    reasoning: thinking.clone(),
                }))
            }
            ContentDelta::SignatureDelta { signature } => {
                if current_thinking.is_none() {
                    *current_thinking = Some(ThinkingState::default());
                }
                if let Some(state) = current_thinking {
                    state.signature.push_str(signature);
                }
                // Don't yield signature chunks, they will be included in the final Reasoning
                None
            }
            ContentDelta::CompactionDelta { content } => {
                // PROV-005-DEBUG: Server-side compaction delta received
                tracing::debug!(
                    "[anthropic/streaming] Server-side COMPACTION delta received - content_len={}",
                    content.len()
                );
                // For now, just log - we may need to emit this as a special event
                None
            }
            ContentDelta::Unknown => {
                // PROV-005-DEBUG: Unknown delta type
                tracing::debug!("[anthropic/streaming] Unknown ContentDelta type received");
                None
            }
        },
        StreamingEvent::ContentBlockStart {
            content_block,
            index,
        } => {
            // PROV-005-DEBUG: Log all content block starts
            tracing::debug!(
                "[anthropic/streaming] ContentBlockStart index={} type={:?}",
                index,
                std::mem::discriminant(content_block)
            );
            match content_block {
                Content::ToolUse { id, name, .. } => {
                    *current_tool_call = Some(ToolCallState {
                        name: name.clone(),
                        id: id.clone(),
                        input_json: String::new(),
                    });
                    Some(Ok(RawStreamingChoice::ToolCallDelta {
                        id: id.clone(),
                        content: ToolCallDeltaContent::Name(name.clone()),
                    }))
                }
                Content::Thinking { .. } => {
                    *current_thinking = Some(ThinkingState::default());
                    None
                }
                Content::Compaction { content } => {
                    // PROV-005-DEBUG: Server-side compaction block received
                    tracing::debug!(
                        "[anthropic/streaming] Server-side COMPACTION block received - content_len={}",
                        content.len()
                    );
                    // For now, just log - we may need to handle this specially
                    None
                }
                Content::Unknown => {
                    // PROV-005-DEBUG: Unknown content block type
                    tracing::debug!(
                        "[anthropic/streaming] Unknown Content type in ContentBlockStart"
                    );
                    None
                }
                // Handle other content types - they don't need special handling
                _ => None,
            }
        }
        StreamingEvent::ContentBlockStop { .. } => {
            if let Some(thinking_state) = Option::take(current_thinking)
                && !thinking_state.thinking.is_empty()
            {
                let signature = if thinking_state.signature.is_empty() {
                    None
                } else {
                    Some(thinking_state.signature)
                };
                return Some(Ok(RawStreamingChoice::Reasoning {
                    id: None,
                    reasoning: thinking_state.thinking,
                    signature,
                }));
            }
            if let Some(tool_call) = Option::take(current_tool_call) {
                let json_str = if tool_call.input_json.is_empty() {
                    "{}"
                } else {
                    &tool_call.input_json
                };
                match serde_json::from_str(json_str) {
                    Ok(json_value) => Some(Ok(RawStreamingChoice::ToolCall(
                        RawStreamingToolCall::new(tool_call.id, tool_call.name, json_value),
                    ))),
                    Err(error) => {
                        // PROV-039: At ContentBlockStop time, we don't yet know the stop_reason
                        // (MessageDelta with stop_reason arrives AFTER ContentBlockStop in Anthropic's SSE protocol).
                        // Emit the generic JSON parse error here. If the stream ends with max_tokens,
                        // the post-loop handler (above) will emit the enriched truncation error for
                        // any pending tool calls that were never closed via ContentBlockStop.
                        Some(Err(CompletionError::from(error)))
                    }
                }
            } else {
                None
            }
        }
        // Ignore other event types or handle as needed
        StreamingEvent::MessageStart { .. }
        | StreamingEvent::MessageDelta { .. }
        | StreamingEvent::MessageStop
        | StreamingEvent::Ping
        | StreamingEvent::Unknown => None,
    }
}
