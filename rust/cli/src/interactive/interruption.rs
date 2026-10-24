use super::message_helpers::add_assistant_tool_calls_message;
use super::output::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use crate::session::Session;
use codelet_providers::codex::{history::CodexItem, session::CodexSession};
use rig::message::{AssistantContent, Message, ToolCall, UserContent};
use std::collections::HashSet;

pub(super) enum CancellationReason {
    UserInterruption,
    LoopDetector,
    Unspecified,
    AcceptedCompletion,
}

fn invocation(call: &ToolCall) -> &str {
    call.call_id.as_deref().unwrap_or(&call.id)
}

pub(super) fn record_tool_cancellations<O: StreamOutput>(
    session: &mut Session,
    buffered: &mut Vec<AssistantContent>,
    output: &O,
    reason: CancellationReason,
) -> anyhow::Result<()> {
    let mut completed = HashSet::new();
    let mut published = HashSet::new();
    for message in &session.messages {
        match message {
            Message::User { content } => {
                for content in content.iter() {
                    if let UserContent::ToolResult(result) = content {
                        completed.insert(result.call_id.as_ref().unwrap_or(&result.id).clone());
                    }
                }
            }
            Message::Assistant { content, .. } => {
                for content in content.iter() {
                    if let AssistantContent::ToolCall(call) = content {
                        published.insert(invocation(call).to_owned());
                    }
                }
            }
        }
    }
    let mut pending: Vec<_> = buffered
        .iter()
        .filter_map(|content| match content {
            AssistantContent::ToolCall(call) => {
                published.insert(invocation(call).to_owned());
                Some(call.clone())
            }
            _ => None,
        })
        .collect();
    if session.current_provider_name() == "codex" {
        if let Some(extension) = session.provider_session.extension::<CodexSession>("codex") {
            if let Some(checkpoint) = extension.history.snapshot()? {
                for item in &checkpoint.items {
                    match item {
                        CodexItem::FunctionCallOutput { call_id, .. }
                        | CodexItem::CustomToolCallOutput { call_id, .. } => {
                            completed.insert(call_id.clone());
                        }
                        _ => {}
                    }
                }
                for item in &checkpoint.items {
                    if matches!(item,
                        CodexItem::FunctionCall { call_id, .. }
                        | CodexItem::CustomToolCall { call_id, .. }
                        if completed.contains(call_id)
                    ) {
                        continue;
                    }
                    if let Some(call) = extension.history.tool_call(item)? {
                        pending.push(call);
                    }
                }
            }
        }
    }
    if !buffered.is_empty() {
        add_assistant_tool_calls_message(&mut session.messages, std::mem::take(buffered))?;
    }
    for call in pending {
        let call_id = invocation(&call).to_owned();
        if completed.insert(call_id.clone()) {
            if published.insert(call_id.clone()) {
                add_assistant_tool_calls_message(
                    &mut session.messages,
                    vec![AssistantContent::ToolCall(call.clone())],
                )?;
                output.emit(StreamEvent::ToolCall(ToolCallEvent {
                    id: call.id.clone(),
                    call_id: call.call_id.clone(),
                    name: call.function.name.clone(),
                    args: call.function.arguments.clone(),
                }));
            }
            let content = match reason {
                CancellationReason::UserInterruption => {
                    "Tool execution cancelled by user interruption. Side effects may have occurred."
                }
                CancellationReason::LoopDetector => {
                    "Tool execution cancelled by loop detector. Side effects may have occurred."
                }
                CancellationReason::Unspecified => {
                    "Tool execution cancelled by interruption. Side effects may have occurred."
                }
                CancellationReason::AcceptedCompletion => {
                    "Tool execution cancelled after accepted completion. The tool was not executed."
                }
            };
            session.messages.push(Message::tool_result_with_call_id(
                &call.id,
                Some(call_id.clone()),
                content,
            ));
            output.emit_tool_cancellation(ToolResultEvent {
                id: call.id,
                call_id: Some(call_id),
                content: content.into(),
                is_error: true,
            });
        }
    }
    Ok(())
}
