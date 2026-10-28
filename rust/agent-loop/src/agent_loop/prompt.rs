use super::state::{LoopState, PendingInput};
use crate::user_persistence::persist_user_input;
use codelet_core::lifecycle_hooks::{run_user_prompt, HookMessageLevel};
use codelet_rpc_types::{NotificationSeverity, SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use std::sync::atomic::Ordering;

pub(super) fn finish_cancelled_prompt(
    session: &BackgroundSession,
    epoch: u64,
    cancelled_at_submission: bool,
) -> bool {
    if !cancelled_at_submission && !session.compaction_cancelled(epoch) {
        return false;
    }
    session.handle_output(StreamChunk::InterruptedWithReason {
        queued_inputs: Vec::new(),
        reason: session
            .interruption_reason()
            .unwrap_or(codelet_rpc_types::InterruptionReason::User),
    });
    session.set_status(SessionStatus::Idle);
    session.handle_output(StreamChunk::done());
    true
}

pub(super) async fn admit_prompt(
    session: &BackgroundSession,
    pending: &PendingInput,
    state: &mut LoopState,
) -> bool {
    session.wait_for_settlement().await;
    let input = &pending.input.text;
    if pending.is_fresh_user_input {
        session.admit_input_epoch(pending.interrupt_epoch);
    }
    if let Some(epoch) = pending.manual_compaction_epoch {
        if session.discard_cancelled_compaction(epoch).await {
            session.handle_output(StreamChunk::done());
            return false;
        }
    }
    if pending.is_compaction_retry && session.is_interrupted.load(Ordering::Acquire) {
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    if !pending.is_retry_input && pending.is_fresh_user_input {
        if state.automatic_recoveries >= 3 {
            let mut inner_session = session.inner.lock().await;
            let restored = codelet_core::persistence::load_session(session.id)
                .map_err(anyhow::Error::msg)
                .and_then(|manifest| {
                    inner_session
                        .restore_compaction_boundary(&manifest)
                        .map_err(anyhow::Error::new)
                });
            if let Err(error) = restored {
                session.handle_output(StreamChunk::error(format!("{error:#}")));
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
                return false;
            }
        }
        state.loop_abort_retry_count = 0;
        state.automatic_recoveries = 0;
    }
    tracing::debug!(
        "Session {} processing input: {}",
        session.id,
        input.chars().take(50).collect::<String>()
    );
    if let Some(ref images) = pending.input.images {
        tracing::debug!(
            "Session {} has {} image(s) attached",
            session.id,
            images.len()
        );
    }
    if let Some(hooks) = session.lifecycle_hooks.as_ref().filter(|_| {
        !pending.cancelled_at_submission && !session.compaction_cancelled(pending.interrupt_epoch)
    }) {
        if !hooks.user_prompt_submit.is_empty() {
            let ctx = session.hook_context();
            let outcome = run_user_prompt(hooks, &ctx, input).await;
            for msg in &outcome.messages {
                if msg.level == HookMessageLevel::Warning || msg.level == HookMessageLevel::Error {
                    tracing::warn!(content = %msg.content, "[HOOK-013] user_prompt_submit hook warning or error");
                }
            }
            if !outcome.allow_prompt {
                let reason = outcome
                    .block_reason
                    .unwrap_or_else(|| "Blocked by hook".to_string());
                tracing::warn!("[HOOK-013] Prompt blocked: {}", reason);
                session.handle_output(StreamChunk::user_notification(
                    format!("Prompt blocked: {}", reason),
                    NotificationSeverity::Warning,
                ));
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
                return false;
            }
            if !outcome.additional_context.is_empty() {
                let mut inner_session = session.inner.lock().await;
                let combined_context = outcome.additional_context.join("\n");
                inner_session.add_system_reminder(
                    codelet_cli::session::SystemReminderType::FspecWorkflow,
                    &combined_context,
                );
                drop(inner_session);
            }
        }
    }
    if let Some(epoch) = pending.manual_compaction_epoch {
        if session.discard_cancelled_compaction(epoch).await {
            session.handle_output(StreamChunk::done());
            return false;
        }
    }
    if pending.is_compaction_retry && session.is_interrupted.load(Ordering::Acquire) {
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    let session_id = session.id;
    let persisted_input = input.clone();
    let images = pending.input.images.clone().unwrap_or_default();
    let persistence = tokio::task::spawn_blocking(move || {
        persist_user_input(&session_id, &persisted_input, &images)
    })
    .await;
    if let Err(error) = persistence
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(|error| error.to_string()))
    {
        tracing::error!(
            "Failed to persist user message for session {}: {}",
            session.id,
            error
        );
        session.handle_output(StreamChunk::error(
            "Conversation persistence requires recovery before inference".into(),
        ));
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    if finish_cancelled_prompt(
        session,
        pending.interrupt_epoch,
        pending.cancelled_at_submission,
    ) {
        return false;
    }
    session.set_status(SessionStatus::Running);
    state.turn_epoch = pending.interrupt_epoch;
    tracing::debug!(
        session_id = %session.id,
        input_chars = input.chars().count(),
        "[compaction-status] turn START — status → Running (thinking mode)"
    );
    true
}
