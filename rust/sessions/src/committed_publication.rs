use crate::background_session::BackgroundSession;
use codelet_core::persistence::compaction_boundary::{
    validate_committed_boundary, CompactionIdentity, CompactionTokenProvenance,
};
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::SessionManifest;
use codelet_rpc_types::{CommittedContextSnapshot, ContextFillInfo, StreamChunk, TokenTracker};
use std::sync::atomic::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum CommittedPublicationError {
    #[error("committed context boundary is invalid: {0}")]
    InvalidBoundary(String),
    #[error("committed context requires a durable generation")]
    MissingGeneration,
    #[error("committed context requires finite positive trigger and capacity limits")]
    InvalidLimits,
    #[error("committed context exceeds the event representation")]
    RepresentationOverflow,
}

impl BackgroundSession {
    pub async fn reconcile_durable_compaction(
        &self,
        identity: &CompactionIdentity,
    ) -> Result<SessionManifest, CompactionCommitError> {
        if identity.session_id != self.id {
            return Err(CompactionCommitError::Rejected(
                "reconciliation session differs".into(),
            ));
        }
        let mut inner = self.inner.lock().await;
        let manifest = codelet_core::persistence::load_session(self.id)
            .map_err(CompactionCommitError::Rejected)?;
        validate_committed_boundary(&manifest).map_err(CompactionCommitError::Rejected)?;
        let boundary = manifest
            .compaction_commit
            .as_ref()
            .filter(|boundary| &boundary.identity == identity)
            .ok_or_else(|| {
                CompactionCommitError::Rejected("identity is no longer authoritative".into())
            })?;
        if !boundary.publication_pending
            && self.committed_context_snapshot().is_some_and(|snapshot| {
                u64::try_from(snapshot.generation).ok() == Some(identity.generation)
            })
        {
            inner
                .ensure_compaction_inference_ready()
                .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
            return Ok(manifest);
        }
        let tokens = saved_tokens(&manifest)
            .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
        let mut published = manifest.clone();
        if let Some(boundary) = published.compaction_commit.as_mut() {
            boundary.publication_pending = false;
        }
        let snapshot = restored_snapshot(self, &published, &tokens)
            .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?
            .ok_or_else(|| {
                CompactionCommitError::Rejected("missing reconciliation snapshot".into())
            })?;
        let reconciled = inner.reconcile_durable_compaction(identity)?;
        self.handle_output(StreamChunk::CommittedCompaction { snapshot });
        Ok(reconciled)
    }
}

pub(crate) fn saved_tokens(
    manifest: &SessionManifest,
) -> Result<TokenTracker, CommittedPublicationError> {
    let saved = &manifest.token_usage;
    let convert =
        |value| u32::try_from(value).map_err(|_| CommittedPublicationError::RepresentationOverflow);
    Ok(TokenTracker {
        input_tokens: convert(saved.current_context_tokens)?,
        output_tokens: convert(saved.cumulative_billed_output)?,
        cache_read_input_tokens: Some(convert(saved.cache_read_tokens)?),
        cache_creation_input_tokens: Some(convert(saved.cache_creation_tokens)?),
        cumulative_billed_input: Some(convert(saved.cumulative_billed_input)?),
        cumulative_billed_output: Some(convert(saved.cumulative_billed_output)?),
        reasoning_tokens: Some(convert(saved.reasoning_tokens)?),
        tokens_per_second: None,
    })
}

pub(crate) fn restored_snapshot(
    session: &BackgroundSession,
    manifest: &SessionManifest,
    tokens: &TokenTracker,
) -> Result<Option<CommittedContextSnapshot>, CommittedPublicationError> {
    let Some(boundary) = manifest.compaction_commit.as_ref() else {
        return Ok(None);
    };
    if boundary.publication_pending {
        validate_committed_boundary(manifest)
            .map_err(CommittedPublicationError::InvalidBoundary)?;
        return Ok(None);
    }
    let mut snapshot = context_snapshot(
        manifest,
        session.pre_compaction_tokens.load(Ordering::Acquire),
        f64::from(session.cached_compaction_threshold.load(Ordering::Acquire)),
        f64::from(session.cached_context_window.load(Ordering::Acquire)),
        true,
    )?;
    snapshot.tokens = tokens.clone();
    snapshot.context_fill.effective_tokens = f64::from(tokens.input_tokens);
    snapshot.context_fill.fill_percentage =
        (f64::from(tokens.input_tokens) / snapshot.context_fill.threshold * 100.0) as u32;
    Ok(Some(snapshot))
}

pub fn context_snapshot(
    manifest: &SessionManifest,
    original_tokens: u32,
    trigger_threshold: f64,
    context_capacity: f64,
    reconciled: bool,
) -> Result<CommittedContextSnapshot, CommittedPublicationError> {
    validate_committed_boundary(manifest).map_err(CommittedPublicationError::InvalidBoundary)?;
    let committed = manifest
        .compaction_commit
        .as_ref()
        .ok_or(CommittedPublicationError::MissingGeneration)?;
    if !trigger_threshold.is_finite()
        || !context_capacity.is_finite()
        || trigger_threshold <= 0.0
        || trigger_threshold > context_capacity
    {
        return Err(CommittedPublicationError::InvalidLimits);
    }
    let input_tokens = u32::try_from(committed.input_tokens)
        .map_err(|_| CommittedPublicationError::RepresentationOverflow)?;
    let generation = i64::try_from(committed.identity.generation)
        .map_err(|_| CommittedPublicationError::RepresentationOverflow)?;
    let covered_turns = u32::try_from(committed.covered_prefix.len())
        .map_err(|_| CommittedPublicationError::RepresentationOverflow)?;
    let retained_turns = u32::try_from(
        manifest
            .messages
            .len()
            .saturating_sub(committed.covered_prefix.len()),
    )
    .map_err(|_| CommittedPublicationError::RepresentationOverflow)?;
    Ok(CommittedContextSnapshot {
        generation,
        tokens: TokenTracker {
            input_tokens,
            output_tokens: 0,
            reasoning_tokens: Some(0),
            ..TokenTracker::default()
        },
        context_fill: ContextFillInfo {
            fill_percentage: (f64::from(input_tokens) / trigger_threshold * 100.0) as u32,
            effective_tokens: f64::from(input_tokens),
            threshold: trigger_threshold,
            context_window: context_capacity,
        },
        original_tokens,
        covered_turns: Some(covered_turns),
        retained_turns: Some(retained_turns),
        estimated: committed.token_provenance == CompactionTokenProvenance::Estimated,
        reconciled,
    })
}
