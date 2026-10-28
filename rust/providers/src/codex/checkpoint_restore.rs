use super::super::history::{CheckpointError, CodexCheckpoint};
use crate::session::ExtensionError;
use serde_json::Value;

pub(super) struct Selected {
    pub checkpoint: Option<CodexCheckpoint>,
    pub generation: u64,
}

pub(super) fn select(
    envelopes: &[String],
    previous: Option<CodexCheckpoint>,
    previous_generation: u64,
) -> Result<Selected, ExtensionError> {
    let mut boundary: Option<CodexCheckpoint> = None;
    let mut selected: Option<CodexCheckpoint> = None;
    let mut generation = 0;
    let mut reset = false;
    for (index, envelope) in envelopes.iter().enumerate() {
        let value: Value = serde_json::from_str(envelope)?;
        let checkpoint = value
            .get("codexContinuation")
            .map(|saved| serde_json::from_value::<Option<CodexCheckpoint>>(saved.clone()))
            .transpose()?;
        if let Some(marker) = value.get("_codexReset").filter(|marker| !marker.is_null()) {
            let next = marker["generation"]
                .as_u64()
                .ok_or(CheckpointError::Missing)?;
            if marker["version"].as_u64() != Some(1)
                || marker["protocol"] != "full-current-dag"
                || next == 0
                || checkpoint != Some(None)
                || value.get("_committedCompaction").is_some()
            {
                return Err(CheckpointError::Lineage("reset marker").into());
            }
            if boundary
                .as_ref()
                .is_some_and(|boundary| next < boundary.generation)
            {
                continue;
            }
            if next <= generation {
                return Err(CheckpointError::Lineage("reset marker generation").into());
            }
            generation = next;
            selected = None;
            boundary = None;
            reset = true;
            continue;
        }
        if let Some(checkpoint) = &checkpoint {
            let checkpoint = checkpoint.as_ref().ok_or(CheckpointError::Missing)?;
            validate(checkpoint)?;
        }
        if let Some(commit) = value.get("_committedCompaction") {
            super::compaction::committed_checkpoint(
                commit["generation"]
                    .as_u64()
                    .ok_or(CheckpointError::Missing)?,
                &value,
            )?;
            let checkpoint = checkpoint
                .as_ref()
                .and_then(Option::as_ref)
                .ok_or(CheckpointError::Missing)?;
            if index != 0
                || value["_synthetic"] != true
                || value["_compactionSummary"] != true
                || commit["version"].as_u64() != Some(1)
                || commit
                    .get("providerGeneration")
                    .unwrap_or(&commit["generation"])
                    .as_u64()
                    != Some(checkpoint.generation)
                || commit["generation"]
                    .as_u64()
                    .is_none_or(|generation| generation == 0 || generation > checkpoint.generation)
                || checkpoint.generation == 0
                || checkpoint.model.as_ref().is_none_or(String::is_empty)
                || !checkpoint.items.is_empty()
                || !checkpoint.compacted_users.is_empty()
            {
                return Err(CheckpointError::Lineage("committed replay boundary").into());
            }
            boundary = Some(checkpoint.clone());
        }
        if let Some(Some(checkpoint)) = checkpoint {
            if let Some(boundary) = &boundary {
                if checkpoint.generation > boundary.generation
                    || (checkpoint.generation == boundary.generation
                        && checkpoint.model != boundary.model)
                {
                    return Err(CheckpointError::Lineage("retained checkpoint identity").into());
                }
                if checkpoint.generation < boundary.generation {
                    continue;
                }
            }
            if checkpoint.generation < generation {
                return Err(CheckpointError::Lineage("checkpoint generation rollback").into());
            }
            generation = checkpoint.generation;
            selected = Some(checkpoint);
            reset = false;
        }
    }
    if generation < previous_generation || (selected.is_none() && !reset && previous.is_some()) {
        return Err(CheckpointError::Lineage("installed generation rollback").into());
    }
    if previous.as_ref().is_some_and(|previous| {
        selected.as_ref().is_some_and(|selected| {
            selected.generation == previous.generation
                && previous.model.is_some()
                && selected.model != previous.model
        })
    }) {
        return Err(CheckpointError::Lineage("installed generation model changed").into());
    }
    if selected.is_none() && !reset && !envelopes.is_empty() {
        return Err(CheckpointError::Missing.into());
    }
    if reset && generation == previous_generation && previous.is_some() {
        return Err(CheckpointError::Lineage("reset marker must advance").into());
    }
    Ok(Selected {
        checkpoint: selected,
        generation,
    })
}

pub(super) fn ordinary(provider: &str, envelopes: &[String]) -> Result<bool, ExtensionError> {
    if matches!(provider.split('/').next(), Some("codex" | "") | None) {
        return Ok(false);
    }
    for envelope in envelopes {
        let value: Value = serde_json::from_str(envelope)?;
        if value.get("codexContinuation").is_some() || value.get("_codexReset").is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate(checkpoint: &CodexCheckpoint) -> Result<(), CheckpointError> {
    if checkpoint.version != 1 {
        return Err(CheckpointError::UnsupportedVersion(checkpoint.version));
    }
    if checkpoint.protocol != "full-current-dag" {
        return Err(CheckpointError::Lineage("protocol"));
    }
    if !checkpoint.compacted_users.is_empty()
        || checkpoint
            .items
            .iter()
            .any(|item| matches!(item, super::super::history::CodexItem::Compaction { .. }))
    {
        return Err(CheckpointError::Lineage(
            "unsupported checkpoint memory mode",
        ));
    }
    Ok(())
}
