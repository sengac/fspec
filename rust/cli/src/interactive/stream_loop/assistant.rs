use super::{
    super::{
        output::{StreamEvent, StreamOutput},
        stream_handlers::{handle_text_chunk, handle_tool_call},
    },
    state::{Context, State},
};
use codelet_core::compaction::annotation_detector::ToolCallInfo;
use rig::streaming::StreamedAssistantContent;

pub(super) fn handle<Response, Output: StreamOutput>(
    item: StreamedAssistantContent<Response>,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<()> {
    match item {
        StreamedAssistantContent::ResponseAttemptStarted => {
            state.input_observation = Default::default();
            state.response_attempt_snapshot = Some((
                state.assistant_text.len(),
                state.accumulated_reasoning.len(),
                state.streaming_display.clone(),
            ));
            context.output.emit(StreamEvent::ResponseAttemptStarted);
        }
        StreamedAssistantContent::ResponseAttemptAborted => {
            state.input_observation = Default::default();
            if let Some((text, reasoning, display)) = state.response_attempt_snapshot.take() {
                state.assistant_text.truncate(text);
                state.accumulated_reasoning.truncate(reasoning);
                state.streaming_display = display;
            }
            context.output.emit(StreamEvent::ResponseAttemptAborted);
        }
        StreamedAssistantContent::Text(text) => {
            state.received(context.output);
            handle_text_chunk(
                &text.text,
                &mut state.assistant_text,
                Some(&context.request_id),
                context.output,
            )?;
            if let Some(update) = state.streaming_display.record_chunk(&text.text) {
                context.output.emit_tokens(&update.into());
            }
        }
        StreamedAssistantContent::ToolCall(tool_call) => {
            state.received(context.output);
            handle_tool_call(
                &tool_call,
                &mut context.session.messages,
                &mut state.assistant_text,
                &mut state.tool_calls_buffer,
                &mut state.last_tool_name,
                context.output,
            )?;
            state.turn_tool_infos.push(ToolCallInfo {
                tool_name: tool_call.function.name.clone(),
                input: tool_call.function.arguments.clone(),
                output: None,
                success: true,
            });
            state.auto_continue_segment_tool_calls += 1;
            if let Ok(mut active) = context.active_tool_call_id.lock() {
                *active = Some(tool_call.id.clone());
            }
            state.tool_execution_in_progress = true;
        }
        StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
            state.received(context.output);
            context.output.emit_thinking(&reasoning);
            state.accumulated_reasoning.push_str(&reasoning);
            if let Some(update) = state.streaming_display.record_chunk(&reasoning) {
                context.output.emit_tokens(&update.into());
            }
        }
        _ => tracing::debug!("[stream_loop] Unknown stream item received (ignored)"),
    }
    Ok(())
}
