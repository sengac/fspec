use codelet_cli::session::Session;
use codelet_sessions::{background_session::BackgroundSession, session_manager::SessionManager};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use uuid::Uuid;

use crate::compaction_builder::execute_captured_compaction_subagent;
pub use crate::compaction_builder::{execute_compaction_subagent, COMPACTION_SUB_AGENT_MAX_DEPTH};
use crate::compaction_evidence::CompactionEvidence;
use crate::turn_compaction::{ActivityOwner, TurnCompaction};

#[derive(Clone)]
struct Builder {
    project: PathBuf,
    provider: String,
    model: Option<String>,
    capacity: Option<usize>,
    output: Option<usize>,
}

impl Builder {
    fn capture(inner: &Session, project: PathBuf) -> Self {
        Self {
            project,
            provider: inner
                .provider_manager()
                .facade_override()
                .unwrap_or(inner.current_provider_name())
                .to_string(),
            model: inner.current_model_id(),
            capacity: Some(inner.provider_manager().context_window()),
            output: Some(inner.provider_manager().max_output_tokens()),
        }
    }

    #[tracing::instrument(skip_all, fields(target_session_id = %target.id, caller_session_id = %caller.id))]
    async fn run(
        &self,
        target: &BackgroundSession,
        caller: &BackgroundSession,
        evidence: CompactionEvidence,
    ) -> Result<(String, tokio::time::Instant), String> {
        let budget = codelet_cli::interactive::deep_search_wall_clock_timeout();
        let deadline = codelet_cli::compaction_timing::deadline()
            .unwrap_or_else(|| tokio::time::Instant::now() + budget);
        let generation = codelet_cli::compaction_timing::scope(
            Uuid::new_v4(),
            deadline,
            budget,
            execute_captured_compaction_subagent(
                &self.project,
                evidence,
                Some(caller.id),
                &target.compaction_in_progress,
                &self.provider,
                self.model.as_deref(),
                self.capacity,
                self.output,
            ),
        );
        tokio::select! {
            biased;
            _ = interrupted(caller) => Err("compaction caller cancelled".into()),
            _ = interrupted(target) => Err("compaction target cancelled".into()),
            result = generation => result.map(|content| (content, deadline))
                .map_err(|error| format!("compaction target={} failed: {error}", target.id)),
        }
    }
}

async fn interrupted(session: &BackgroundSession) {
    loop {
        let notified = session.interrupt_notify.notified();
        if session.is_interrupted.load(Ordering::Acquire) {
            return;
        }
        notified.await;
    }
}

pub fn register_compactor_sub_agent_handler(
    session_id: Uuid,
    inner_session: &Session,
    project_path: PathBuf,
    compaction_in_progress: Arc<AtomicBool>,
) {
    let builder = Builder::capture(inner_session, project_path);
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(move |target, source| {
            let builder = builder.clone();
            let flag = compaction_in_progress.clone();
            Box::pin(async move {
                let evidence = CompactionEvidence::capture(source)?;
                if evidence.source().session_id != target {
                    return Err("compaction target identity mismatch".into());
                }
                execute_captured_compaction_subagent(
                    &builder.project,
                    evidence,
                    Some(session_id),
                    &flag,
                    &builder.provider,
                    builder.model.as_deref(),
                    builder.capacity,
                    builder.output,
                )
                .await
            })
        })),
    );
}

pub fn register_generate_compaction_handler(
    session_id: Uuid,
    inner_session: &Session,
    project_path: PathBuf,
    session: Arc<BackgroundSession>,
    owning_manager: Option<Arc<SessionManager>>,
) -> Arc<TurnCompaction> {
    let builder = Builder::capture(inner_session, project_path);
    let queue = TurnCompaction::capture(inner_session, &session);
    let injection = queue.clone();
    codelet_tools::set_inject_summary_handler(
        session_id,
        Some(Arc::new(move |target, content| {
            if target != session_id {
                return Err("submission target identity mismatch".into());
            }
            injection.submit(content)
        })),
    );
    let self_queue = queue.clone();
    codelet_tools::set_generate_compaction_handler(
        session_id,
        Some(Arc::new(move |target_id| {
            let builder = builder.clone();
            let caller = session.clone();
            let manager = owning_manager.clone();
            let self_queue = self_queue.clone();
            Box::pin(async move {
                if target_id == session_id {
                    let _activity = ActivityOwner::acquire(caller.clone())?;
                    let (content, deadline) = builder
                        .run(&caller, &caller, self_queue.evidence()?)
                        .await?;
                    self_queue.submit_with_deadline(content.clone(), Some(deadline))?;
                    return Ok(content);
                }
                let target = manager
                    .as_ref()
                    .ok_or_else(|| "target is not a live session".to_string())?
                    .get_session(&target_id.to_string())
                    .map_err(|error| error.to_string())?;
                let queue = {
                    let inner = target
                        .inner
                        .try_lock()
                        .map_err(|_| "target capture busy".to_string())?;
                    TurnCompaction::capture(&inner, &target)
                };
                let _activity = ActivityOwner::acquire(target.clone())?;
                let (content, deadline) = builder.run(&target, &caller, queue.evidence()?).await?;
                let mut inner = target
                    .inner
                    .try_lock()
                    .map_err(|_| "target commit busy".to_string())?;
                queue.submit_with_deadline(content.clone(), Some(deadline))?;
                queue.apply_unless_cancelled(&mut inner, &target, || {
                    caller.is_interrupted.load(Ordering::Acquire)
                })?;
                Ok(content)
            })
        })),
    );
    queue
}
