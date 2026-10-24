use super::compaction_boundary::*;
use super::manifest::{CompactionState, SessionManifest};
use super::{update_session_manifest, ManifestCommitError};
use crate::compaction::{validate_dag_candidate, DagValidationContext, ValidatedDagCandidate};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CompactionCommitError {
    #[error("compaction candidate rejected: {0}")]
    Rejected(String),
    #[error(transparent)]
    Persistence(#[from] ManifestCommitError),
    #[error("generation {identity:?} was promoted; publication failed: {reason}")]
    Publication {
        identity: CompactionIdentity,
        reason: String,
    },
    #[error("generation {identity:?} was promoted; durability acknowledgement failed: {source}")]
    Promoted {
        identity: CompactionIdentity,
        #[source]
        source: ManifestCommitError,
    },
}

impl CompactionCommitError {
    pub fn promoted_identity(&self) -> Option<&CompactionIdentity> {
        match self {
            Self::Publication { identity, .. } | Self::Promoted { identity, .. } => Some(identity),
            _ => None,
        }
    }
}

fn finish_commit(
    result: Result<SessionManifest, ManifestCommitError>,
    identity: Option<CompactionIdentity>,
) -> Result<SessionManifest, CompactionCommitError> {
    match (result, identity) {
        (Err(source @ ManifestCommitError::Promoted { .. }), Some(identity)) => {
            Err(CompactionCommitError::Promoted { identity, source })
        }
        (result, _) => Ok(result?),
    }
}

pub struct CompactionCandidate {
    pub operation_id: Uuid,
    pub source: CompactionSource,
    pub dag: ValidatedDagCandidate,
    pub checkpoint: CompactionCheckpoint,
    pub input_tokens: u64,
    pub output_reservation: u64,
    pub token_provenance: CompactionTokenProvenance,
}

pub fn commit_compaction(
    candidate: CompactionCandidate,
    selected_model: &str,
    admit: impl FnOnce(&SessionManifest, &CompactionCandidate) -> Result<(), String>,
) -> Result<SessionManifest, CompactionCommitError> {
    commit_compaction_admitted(candidate, selected_model, |current, candidate| {
        admit(current, candidate)?;
        Ok(CompactionAdmission {
            input_tokens: candidate.input_tokens,
            output_reservation: candidate.output_reservation,
            provenance: candidate.token_provenance.clone(),
        })
    })
}

pub struct CompactionAdmission {
    pub input_tokens: u64,
    pub output_reservation: u64,
    pub provenance: CompactionTokenProvenance,
}

pub fn commit_compaction_admitted(
    candidate: CompactionCandidate,
    selected_model: &str,
    admit: impl FnOnce(&SessionManifest, &CompactionCandidate) -> Result<CompactionAdmission, String>,
) -> Result<SessionManifest, CompactionCommitError> {
    let mut rejection = None;
    let mut promoted_identity = None;
    let result = update_session_manifest(candidate.source.session_id, |current| {
        let validate = || -> Result<CompactionAdmission, String> {
            validate_committed_boundary(current)?;
            validate_original_references(current)?;
            let generation = current
                .compaction_commit
                .as_ref()
                .map_or(0, |commit| commit.identity.generation);
            let prefix: Vec<_> = current
                .messages
                .iter()
                .map(|message| message.message_id)
                .collect();
            if candidate.source.generation != generation
                || candidate.source.provider != current.provider
                || candidate.source.model != selected_model
                || !prefix.starts_with(&candidate.source.prefix)
                || current
                    .compaction_commit
                    .as_ref()
                    .is_some_and(|commit| commit.publication_pending)
            {
                return Err("stale source generation, prefix or model identity".into());
            }
            let next = generation.checked_add(1).ok_or("generation exhausted")?;
            if candidate.operation_id.is_nil()
                || candidate.source.protocol.is_empty()
                || candidate.checkpoint.generation != next
                || candidate.checkpoint.provider != current.provider
                || candidate.checkpoint.model != selected_model
            {
                return Err("incompatible candidate identity or checkpoint".into());
            }
            let validated = validate_dag_candidate(
                &candidate.dag.content,
                &DagValidationContext {
                    persisted_turn_count: candidate.source.prefix.len(),
                    required_coverage_end: candidate.source.prefix.len().checked_sub(1),
                    existing_dag: current
                        .compaction
                        .as_ref()
                        .map(|state| state.summary.clone()),
                    ..Default::default()
                },
            )
            .map_err(|error| error.to_string())?;
            if validated != candidate.dag {
                return Err("candidate content or metadata changed after validation".into());
            }
            let admitted = admit(current, &candidate)?;
            if admitted.output_reservation == 0 {
                return Err("missing admitted output reservation".into());
            }
            Ok(admitted)
        };
        let admitted = validate().map_err(|reason| {
            rejection = Some(reason);
            ManifestCommitError::InvalidUpdate
        })?;
        let generation = candidate.source.generation + 1;
        let covered = candidate.source.prefix.len();
        current.compaction = Some(CompactionState {
            summary: candidate.dag.content,
            compacted_before_index: covered,
            compacted_at: chrono::Utc::now(),
        });
        current.compaction_commit = Some(CommittedGeneration {
            version: 1,
            identity: CompactionIdentity {
                session_id: current.id,
                generation,
                operation_id: candidate.operation_id,
                revision: current
                    .revision
                    .checked_add(1)
                    .ok_or(ManifestCommitError::RevisionExhausted)?,
            },
            covered_prefix: candidate.source.prefix.clone(),
            verified_suffix: current
                .messages
                .iter()
                .skip(covered)
                .map(|message| message.message_id)
                .collect(),
            source: candidate.source,
            checkpoint: candidate.checkpoint,
            input_tokens: admitted.input_tokens,
            output_reservation: admitted.output_reservation,
            token_provenance: admitted.provenance,
            publication_pending: true,
        });
        promoted_identity = current
            .compaction_commit
            .as_ref()
            .map(|commit| commit.identity.clone());
        current.token_usage.current_context_tokens = admitted.input_tokens;
        Ok(())
    });
    if let Some(reason) = rejection {
        return Err(CompactionCommitError::Rejected(reason));
    }
    finish_commit(result, promoted_identity)
}

pub fn reconcile_compaction(
    identity: &CompactionIdentity,
) -> Result<SessionManifest, CompactionCommitError> {
    let mut rejected = false;
    let result = update_session_manifest(identity.session_id, |current| {
        if validate_committed_boundary(current)
            .and_then(|()| validate_original_references(current))
            .is_err()
        {
            rejected = true;
            return Err(ManifestCommitError::InvalidUpdate);
        }
        if let Some(commit) = current
            .compaction_commit
            .as_mut()
            .filter(|commit| &commit.identity == identity)
        {
            commit.publication_pending = false;
            Ok(())
        } else {
            rejected = true;
            Err(ManifestCommitError::InvalidUpdate)
        }
    });
    if rejected {
        return Err(CompactionCommitError::Rejected(
            "reconciliation boundary is invalid or identity is not authoritative".into(),
        ));
    }
    finish_commit(result, Some(identity.clone()))
}

pub fn publish_compaction(
    committed: &SessionManifest,
    publish: impl FnOnce(&CommittedGeneration) -> Result<(), String>,
) -> Result<SessionManifest, CompactionCommitError> {
    let identity = &committed
        .compaction_commit
        .as_ref()
        .ok_or_else(|| CompactionCommitError::Rejected("missing promoted generation".into()))?
        .identity;
    let mut failure = None;
    let result = update_session_manifest(identity.session_id, |current| {
        let checked = (|| {
            validate_committed_boundary(current).map_err(CompactionCommitError::Rejected)?;
            validate_original_references(current).map_err(CompactionCommitError::Rejected)?;
            let boundary = current
                .compaction_commit
                .as_mut()
                .filter(|boundary| &boundary.identity == identity && boundary.publication_pending)
                .ok_or_else(|| {
                    CompactionCommitError::Rejected(
                        "publication identity is not authoritative and pending".into(),
                    )
                })?;
            publish(boundary).map_err(|reason| CompactionCommitError::Publication {
                identity: boundary.identity.clone(),
                reason,
            })?;
            boundary.publication_pending = false;
            Ok(())
        })();
        checked.map_err(|error| {
            failure = Some(error);
            ManifestCommitError::InvalidUpdate
        })
    });
    if let Some(error) = failure {
        return Err(error);
    }
    finish_commit(result, Some(identity.clone()))
}
