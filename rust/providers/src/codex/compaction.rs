use super::checkpoint_restore;
use crate::codex::history::{CheckpointError, CodexCheckpoint, CodexHistory};
use crate::session::{ExtensionError, PreparedProviderGeneration};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    version: u32,
    durable_generation: u64,
    provider_generation: u64,
    source_generation: u64,
    source_state_id: String,
}

pub(super) fn prepare(
    history: &CodexHistory,
    durable_generation: u64,
    model: &str,
    envelopes: &[String],
) -> Result<PreparedProviderGeneration, ExtensionError> {
    let generation = history.generation()?;
    let checkpoint = history.snapshot()?;
    let uninitialized = generation == 0 && checkpoint.is_none();
    let has_metadata = envelopes.iter().try_fold(false, |present, envelope| {
        let value: Value = serde_json::from_str(envelope)?;
        Ok::<_, serde_json::Error>(
            present
                || value.get("codexContinuation").is_some()
                || value.get("_codexReset").is_some(),
        )
    })?;
    if !uninitialized || has_metadata {
        let selected = checkpoint_restore::select(envelopes, checkpoint.clone(), generation)?;
        if selected.generation != generation || selected.checkpoint != checkpoint {
            return Err(CheckpointError::Lineage(
                "runtime checkpoint differs from committed replay",
            )
            .into());
        }
    }
    if checkpoint
        .as_ref()
        .is_some_and(|saved| saved.model.as_deref() != Some(model))
    {
        return Err(CheckpointError::Lineage("selected checkpoint model").into());
    }
    let next = generation
        .checked_add(1)
        .ok_or(CheckpointError::Lineage("generation exhausted"))?;
    if durable_generation == 0 || next < durable_generation {
        return Err(CheckpointError::Lineage("durable generation exceeds provider lineage").into());
    }
    let source_state_id = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(generation, &checkpoint))?)
    );
    let checkpoint = history.prepare_generation(next, model)?;
    let binding = Binding {
        version: 1,
        durable_generation,
        provider_generation: next,
        source_generation: generation,
        source_state_id,
    };
    Ok(PreparedProviderGeneration {
        generation: next,
        metadata: [
            (
                "codexContinuation".into(),
                serde_json::to_value(checkpoint)?,
            ),
            ("_codexCompaction".into(), serde_json::to_value(binding)?),
        ]
        .into_iter()
        .collect(),
    })
}

pub fn committed_checkpoint(
    durable_generation: u64,
    metadata: &Value,
) -> Result<CodexCheckpoint, ExtensionError> {
    let checkpoint: CodexCheckpoint = serde_json::from_value(
        metadata
            .get("codexContinuation")
            .ok_or(CheckpointError::Missing)?
            .clone(),
    )?;
    if checkpoint.version != 1 {
        return Err(CheckpointError::UnsupportedVersion(checkpoint.version).into());
    }
    if durable_generation == 0
        || checkpoint.generation < durable_generation
        || checkpoint.protocol != "full-current-dag"
        || checkpoint.model.as_ref().is_none_or(String::is_empty)
        || !checkpoint.items.is_empty()
        || !checkpoint.compacted_users.is_empty()
    {
        return Err(CheckpointError::Lineage("committed provider generation").into());
    }
    match metadata.get("_codexCompaction") {
        Some(value) => {
            let binding: Binding = serde_json::from_value(value.clone())?;
            if binding.version != 1
                || binding.durable_generation != durable_generation
                || binding.provider_generation != checkpoint.generation
                || binding.source_generation.checked_add(1) != Some(checkpoint.generation)
                || binding.source_state_id.len() != 64
                || !binding
                    .source_state_id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(CheckpointError::Lineage("committed generation binding").into());
            }
        }
        None if checkpoint.generation != durable_generation => {
            return Err(CheckpointError::Lineage("missing committed generation binding").into());
        }
        None => {}
    }
    Ok(checkpoint)
}
