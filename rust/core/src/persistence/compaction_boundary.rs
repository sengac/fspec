use super::manifest::SessionManifest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionIdentity {
    pub session_id: Uuid,
    pub generation: u64,
    pub operation_id: Uuid,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionSource {
    pub session_id: Uuid,
    pub generation: u64,
    pub provider: String,
    pub model: String,
    pub protocol: String,
    pub prefix: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionCheckpoint {
    pub provider: String,
    pub model: String,
    pub generation: u64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionTokenProvenance {
    Estimated,
    ProviderMeasured,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedGeneration {
    pub version: u32,
    pub identity: CompactionIdentity,
    pub source: CompactionSource,
    pub covered_prefix: Vec<Uuid>,
    pub verified_suffix: Vec<Uuid>,
    pub checkpoint: CompactionCheckpoint,
    pub input_tokens: u64,
    pub output_reservation: u64,
    pub token_provenance: CompactionTokenProvenance,
    pub publication_pending: bool,
}

pub fn capture_compaction_source(
    manifest: &SessionManifest,
    model: &str,
    protocol: &str,
) -> Result<CompactionSource, super::compaction_commit::CompactionCommitError> {
    use super::compaction_commit::CompactionCommitError;
    use crate::compaction::{validate_dag_candidate, DagValidationContext};
    validate_committed_boundary(manifest).map_err(CompactionCommitError::Rejected)?;
    validate_original_references(manifest).map_err(CompactionCommitError::Rejected)?;
    if let Some(state) = &manifest.compaction {
        if state.compacted_before_index > manifest.messages.len() {
            return Err(CompactionCommitError::Rejected(
                "source coverage exceeds persisted history".into(),
            ));
        }
        validate_dag_candidate(
            &state.summary,
            &DagValidationContext {
                persisted_turn_count: manifest.messages.len(),
                required_coverage_end: state.compacted_before_index.checked_sub(1),
                existing_dag: Some(state.summary.clone()),
                ..Default::default()
            },
        )
        .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
    }
    if model.is_empty() || protocol.is_empty() || manifest.provider.is_empty() {
        return Err(CompactionCommitError::Rejected(
            "missing source identity".into(),
        ));
    }
    if manifest
        .compaction_commit
        .as_ref()
        .is_some_and(|commit| commit.publication_pending)
    {
        return Err(CompactionCommitError::Rejected(
            "unreconciled durable generation".into(),
        ));
    }
    Ok(CompactionSource {
        session_id: manifest.id,
        generation: manifest
            .compaction_commit
            .as_ref()
            .map_or(0, |commit| commit.identity.generation),
        provider: manifest.provider.clone(),
        model: model.into(),
        protocol: protocol.into(),
        prefix: manifest
            .messages
            .iter()
            .map(|message| message.message_id)
            .collect(),
    })
}

pub fn validate_committed_boundary(manifest: &SessionManifest) -> Result<(), String> {
    let Some(commit) = &manifest.compaction_commit else {
        return Ok(());
    };
    let state = manifest
        .compaction
        .as_ref()
        .ok_or("committed generation lacks DAG")?;
    let lineage: Vec<_> = manifest
        .messages
        .iter()
        .map(|message| message.message_id)
        .collect();
    let mut verified = commit.covered_prefix.clone();
    verified.extend(&commit.verified_suffix);
    if commit.version != 1
        || commit.identity.session_id != manifest.id
        || commit.identity.generation == 0
        || commit.identity.operation_id.is_nil()
        || commit.identity.revision == 0
        || commit.identity.revision > manifest.revision
        || commit.source.session_id != manifest.id
        || commit.source.generation.checked_add(1) != Some(commit.identity.generation)
        || commit.source.protocol.is_empty()
        || commit.source.model.is_empty()
        || commit.source.provider != manifest.provider
        || commit.output_reservation == 0
        || commit.covered_prefix != commit.source.prefix
        || !lineage.starts_with(&verified)
        || state.compacted_before_index != commit.covered_prefix.len()
        || commit.checkpoint.generation != commit.identity.generation
        || commit.checkpoint.model != commit.source.model
        || commit.checkpoint.provider != manifest.provider
    {
        return Err("incompatible committed generation boundary".into());
    }
    crate::compaction::validate_dag_candidate(
        &state.summary,
        &crate::compaction::DagValidationContext {
            persisted_turn_count: commit.covered_prefix.len(),
            required_coverage_end: commit.covered_prefix.len().checked_sub(1),
            ..Default::default()
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn validate_original_references(manifest: &SessionManifest) -> Result<(), String> {
    if manifest.messages.is_empty() {
        return Ok(());
    }
    let originals = super::MessageStore::new()?;
    for reference in &manifest.messages {
        let stored = originals.get(reference.message_id).ok_or_else(|| {
            format!(
                "missing committed message reference {}",
                reference.message_id
            )
        })?;
        let mut envelope = serde_json::Value::Object(stored.metadata.into_iter().collect());
        super::original_blobs::restore_original_blobs(&mut envelope).map_err(|error| {
            format!(
                "invalid committed message blob evidence {}: {error}",
                reference.message_id
            )
        })?;
    }
    Ok(())
}
