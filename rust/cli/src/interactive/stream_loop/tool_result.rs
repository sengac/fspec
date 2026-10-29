use super::{
    super::{
        interruption,
        output::StreamOutput,
        stream_handlers::{handle_final_response, handle_tool_result},
    },
    annotations::process_turn_annotations,
    state::{Context, Control, State},
};
use rig::message::ToolResult;

pub(super) fn handle<Output: StreamOutput>(
    tool_result: ToolResult,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<Control> {
    handle_tool_result(
        &tool_result,
        &mut context.session.messages,
        &mut state.tool_calls_buffer,
        &state.last_tool_name,
        context.output,
    )?;
    if let Some(info) = state
        .turn_tool_infos
        .iter_mut()
        .rev()
        .find(|info| info.output.is_none())
    {
        let result_text = tool_result
            .content
            .clone()
            .into_iter()
            .map(|content| match content {
                rig::message::ToolResultContent::Text(text) => text.text,
                rig::message::ToolResultContent::Image(_) => "[Image]".to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        info.success = !super::super::stream_handlers::detect_tool_error(&result_text);
        info.output = Some(result_text);
    }
    if let Ok(mut active) = context.active_tool_call_id.lock() {
        *active = None;
    }
    state.tool_execution_in_progress = false;
    let handoff = super::super::compaction_handoff::requires_handoff(
        state.last_tool_name.as_deref(),
        state.turn_tool_infos.last().map(|info| &info.input),
        context.session_id,
        &tool_result,
    );
    match handoff {
        Ok(true) => {
            context.session.block_compaction_inference(
                "queued candidate awaits durable reconciliation".into(),
            );
            state.summary_handoff = true;
            return Ok(Control::Stop);
        }
        Err(error) => {
            context
                .session
                .block_compaction_inference(error.reason.clone());
            return Err(error.into());
        }
        Ok(false) => {}
    }
    Ok(Control::Next)
}

pub(super) async fn accepted<Output: StreamOutput>(
    context: &mut Context<'_, Output>,
    state: &mut State,
    summary: &str,
) -> anyhow::Result<()> {
    interruption::record_tool_cancellations(
        context.session,
        &mut state.tool_calls_buffer,
        context.output,
        interruption::CancellationReason::AcceptedCompletion,
    )
    .await?;
    if !state.assistant_text.is_empty() {
        handle_final_response(&state.assistant_text, &mut context.session.messages)?;
        state.assistant_text.clear();
    }
    process_turn_annotations(
        context.session,
        &mut state.turn_tool_infos,
        &mut state.previous_turn_tool_infos,
    );
    super::super::done_early_exit::apply_finish_with_summary(
        context.session,
        context.session_id,
        summary,
        context.output,
    );
    context.output.emit_done_with_stop_reason(Some(
        super::super::done_early_exit::DONE_EARLY_EXIT_STOP_REASON.to_owned(),
    ));
    Ok(())
}
