use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use codelet_core::compaction::validate_dag_structure;
use codelet_core::persistence::compaction_boundary::CompactionSource;
use codelet_core::TokenState;
use tracing::Instrument;
use uuid::Uuid;

use crate::compaction_cancellation::CompactionCancellation;
use crate::compaction_operation::{CompactionFailure, CompactionOperation};
use crate::interactive::output::StreamOutput;
use crate::session::Session;

pub type CompactorSubAgentHandler = Arc<
    dyn Fn(Uuid, CompactionSource) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

static COMPACTOR_SUB_AGENT_HANDLERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, CompactorSubAgentHandler>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

static TERMINAL_OVERFLOW_OWNERS: once_cell::sync::Lazy<RwLock<HashSet<Uuid>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashSet::new()));

pub fn set_terminal_overflow_owner(session_id: Uuid, present: bool) {
    match TERMINAL_OVERFLOW_OWNERS.write() {
        Ok(mut owners) => {
            if present {
                owners.insert(session_id);
            } else {
                owners.remove(&session_id);
            }
        }
        Err(error) => tracing::error!(%error, "terminal overflow owner registration failed"),
    }
}

pub fn has_terminal_overflow_owner(session_id: Uuid) -> bool {
    TERMINAL_OVERFLOW_OWNERS
        .read()
        .is_ok_and(|owners| owners.contains(&session_id))
}

pub fn set_compactor_sub_agent_handler(
    session_id: Uuid,
    handler: Option<CompactorSubAgentHandler>,
) {
    match COMPACTOR_SUB_AGENT_HANDLERS.write() {
        Ok(mut registry) => match handler {
            Some(handler) => {
                registry.insert(session_id, handler);
            }
            None => {
                registry.remove(&session_id);
            }
        },
        Err(error) => tracing::error!(%error, "compaction builder registration failed"),
    }
}

pub fn has_compactor_sub_agent_handler(session_id: Uuid) -> bool {
    COMPACTOR_SUB_AGENT_HANDLERS
        .read()
        .is_ok_and(|registry| registry.contains_key(&session_id))
}

pub fn clear_all_compactor_sub_agent_handlers() {
    match COMPACTOR_SUB_AGENT_HANDLERS.write() {
        Ok(mut registry) => registry.clear(),
        Err(error) => tracing::error!(%error, "compaction builder cleanup failed"),
    }
}

struct BusyOwnership(Arc<AtomicBool>);

impl Drop for BusyOwnership {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub async fn run_compactor_sub_agent_round<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    token_state: &Arc<Mutex<TokenState>>,
    compaction_in_progress: &Arc<AtomicBool>,
    output: &O,
) -> Result<(), CompactionFailure> {
    run_round(
        session,
        session_id,
        token_state,
        compaction_in_progress,
        output,
        None,
    )
    .await
}

pub async fn run_compactor_sub_agent_round_cancellable<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    token_state: &Arc<Mutex<TokenState>>,
    compaction_in_progress: &Arc<AtomicBool>,
    output: &O,
    cancellation: &CompactionCancellation,
) -> Result<(), CompactionFailure> {
    run_round(
        session,
        session_id,
        token_state,
        compaction_in_progress,
        output,
        Some(cancellation),
    )
    .await
}

