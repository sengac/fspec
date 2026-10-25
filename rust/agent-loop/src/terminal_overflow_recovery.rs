use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use codelet_cli::session::Session;
use codelet_core::TokenState;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;
use thiserror::Error;
use uuid::Uuid;

pub(crate) enum RetryInput {
    AutomaticCompaction(String),
    LoopAbort(u64),
}

pub(crate) fn continuation(session: &BackgroundSession, inner: &Session) -> Option<RetryInput> {
    if session.is_interrupted.load(Ordering::Acquire) {
        session.set_status(SessionStatus::Idle);
        session.handle_output(codelet_rpc_types::StreamChunk::done());
        return None;
    }
    if let Err(error) = inner.ensure_compaction_inference_ready() {
        session.set_status(SessionStatus::Idle);
        session.handle_output(codelet_rpc_types::StreamChunk::error(error.to_string()));
        session.handle_output(codelet_rpc_types::StreamChunk::done());
        return None;
    }
    session.set_status(SessionStatus::Running);
    Some(RetryInput::AutomaticCompaction("Continue the unfinished original task using the committed summary. Do not repeat completed tool actions.".into()))
}

pub struct CompactorRegistration(Uuid);

impl CompactorRegistration {
    pub fn new(session_id: Uuid) -> Self {
        codelet_cli::compactor_sub_agent::set_terminal_overflow_owner(session_id, true);
        Self(session_id)
    }
}

impl Drop for CompactorRegistration {
    fn drop(&mut self) {
        codelet_cli::compactor_sub_agent::set_terminal_overflow_owner(self.0, false);
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.0, None);
        codelet_tools::set_generate_compaction_handler(self.0, None);
        codelet_tools::set_inject_summary_handler(self.0, None);
    }
}

#[derive(Debug, Error)]
pub enum TerminalRecoveryError {
    #[error("terminal compaction builder unavailable")]
    Unavailable,
    #[error("terminal compaction cancelled")]
    Cancelled,
    #[error("terminal compaction failed: {0}")]
    Failed(String),
}

struct RecoveryOwnership<'session> {
    session: &'session BackgroundSession,
    previous: SessionStatus,
    completed: bool,
}

impl Drop for RecoveryOwnership<'_> {
    fn drop(&mut self) {
        let status = if self.completed && !self.session.is_interrupted.load(Ordering::Acquire) {
            self.previous
        } else {
            SessionStatus::Idle
        };
        self.session.set_status(status);
        self.session.set_compaction_progress(None);
        self.session
            .compaction_in_progress
            .store(false, Ordering::SeqCst);
    }
}

pub async fn try_terminal_overflow_recovery(
    session: Arc<BackgroundSession>,
    error: &anyhow::Error,
) -> Result<bool, TerminalRecoveryError> {
    if !codelet_cli::interactive::is_context_overflow_error(error) {
        return Ok(false);
    }
    let mut inner = session.inner.lock().await;
    let output = crate::background_output::BackgroundOutput::with_provider(
        session.clone(),
        inner.current_provider_name().to_string(),
        None,
    );
    try_terminal_overflow_recovery_owned(&session, &mut inner, error, &output).await
}

pub(crate) async fn try_terminal_overflow_recovery_owned<
    O: codelet_cli::interactive::StreamOutput,
>(
    session: &Arc<BackgroundSession>,
    inner: &mut Session,
    error: &anyhow::Error,
    output: &O,
) -> Result<bool, TerminalRecoveryError> {
    if !codelet_cli::interactive::is_context_overflow_error(error) {
        return Ok(false);
    }
    let mut ownership = RecoveryOwnership {
        session,
        previous: session.get_status(),
        completed: false,
    };
    inner.block_compaction_inference("terminal compaction requires durable reconciliation".into());
    if !codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler(session.id) {
        return Err(TerminalRecoveryError::Unavailable);
    }
    let cancellation = codelet_cli::compaction_cancellation::CompactionCancellation::new(
        session.is_interrupted.clone(),
        Some(session.interrupt_notify.clone()),
    );
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: inner.token_tracker.input_tokens,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: true,
    }));
    let recovery = codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
        inner,
        session.id,
        &token_state,
        &session.compaction_in_progress,
        output,
        &cancellation,
    );
    let result = recovery.await.map(|()| true).map_err(|error| match error {
        codelet_cli::compaction_operation::CompactionFailure::Cancelled => {
            TerminalRecoveryError::Cancelled
        }
        error => TerminalRecoveryError::Failed(error.to_string()),
    });
    ownership.completed = result.is_ok();
    result
}
