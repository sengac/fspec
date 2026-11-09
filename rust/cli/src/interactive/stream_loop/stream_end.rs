use super::{
    super::{output::StreamOutput, stream_handlers::handle_final_response},
    annotations::process_turn_annotations,
    state::{Context, State},
};

pub(super) fn handle<Output: StreamOutput>(
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<()> {
    tracing::debug!(
        "[stream_loop] Stream ended (None) - assistant_text_len={}, checking compaction_needed",
        state.assistant_text.len()
    );
    if let Ok(tokens) = context.token_state.lock() {
        tracing::debug!(
            "[stream_loop] At stream end: compaction_needed={}, input={}, output={}",
            tokens.compaction_needed,
            tokens.input_tokens,
            tokens.output_tokens
        );
    }
    if !state.assistant_text.is_empty() {
        handle_final_response(&state.assistant_text, &mut context.session.messages)?;
    }
    process_turn_annotations(
        context.session,
        &mut state.turn_tool_infos,
        &mut state.previous_turn_tool_infos,
    );
    context
        .output
        .emit_done_with_stop_reason(state.final_stop_reason.take());
    Ok(())
}
