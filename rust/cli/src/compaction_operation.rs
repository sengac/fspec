use std::future::Future;
use std::time::Duration;

use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CompactionFailure {
    #[error("compaction builder unavailable")]
    Unavailable,
    #[error("compaction builder registry unavailable")]
    Registry,
    #[error("compaction builder failed: {0}")]
    Builder(String),
    #[error("invalid compaction candidate: {0}")]
    Candidate(String),
    #[error("compaction deadline exceeded")]
    Deadline,
    #[error("compaction cancelled")]
    Cancelled,
    #[error("compaction requires explicit resolution")]
    Blocked,
    #[error("compaction attempt budget exhausted")]
    Exhausted,
    #[error("compaction commit failed: {0}")]
    Commit(String),
}

impl CompactionFailure {
    /// The builder failed before anything was installed: a fresh builder may
    /// retry it, and a threshold compaction may carry on uncompacted (BUG-641).
    pub fn is_builder_failure(&self) -> bool {
        matches!(
            self,
            Self::Builder(_) | Self::Candidate(_) | Self::Deadline | Self::Unavailable
        )
    }
}

#[derive(Debug)]
pub struct CompactionOperation {
    identity: Option<Uuid>,
    attempts: usize,
    maximum_attempts: usize,
    deadline: Duration,
    deadline_at: Option<tokio::time::Instant>,
    blocked: bool,
    retryable_builder_failure: bool,
}

impl CompactionOperation {
    pub fn new(deadline: Duration, maximum_attempts: usize) -> Self {
        Self {
            identity: None,
            attempts: 0,
            maximum_attempts,
            deadline,
            deadline_at: None,
            blocked: false,
            retryable_builder_failure: false,
        }
    }

    pub fn begin(&mut self) -> Result<Uuid, CompactionFailure> {
        if self.blocked {
            return Err(CompactionFailure::Blocked);
        }
        if self.attempts >= self.maximum_attempts {
            self.blocked = true;
            return Err(CompactionFailure::Exhausted);
        }
        self.attempts += 1;
        self.deadline_at
            .get_or_insert_with(|| tokio::time::Instant::now() + self.deadline);
        Ok(*self.identity.get_or_insert_with(Uuid::new_v4))
    }

    pub async fn execute<Output>(
        &mut self,
        work: impl Future<Output = Result<Output, CompactionFailure>>,
    ) -> Result<Output, CompactionFailure> {
        if self.blocked || self.identity.is_none() {
            return Err(CompactionFailure::Blocked);
        }
        let deadline = self.deadline_at.ok_or(CompactionFailure::Blocked)?;
        if tokio::time::Instant::now() >= deadline {
            self.block(CompactionFailure::Deadline);
            return Err(CompactionFailure::Deadline);
        }
        let result = crate::compaction_timing::scope(
            self.identity.ok_or(CompactionFailure::Blocked)?,
            deadline,
            self.deadline,
            tokio::time::timeout_at(deadline, work),
        )
        .await
        .map_err(|_| CompactionFailure::Deadline)
        .and_then(std::convert::identity);
        if let Err(error) = &result {
            tracing::warn!(operation_id = ?self.identity, phase = "operation",
                effective_budget_ms = self.deadline.as_millis(), attempt = self.attempts,
                retryable = error.is_builder_failure(),
                failure = %error, "Compaction operation failed; automatic continuation blocked");
            self.block(error.clone());
        }
        result
    }

    pub fn block(&mut self, failure: CompactionFailure) {
        self.blocked = true;
        self.retryable_builder_failure = failure.is_builder_failure();
    }

    pub fn prevent_fresh_retry(&mut self) {
        self.retryable_builder_failure = false;
    }

    pub fn restart_failed_builder(&mut self) -> bool {
        if !self.blocked || !self.retryable_builder_failure {
            return false;
        }
        *self = Self::new(self.deadline, self.maximum_attempts);
        true
    }

    pub fn resolve(&mut self) {
        self.blocked = false;
        self.retryable_builder_failure = false;
    }

    pub fn committed(&mut self, identity: Uuid) -> Result<(), CompactionFailure> {
        if self.identity != Some(identity) {
            return Err(CompactionFailure::Commit(
                "operation identity mismatch".into(),
            ));
        }
        self.identity = None;
        self.deadline_at = None;
        self.attempts = 0;
        self.blocked = false;
        self.retryable_builder_failure = false;
        Ok(())
    }

    pub fn attempts(&self) -> usize {
        self.attempts
    }

    pub(crate) fn deadline_at(&self) -> Option<tokio::time::Instant> {
        self.deadline_at
    }

    pub fn is_blocked(&self) -> bool {
        self.blocked
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DagProtocol {
    InView,
}

pub fn selected_overflow_protocol(admitted: bool, _retry: u32) -> Option<DagProtocol> {
    admitted.then_some(DagProtocol::InView)
}

/// BUG-614 (Q7): what the compactor is asked to produce. A correction
/// continues the conversation that generated the rejected candidate: the
/// candidate is its assistant turn and this message, with the exact
/// validator error, is the next user turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildRequest {
    First,
    Correction(String),
}

/// Validate every candidate the builder returns, sending each rejection back
/// for correction within one limit of attempts (BUG-614 Q6). Nothing is
/// salvaged, clamped or invented.
pub async fn build_validated_dag<Build, Response>(
    context: &codelet_core::compaction::DagValidationContext,
    mut build: Build,
) -> Result<String, String>
where
    Build: FnMut(BuildRequest) -> Response,
    Response: Future<Output = Result<String, String>>,
{
    use crate::compaction_dag::{DAG_NODE_SYNTAX, DAG_RANGE_RULES};
    use codelet_core::compaction::{validate_dag_candidate, DagValidationError};
    use codelet_tools::inject_summary::MAX_SUMMARY_ATTEMPTS as ATTEMPTS;

    let operation = crate::compaction_timing::operation_id().unwrap_or_else(Uuid::new_v4);
    let mut request = BuildRequest::First;
    for attempt in 1..=ATTEMPTS {
        let candidate =
            crate::compaction_timing::attempt(attempt as u32, ATTEMPTS as u32, build(request))
                .await?;
        let phase = crate::compaction_timing::Phase::new("validation");
        let validated = validate_dag_candidate(&candidate, context);
        drop(phase);
        match validated {
            Ok(validated) => return Ok(validated.content),
            Err(error @ DagValidationError::InvalidSource(_)) => return Err(error.to_string()),
            Err(error) if attempt == ATTEMPTS => {
                return Err(format!(
                    "compaction operation={operation} candidate rejected after {attempt} attempts: {error}\n{DAG_NODE_SYNTAX}\n{DAG_RANGE_RULES}"
                ));
            }
            Err(error) => {
                let trimmed = candidate.trim_end();
                tracing::warn!(%operation, attempt, %error,
                    candidate_chars = candidate.chars().count(),
                    closed = trimmed.ends_with("</dag-node>") || trimmed.ends_with("</dag-files>"),
                    "compaction builder output requires correction");
                request = BuildRequest::Correction(format!(
                    "The previous candidate was rejected and was NOT installed.\nCorrection operation={operation} attempt={}/{ATTEMPTS}\nValidation error: {error}\nReturn a complete corrected DAG, not a patch or partial fragment. Preserve the authoritative source boundary and all existing durable payloads.\n{DAG_NODE_SYNTAX}\n{DAG_RANGE_RULES}",
                    attempt + 1,
                ));
            }
        }
    }
    Err("compaction candidate correction budget exhausted".into())
}
