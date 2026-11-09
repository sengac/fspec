use super::admission::{self, settle};
use super::state::{LoopState, PendingInput};
use codelet_core::lifecycle_hooks::{run_user_prompt, HookMessageLevel};
use codelet_core::persistence::input_receipts::InputDisposition;
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
        queued_inputs: session.queued_inputs(),
        reason: session
            .interruption_reason()
            .unwrap_or(codelet_rpc_types::InterruptionReason::User),
    });
    session.set_status(SessionStatus::Idle);
    session.handle_output(StreamChunk::done());
    true
}

/// Settle a canceled submission durably before announcing its completion.
pub(super) async fn cancel_pending(
    session: &BackgroundSession,
    pending: &PendingInput,
    cancelled_at_submission: bool,
) -> bool {
    if !cancelled_at_submission && !session.compaction_cancelled(pending.interrupt_epoch) {
        return false;
    }
    settle(session, pending, InputDisposition::Cancelled).await;
    finish_cancelled_prompt(session, pending.interrupt_epoch, cancelled_at_submission)
}

/// BUG-516: a manual /compact request is not a conversation input: it takes
/// no receipt, runs no prompt hooks and stores no original. Cancellation
/// before it starts discards it (BUG-214).
async fn admit_manual_compaction(
    session: &BackgroundSession,
    pending: &PendingInput,
    state: &mut LoopState,
) -> bool {
    if let Some(epoch) = pending.manual_compaction_epoch {
        if session.discard_cancelled_compaction(epoch).await {
            session.handle_output(StreamChunk::done());
            return false;
        }
    }
    if finish_cancelled_prompt(
        session,
        pending.interrupt_epoch,
        pending.cancelled_at_submission,
    ) {
        return false;
    }
    state.turn_epoch = pending.interrupt_epoch;
    true
}

pub(super) async fn admit_prompt(
    session: &BackgroundSession,
    pending: &mut PendingInput,
    state: &mut LoopState,
) -> bool {
    session.wait_for_settlement().await;
    if pending.is_manual_compaction {
        return admit_manual_compaction(session, pending, state).await;
    }
    if !admission::ensure_receipt(session, pending) {
        session.handle_output(StreamChunk::error(
            "Input receipt retention failed; the input was not admitted".into(),
        ));
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    let pending = &*pending;
    let input = &pending.input.text;
    if pending.is_fresh_user_input {
        session.admit_input_epoch(pending.interrupt_epoch);
    }
    if let Some(epoch) = pending.manual_compaction_epoch {
        if session.discard_cancelled_compaction(epoch).await {
            settle(session, pending, InputDisposition::Cancelled).await;
            session.handle_output(StreamChunk::done());
            return false;
        }
    }
    if pending.is_compaction_retry && session.is_interrupted.load(Ordering::Acquire) {
        settle(session, pending, InputDisposition::Cancelled).await;
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
                let reason = format!("{error:#}");
                drop(inner_session);
                settle(session, pending, InputDisposition::Refused { reason }).await;
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
                let refused = InputDisposition::Refused {
                    reason: reason.clone(),
                };
                settle(session, pending, refused).await;
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
            settle(session, pending, InputDisposition::Cancelled).await;
            session.handle_output(StreamChunk::done());
            return false;
        }
    }
    if pending.is_compaction_retry && session.is_interrupted.load(Ordering::Acquire) {
        settle(session, pending, InputDisposition::Cancelled).await;
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    if cancel_pending(session, pending, pending.cancelled_at_submission).await {
        return false;
    }
    if let Err(error) = admission::store_original(session, pending).await {
        tracing::error!(
            "Failed to persist user message for session {}: {}",
            session.id,
            error
        );
        settle(
            session,
            pending,
            InputDisposition::Refused { reason: error },
        )
        .await;
        session.handle_output(StreamChunk::error(
            "Conversation persistence requires recovery before inference".into(),
        ));
        session.set_status(SessionStatus::Idle);
        session.handle_output(StreamChunk::done());
        return false;
    }
    if cancel_pending(session, pending, pending.cancelled_at_submission).await {
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
