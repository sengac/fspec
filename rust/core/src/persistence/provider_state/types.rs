use super::super::{compute_hash, ManifestCommitError, SessionManifest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub(super) type Metadata = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStateRef {
    pub version: u32,
    pub state_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderStateError {
    #[error("incompatible provider state: {0}")]
    Incompatible(String),
    #[error("provider state storage unavailable: {0}")]
    Storage(String),
    #[error("provider state I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid provider state JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Commit(#[from] ManifestCommitError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Checkpoint {
    pub provider: String,
    pub generation: u64,
    pub source_revision: u64,
    pub prefix_len: usize,
    pub prefix_hash: String,
    pub state_id: String,
    pub metadata: Metadata,
    pub reset: Option<Metadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct State {
    pub version: u32,
    pub session_id: Uuid,
    pub checkpoints: Vec<Checkpoint>,
}

pub(super) fn invalid(message: &str) -> ProviderStateError {
    ProviderStateError::Incompatible(message.into())
}

pub(crate) fn prefix_hash(
    manifest: &SessionManifest,
    end: usize,
) -> Result<String, ProviderStateError> {
    let prefix = manifest
        .messages
        .get(..end)
        .ok_or_else(|| invalid("source prefix length"))?;
    Ok(compute_hash(&serde_json::to_vec(prefix)?))
}

pub(super) fn canonical_metadata(metadata: Metadata) -> Metadata {
    metadata
        .into_iter()
        .map(|(key, mut value)| {
            value.sort_all_objects();
            (key, value)
        })
        .collect()
}

pub(super) fn metadata_hash(metadata: &Metadata) -> Result<String, ProviderStateError> {
    Ok(compute_hash(&serde_json::to_vec(metadata)?))
}

pub(super) fn native_identity(metadata: &Metadata) -> Result<(String, u64), ProviderStateError> {
    let provider = metadata
        .get("provider")
        .and_then(Value::as_str)
        .filter(|provider| *provider == "codex")
        .ok_or_else(|| invalid("provider identity"))?;
    if metadata.contains_key("message") || metadata.contains_key("uuid") {
        return Err(invalid("checkpoint contains conversation fields"));
    }
    let checkpoint = metadata
        .get("codexContinuation")
        .ok_or_else(|| invalid("native checkpoint"))?;
    let identity = if checkpoint.is_null() {
        metadata
            .get("_codexReset")
            .filter(|reset| !reset.is_null())
            .ok_or_else(|| invalid("native reset identity"))?
    } else {
        checkpoint
    };
    let generation = identity["generation"]
        .as_u64()
        .ok_or_else(|| invalid("generation"))?;
    if identity["version"] != 1
        || identity["protocol"] != "full-current-dag"
        || (checkpoint.is_null() && generation == 0)
    {
        return Err(invalid("native version, protocol or reset generation"));
    }
    Ok((provider.into(), generation))
}

impl State {
    pub fn empty(session_id: Uuid) -> Self {
        Self {
            version: 1,
            session_id,
            checkpoints: Vec::new(),
        }
    }

    pub fn upsert(
        &mut self,
        manifest: &SessionManifest,
        metadata: Metadata,
        end: usize,
    ) -> Result<(), ProviderStateError> {
        let metadata = canonical_metadata(metadata);
        let (provider, generation) = native_identity(&metadata)?;
        if self
            .checkpoints
            .iter()
            .any(|saved| saved.provider == provider && saved.generation > generation)
        {
            return Err(invalid("generation rollback"));
        }
        let previous = self
            .checkpoints
            .iter()
            .position(|saved| saved.provider == provider && saved.generation == generation);
        let mut checkpoint = Checkpoint {
            provider,
            generation,
            source_revision: manifest.revision,
            prefix_len: end,
            prefix_hash: prefix_hash(manifest, end)?,
            state_id: metadata_hash(&metadata)?,
            reset: metadata
                .get("codexContinuation")
                .is_some_and(Value::is_null)
                .then(|| metadata.clone()),
            metadata,
        };
        if let Some(index) = previous {
            let prior = &self.checkpoints[index];
            if checkpoint.metadata == prior.metadata
                && checkpoint.prefix_hash == prior.prefix_hash
                && checkpoint.prefix_len == prior.prefix_len
            {
                return Ok(());
            }
            if !prior.metadata["codexContinuation"].is_null()
                && !prior.metadata["codexContinuation"]["model"].is_null()
                && !checkpoint.metadata["codexContinuation"].is_null()
                && prior.metadata["codexContinuation"]["model"]
                    != checkpoint.metadata["codexContinuation"]["model"]
            {
                return Err(invalid("model changed within generation"));
            }
            if checkpoint.reset.is_none() {
                checkpoint.reset = prior.reset.clone();
            }
            self.checkpoints[index] = checkpoint;
        } else {
            self.checkpoints.push(checkpoint);
            self.checkpoints.sort_by(|left, right| {
                (&left.provider, left.generation).cmp(&(&right.provider, right.generation))
            });
        }
        Ok(())
    }

    pub fn validate(&self, manifest: &SessionManifest) -> Result<(), ProviderStateError> {
        if self.version != 1 || self.session_id != manifest.id || self.checkpoints.is_empty() {
            return Err(invalid("state root identity"));
        }
        let mut identities = std::collections::HashSet::new();
        for checkpoint in &self.checkpoints {
            let (provider, generation) = native_identity(&checkpoint.metadata)?;
            if checkpoint.provider != provider
                || checkpoint.generation != generation
                || checkpoint.source_revision > manifest.revision
                || checkpoint.prefix_hash != prefix_hash(manifest, checkpoint.prefix_len)?
                || checkpoint.state_id != metadata_hash(&checkpoint.metadata)?
                || !identities.insert((provider.clone(), generation))
            {
                return Err(invalid("checkpoint identity or source anchor"));
            }
            if let Some(reset) = &checkpoint.reset {
                if native_identity(reset)? != (provider, generation)
                    || !reset["codexContinuation"].is_null()
                {
                    return Err(invalid("retained reset provenance"));
                }
            }
        }
        Ok(())
    }
}
