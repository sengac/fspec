use super::state::{LoopState, PendingInput};
use crate::{dispatch::InputWithImages, terminal_overflow_recovery::RetryInput};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{
    format_incoming_message, BackgroundSession, IncomingMessage,
};
use std::sync::atomic::Ordering;

pub(super) fn next_retry_input(
    session: &BackgroundSession,
    state: &mut LoopState,
) -> Option<PendingInput> {
    let retry = state.compaction_retry_input.take()?;
    let mut manual_compaction_epoch = None;
    let retry_text = match retry {
        RetryInput::AutomaticCompaction(text) => {
            tracing::info!("[AGENT-LOOP] Compaction watchdog: retrying with escalation input");
            text
        }
        RetryInput::LoopAbort(epoch) => {
            if !session.admit_loop_retry(epoch) {
                return None;
            }
            manual_compaction_epoch = Some(epoch);
            tracing::info!("[AGENT-LOOP] Loop detector: retrying with corrective input");
            "Continue".into()
        }
    };
    Some(PendingInput {
        input: InputWithImages {
            text: retry_text,
            thinking_config: None,
            images: None,
        },
        is_retry_input: true,
        is_fresh_user_input: false,
        is_compaction_retry: true,
        manual_compaction_epoch,
    })
}

pub(super) fn supervisor_input(
    session: &BackgroundSession,
    supervisor_input: IncomingMessage,
) -> InputWithImages {
    session
        .incoming_message_pending
        .fetch_sub(1, Ordering::Release);
    tracing::debug!(role = %supervisor_input.role_name, input_bytes = supervisor_input.message.len(), input = %supervisor_input.message, "agent_loop received supervisor input");
    let formatted = format_incoming_message(&supervisor_input);
    if let Some(ref images) = supervisor_input.images {
        let supervisor_images = images
            .iter()
            .map(|img| codelet_rpc_types::IncomingMessageImage {
                data: img.data.clone(),
                media_type: img.media_type.clone(),
            })
            .collect();
        session.handle_output(StreamChunk::incoming_message_with_images(
            formatted.clone(),
            supervisor_images,
        ));
    } else {
        session.handle_output(StreamChunk::incoming_message(formatted.clone()));
    }
    InputWithImages {
        text: formatted,
        thinking_config: None,
        images: supervisor_input.images,
    }
}
