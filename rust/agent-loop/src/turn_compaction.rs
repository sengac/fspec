use crate::compaction_evidence::CompactionEvidence;
use codelet_cli::session::Session;
use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_providers::request_admission::RequestAdmissionError;
use codelet_rpc_types::{CompactionResult, SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::inject_summary::InjectSummaryResult;
use std::sync::{atomic::Ordering, Arc, Mutex};
use uuid::Uuid;

struct PendingCandidate {
    content: String,
    deadline: Option<tokio::time::Instant>,
}

pub struct TurnCompaction {
    evidence: Result<CompactionEvidence, String>,
    operation_id: Uuid,
    pending: Mutex<Option<PendingCandidate>>,
    submitted: std::sync::atomic::AtomicBool,
    original_tokens: u32,
    threshold: f64,
    capacity: f64,
}

impl TurnCompaction {
    pub(crate) fn capture(inner: &Session, session: &BackgroundSession) -> Arc<Self> {
        let manager = inner.provider_manager();
        let capacity = manager.context_window() as u64;
        let threshold_config = manager
            .compaction_threshold_override()
            .map(|(kind, value)| {
                codelet_cli::compaction_threshold::CompactionThresholdConfig::from_type_value(
                    kind, value,
                )
            });
        let threshold = codelet_cli::compaction_threshold::resolve_compaction_threshold(
            capacity,
            manager.max_output_tokens() as u64,
            inner.current_model_id().as_deref(),
            threshold_config.as_ref(),
        );
        Arc::new(Self {
            evidence: inner
                .capture_durable_compaction(session.id)
                .map_err(|error| error.to_string())
                .and_then(CompactionEvidence::capture),
            operation_id: Uuid::new_v4(),
            pending: Mutex::new(None),
            submitted: std::sync::atomic::AtomicBool::new(false),
            original_tokens: u32::try_from(
                codelet_cli::interactive_helpers::tracked_pre_compaction_basis(
                    &inner.token_tracker,
                    &inner.messages,
                ),
            )
            .unwrap_or(u32::MAX),
            threshold: threshold as f64,
            capacity: capacity as f64,
        })
    }

    pub(crate) fn evidence(&self) -> Result<CompactionEvidence, String> {
        self.evidence.clone()
    }

    pub(crate) fn submit(&self, content: String) -> Result<InjectSummaryResult, String> {
        self.submit_with_deadline(content, None)
    }

    pub(crate) fn submit_with_deadline(
        &self,
        content: String,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<InjectSummaryResult, String> {
        let evidence = self.evidence.as_ref().map_err(Clone::clone)?;
        let source = evidence.source();
        let mut pending = self.pending.lock().map_err(|error| error.to_string())?;
        if self.submitted.load(Ordering::Acquire) {
            return Err("compaction candidate already submitted for this operation".into());
        }
        validate_dag_candidate(
            &content,
            &DagValidationContext {
                persisted_turn_count: source.prefix.len(),
                required_coverage_end: source.prefix.len().checked_sub(1),
                existing_dag: evidence.existing_dag().map(|(content, _)| content),
                ..Default::default()
            },
        )
        .map_err(|error| error.to_string())?;
        let injected_tokens = codelet_common::token_estimator::count_tokens(&content) as u64;
        *pending = Some(PendingCandidate { content, deadline });
        self.submitted.store(true, Ordering::Release);
        Ok(InjectSummaryResult {
            injected_tokens,
            remaining_budget: 0,
        })
    }

    pub fn apply(&self, inner: &mut Session, session: &BackgroundSession) -> Result<bool, String> {
        self.apply_unless_cancelled(inner, session, || false)
    }

    pub(crate) fn apply_unless_cancelled(
        &self,
        inner: &mut Session,
        session: &BackgroundSession,
        caller_cancelled: impl Fn() -> bool,
    ) -> Result<bool, String> {
        let is_cancelled = || session.is_interrupted.load(Ordering::Acquire) || caller_cancelled();
        let mut pending = self.pending.lock().map_err(|error| error.to_string())?;
        if pending.is_none() {
            return Ok(false);
        }
        let source = self.evidence.as_ref().map_err(Clone::clone)?.source();
        if source.session_id != session.id {
            return Err("compaction target identity mismatch".into());
        }
        if is_cancelled() {
            pending.take();
            inner.block_compaction_inference("compaction application cancelled".into());
            return Err("compaction application cancelled".into());
        }
        let candidate = pending.take();
        drop(pending);
        let Some(candidate) = candidate else {
            return Ok(false);
        };
        let check = || {
            if is_cancelled() {
                Err(RequestAdmissionError::Preparation(
                    "compaction application cancelled".into(),
                ))
            } else if candidate
                .deadline
                .is_some_and(|deadline| tokio::time::Instant::now() >= deadline)
            {
                Err(RequestAdmissionError::Preparation(
                    "compaction application deadline exceeded".into(),
                ))
            } else {
                Ok(())
            }
        };
        let committed = inner
            .commit_selected_compaction_guarded(
                source.clone(),
                self.operation_id,
                &candidate.content,
                check,
                |boundary| {
                    let manifest = codelet_core::persistence::load_session(session.id)?;
                    if manifest
                        .compaction_commit
                        .as_ref()
                        .map(|commit| &commit.identity)
                        != Some(&boundary.identity)
                    {
                        return Err("publication identity is not authoritative".into());
                    }
                    let snapshot = codelet_sessions::committed_publication::context_snapshot(
                        &manifest,
                        self.original_tokens,
                        self.threshold,
                        self.capacity,
                        true,
                    )
                    .map_err(|error| error.to_string())?;
                    session.handle_output(StreamChunk::CommittedCompaction { snapshot });
                    Ok(())
                },
            )
            .map_err(|error| error.to_string())?;
        if is_cancelled() {
            return Err("compaction application cancelled after promotion".into());
        }
        let boundary = committed
            .compaction_commit
            .as_ref()
            .ok_or("missing reconciled compaction generation")?;
        let compacted_tokens = u32::try_from(boundary.input_tokens).unwrap_or(u32::MAX);
        session.handle_output(StreamChunk::compaction_complete(CompactionResult {
            original_tokens: self.original_tokens,
            compacted_tokens,
            compression_ratio: codelet_cli::interactive_helpers::compression_ratio(
                u64::from(self.original_tokens),
                u64::from(compacted_tokens),
            ) * 100.0,
            turns_summarized: u32::try_from(boundary.covered_prefix.len()).unwrap_or(u32::MAX),
            turns_kept: u32::try_from(boundary.verified_suffix.len()).unwrap_or(u32::MAX),
        }));
        Ok(true)
    }
}

pub(crate) fn extract_committed_learnings(
    session_id: Uuid,
    provider: String,
    model: Option<String>,
) {
    let Ok(manifest) = codelet_core::persistence::load_session(session_id) else {
        return;
    };
    if manifest.compaction_commit.is_none()
        || codelet_core::persistence::compaction_boundary::validate_committed_boundary(&manifest)
            .is_err()
    {
        return;
    }
    let Some(state) = manifest.compaction else {
        return;
    };
    let text = codelet_core::compaction::parse_dag_nodes(&state.summary, None)
        .iter()
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        runtime.block_on(async move {
            let response =
                codelet_graph::call_learnings_extraction_llm(&provider, model.as_deref(), &text)
                    .await;
            codelet_graph::extract_learnings_from_dag(&text, response.as_deref()).await;
        });
    });
}

pub(crate) struct ActivityOwner {
    session: Arc<BackgroundSession>,
    previous: SessionStatus,
}

impl ActivityOwner {
    pub(crate) fn acquire(session: Arc<BackgroundSession>) -> Result<Self, String> {
        session
            .compaction_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "target compaction busy".to_string())?;
        let previous = session.get_status();
        session.set_status(SessionStatus::Compacting);
        session.update_compaction_progress("Compaction sub-agent working".into(), 0, 0);
        Ok(Self { session, previous })
    }
}

impl Drop for ActivityOwner {
    fn drop(&mut self) {
        self.session.set_status(self.previous);
        self.session.set_compaction_progress(None);
        self.session
            .compaction_in_progress
            .store(false, Ordering::Release);
    }
}
