use super::state::LoopState;
use crate::{background_output::BackgroundOutput, terminal_overflow_recovery::RetryInput};
use codelet_cli::session::Session;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;
use std::sync::Arc;

pub(super) async fn settle_turn_result(
    session: &Arc<BackgroundSession>,
    inner_session: &mut Session,
    result: anyhow::Result<()>,
    applied: bool,
    builder_retry_failed: bool,
    state: &mut LoopState,
    output: &BackgroundOutput,
) {
    if let Err(error) = result {
        let recovery = if builder_retry_failed {
            Ok(false)
        } else if codelet_cli::interactive::is_context_overflow_error(&error)
            && state.automatic_recoveries >= 3
        {
            Err(crate::terminal_overflow_recovery::TerminalRecoveryError::Failed(
                "automatic recovery budget exhausted; reduce the task context or select a larger model, then submit a new task".into(),
            ))
        } else {
            crate::terminal_overflow_recovery::try_terminal_overflow_recovery_owned(
                session,
                inner_session,
                &error,
                output,
            )
            .await
        };
        if matches!(recovery, Ok(true)) {
            state.automatic_recoveries += 1;
            state.compaction_retry_input =
                crate::terminal_overflow_recovery::continuation(session, inner_session);
        } else {
            let error = match recovery {
                Err(error) => anyhow::Error::new(error),
                _ => error,
            };
            tracing::warn!(
                "[AGENT-LOOP] ERROR received - session={}, error={}, error_chain={:?}",
                session.id,
                error,
                error
                    .chain()
                    .map(|cause| cause.to_string())
                    .collect::<Vec<_>>()
            );
            tracing::error!("Agent stream error for session {}: {}", session.id, error);
            output.finish_outer_failure(format!("{error:#}"));
        }
    } else if applied {
        if state.automatic_recoveries >= 3 {
            output.finish_outer_failure(
                "automatic recovery budget exhausted; submit a new task with reduced context"
                    .into(),
            );
        } else {
            state.automatic_recoveries += 1;
            state.compaction_retry_input =
                crate::terminal_overflow_recovery::continuation(session, inner_session);
        }
    } else {
        session.set_status(SessionStatus::Idle);
    }
}

pub(super) fn schedule_loop_retry(
    session: &BackgroundSession,
    output: &BackgroundOutput,
    state: &mut LoopState,
) {
    if session.has_pending_loop_abort_note() {
        let loop_abort_retry_cap = output.loop_wiring().max_retries as usize;
        if state.loop_abort_retry_count < loop_abort_retry_cap {
            state.loop_abort_retry_count += 1;
            tracing::info!(
                "[RIG-014] Loop abort auto-continue: injecting synthetic Continue input (session={}, retry {}/{})",
                session.id, state.loop_abort_retry_count, loop_abort_retry_cap
            );
            state.compaction_retry_input = Some(RetryInput::LoopAbort(session.compaction_epoch()));
        } else {
            tracing::warn!(
                "[RIG-014] Loop abort retry limit reached (session={}) — giving up, waiting for user input",
                session.id
            );
            let _ = session.take_pending_loop_abort_note();
        }
    }
}
