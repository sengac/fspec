use super::{
    super::{
        interruption,
        output::{StreamEvent, StreamOutput},
        stream_handlers::handle_final_response,
    },
    state::{Context, State},
};

pub(super) async fn settle<Output: StreamOutput>(
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<()> {
    context.output.acknowledge_interruption();
    interruption::record_tool_cancellations(
        context.session,
        &mut state.tool_calls_buffer,
        context.output,
        match context.output.interruption_reason() {
            Some(codelet_rpc_types::InterruptionReason::User) => {
                interruption::CancellationReason::UserInterruption
            }
            Some(codelet_rpc_types::InterruptionReason::LoopDetector) => {
                interruption::CancellationReason::LoopDetector
            }
            None if context.cli_mode => interruption::CancellationReason::UserInterruption,
            None => interruption::CancellationReason::Unspecified,
        },
    )
    .await?;
    let queued = if let Some(input_queue) = &mut context.input_queue {
        input_queue.dequeue_all()
    } else {
        vec![]
    };
    interruption::publish_settlement(
        context.output,
        vec![StreamEvent::Interrupted(queued)],
        false,
    )
    .await?;
    if !state.assistant_text.is_empty() {
        handle_final_response(&state.assistant_text, &mut context.session.messages)?;
    }
    interruption::publish_settlement(
        context.output,
        vec![StreamEvent::Done(state.final_stop_reason.take())],
        false,
    )
    .await?;
    Ok(())
}
