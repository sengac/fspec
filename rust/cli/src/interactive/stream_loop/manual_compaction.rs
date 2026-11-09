//! BUG-516: manual `/compact` through the compactor sub-agent round.
//!
//! The product owner chose to run manual compaction through the same
//! compactor sub-agent round automatic pre-prompt compaction uses
//! ([`crate::compactor_sub_agent`]), instead of the RPC-418 in-view
//! 'Continue' turn that the BUG-214 inference gate refuses. No ordinary
//! inference runs: the round commits the summary (CompactionComplete) or
//! reports 'Compaction failed: <reason>' and leaves the conversation as it
//! was, then the run ends with Done.

use super::super::output::StreamOutput;
use crate::{compaction_cancellation::CompactionCancellation, session::Session};
use codelet_core::{RigAgent, TokenState};
use rig::completion::CompletionModel;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

pub(super) async fn run<Model, Output>(
    agent: &RigAgent<Model>,
    session: &mut Session,
    output: &Output,
    session_id: uuid::Uuid,
    compaction_in_progress: &Arc<AtomicBool>,
    cancellation: &CompactionCancellation,
) -> anyhow::Result<()>
where
    Model: CompletionModel,
    Output: StreamOutput,
{
    // The commit replaces chat history through the selected request
    // preparer, exactly as pre-prompt compaction installs it (no prompt is
    // pending, so nothing is re-appended).
    let crate::model_limits::RequiredLimits {
        context_window,
        max_output_tokens,
    } = crate::model_limits::required_limits(session.provider_manager())?;
    let preparer = agent
        .selected_request_preparer(
            "",
            codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(context_window),
                output_reservation: Some(max_output_tokens),
            },
        )
        .await?;
    session.install_selected_preparer(preparer);
    // A builder that failed earlier is restarted, as the next input does
    // (no-op unless a failed round left the session blocked and nothing is
    // pending durable reconciliation).
    session.restart_failed_compaction_builder();
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        compaction_needed: true,
        ..Default::default()
    }));
    // The round reports its own outcome: CompactionComplete on success,
    // 'Compaction failed: <reason>' on failure.
    let round = crate::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
        session,
        session_id,
        &token_state,
        compaction_in_progress,
        output,
        cancellation,
    )
    .await;
    if let Err(failure) = &round {
        tracing::warn!(%session_id, %failure, "[BUG-516] manual compaction failed");
    }
    output.emit_done();
    Ok(())
}