async fn run_round<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    token_state: &Arc<Mutex<TokenState>>,
    compaction_in_progress: &Arc<AtomicBool>,
    output: &O,
    cancellation: Option<&CompactionCancellation>,
) -> Result<(), CompactionFailure> {
    let check = || cancellation.map_or(Ok(()), CompactionCancellation::check);
    let operation_id = session
        .compaction_operation
        .get_or_insert_with(|| {
            CompactionOperation::new(crate::interactive::deep_search_wall_clock_timeout(), 3)
        })
        .begin()?;
    let deadline = session
        .compaction_operation
        .as_ref()
        .and_then(CompactionOperation::deadline_at)
        .ok_or(CompactionFailure::Blocked)?;
    let check_before_commit = || {
        check()?;
        if tokio::time::Instant::now() >= deadline {
            Err(CompactionFailure::Deadline)
        } else {
            Ok(())
        }
    };
    session
        .block_compaction_inference("compaction operation requires durable reconciliation".into());
    compaction_in_progress.store(true, Ordering::Release);
    let _ownership = BusyOwnership(compaction_in_progress.clone());
    output.emit_compaction_started();
    output.emit_compaction_progress("Compaction builder working", 0, 0);
    let captured = session.capture_durable_compaction(session_id);
    let operation_span = tracing::info_span!("compaction_operation", %operation_id, %session_id,
        provider = session.current_provider_name(),
        model = session.current_model_id().as_deref().unwrap_or("default"));
    let result = session
        .compaction_operation
        .as_mut()
        .ok_or(CompactionFailure::Blocked)?
        .execute(async {
            check()?;
            let source = captured
                .as_ref()
                .map_err(|error| CompactionFailure::Commit(error.to_string()))?;
            token_state
                .lock()
                .map_err(|_| CompactionFailure::Registry)?
                .compaction_needed = true;
            let handler = COMPACTOR_SUB_AGENT_HANDLERS
                .read()
                .map_err(|_| CompactionFailure::Registry)?
                .get(&session_id)
                .cloned()
                .ok_or(CompactionFailure::Unavailable)?;
            let builder = handler(session_id, source.clone());
            let content = if let Some(cancellation) = cancellation {
                tokio::select! {
                    biased;
                    _ = cancellation.cancelled() => return Err(CompactionFailure::Cancelled),
                    result = builder => result,
                }
            } else {
                builder.await
            }
            .map_err(CompactionFailure::Builder)?;
            check()?;
            validate_dag_structure(&content)
                .map_err(|error| CompactionFailure::Candidate(error.to_string()))?;
            Ok(content)
        })
        .instrument(operation_span)
        .await;
    let result = result.and_then(|content| {
        check_before_commit()?;
        let source = captured.map_err(|error| CompactionFailure::Commit(error.to_string()))?;
        let mut state = token_state
            .lock()
            .map_err(|_| CompactionFailure::Registry)?;
        let original_tokens = session.token_tracker.input_tokens;
        let publication = crate::compaction_publication::CompactionPublication::new(session);
        let committed = session
            .commit_selected_compaction_guarded(
                source,
                operation_id,
                &content,
                || {
                    check_before_commit().map_err(|error| {
                        codelet_providers::request_admission::RequestAdmissionError::Preparation(
                            error.to_string(),
                        )
                    })
                },
                |boundary| publication.publish(output, boundary),
            )
            .map_err(|error| {
                if error.promoted_identity().is_some() {
                    return CompactionFailure::Commit(error.to_string());
                }
                check_before_commit()
                    .err()
                    .unwrap_or_else(|| CompactionFailure::Commit(error.to_string()))
            })?;
        let boundary = committed
            .compaction_commit
            .as_ref()
            .ok_or_else(|| CompactionFailure::Commit("missing committed generation".into()))?;
        state.input_tokens = boundary.input_tokens;
        state.output_tokens = 0;
        state.compaction_needed = false;
        session
            .compaction_operation
            .as_mut()
            .ok_or(CompactionFailure::Blocked)?
            .committed(operation_id)?;
        check()?;
        output.emit_committed_compaction_complete(codelet_rpc_types::CompactionResult {
            original_tokens: u32::try_from(original_tokens).unwrap_or(u32::MAX),
            compacted_tokens: u32::try_from(boundary.input_tokens).unwrap_or(u32::MAX),
            compression_ratio: crate::interactive_helpers::compression_ratio(
                original_tokens,
                boundary.input_tokens,
            ) * 100.0,
            turns_summarized: u32::try_from(boundary.covered_prefix.len()).unwrap_or(u32::MAX),
            turns_kept: u32::try_from(boundary.verified_suffix.len()).unwrap_or(u32::MAX),
        });
        Ok(())
    });
    if let Err(error) = &result {
        if session.ensure_compaction_inference_ready().is_err() {
            session.block_compaction_inference(error.to_string());
        }
        if let Some(operation) = &mut session.compaction_operation {
            operation.block(error.clone());
        }
        output.emit_compaction_failed(&error.to_string());
    }
    result
}

pub async fn retry_failed_compactor_for_user_input<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    compaction_in_progress: &Arc<AtomicBool>,
    interrupted: &Arc<AtomicBool>,
    interrupt_notify: &Arc<tokio::sync::Notify>,
    output: &O,
) -> Result<bool, CompactionFailure> {
    if !session.restart_failed_compaction_builder() {
        return Ok(false);
    }
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        compaction_needed: true,
        ..Default::default()
    }));
    let cancellation =
        CompactionCancellation::new(interrupted.clone(), Some(interrupt_notify.clone()));
    run_compactor_sub_agent_round_cancellable(
        session,
        session_id,
        &token_state,
        compaction_in_progress,
        output,
        &cancellation,
    )
    .await
    .map(|()| true)
}
