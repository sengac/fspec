use super::state::LoopState;
use crate::{persist::persist_pending_annotations, turn_compaction::TurnCompaction};
use codelet_cli::session::Session;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use std::sync::atomic::Ordering;

pub(super) fn reconcile_turn(
    session: &BackgroundSession,
    inner_session: &mut Session,
    turn_compaction: &TurnCompaction,
    result: &mut anyhow::Result<()>,
    state: &mut LoopState,
    current_provider: String,
    current_model: Option<String>,
) -> bool {
    persist_pending_annotations(&session.id, inner_session);
    let application = turn_compaction.apply(inner_session, session);
    let cancelled_after_reconciliation = session.is_interrupted.load(Ordering::Acquire)
        && inner_session.ensure_compaction_inference_ready().is_ok();
    let applied = match application {
        Ok(applied) => applied,
        Err(error) => {
            if !cancelled_after_reconciliation {
                inner_session.block_compaction_inference(error.clone());
            }
            *result = Err(anyhow::anyhow!(error));
            false
        }
    };
    if cancelled_after_reconciliation {
        state.compaction_retry_input = None;
    }
    if applied {
        crate::turn_compaction::extract_committed_learnings(
            session.id,
            current_provider,
            current_model,
        );
        session.set_compaction_progress(None);
    }
    let was_compacting = session.compaction_in_progress.load(Ordering::Acquire);
    if was_compacting && !applied && !cancelled_after_reconciliation {
        let failure = codelet_cli::compaction_operation::CompactionFailure::Exhausted;
        inner_session.block_compaction_inference(failure.to_string());
        session.handle_output(StreamChunk::error(failure.to_string()));
        state.compaction_retry_input = None;
        session.set_compaction_progress(None);
    }
    if state.compaction_retry_input.is_none() {
        let was_compacting = session.compaction_in_progress.swap(false, Ordering::SeqCst);
        if was_compacting {
            tracing::debug!(
                session_id = %session.id, current_status = ?session.get_status(),
                "[compaction-status] turn END — compaction flag was set → clearing + forcing Idle"
            );
            session.set_compaction_progress(None);
            if session.get_status() != SessionStatus::Idle {
                session.set_status(SessionStatus::Idle);
            }
        }
    } else {
        tracing::debug!(session_id = %session.id,
            "[compaction-status] turn END — watchdog retry pending; keeping compaction flag");
    }
    applied
}
