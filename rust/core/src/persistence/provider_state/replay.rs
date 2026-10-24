use super::super::SessionManifest;
use super::{legacy, storage, types::*};
use serde_json::Value;

pub(crate) fn metadata(manifest: &SessionManifest) -> Result<Vec<Value>, ProviderStateError> {
    let state = match storage::load(manifest)? {
        Some(state) => state,
        None => legacy::collect(manifest)?,
    };
    let mut latest = std::collections::BTreeMap::new();
    for checkpoint in &state.checkpoints {
        if manifest.compaction_commit.is_none()
            && manifest.compaction.as_ref().is_some_and(|compaction| {
                checkpoint.prefix_len <= compaction.compacted_before_index
            })
        {
            continue;
        }
        if latest
            .get(&checkpoint.provider)
            .is_none_or(|prior: &&Checkpoint| prior.generation < checkpoint.generation)
        {
            latest.insert(&checkpoint.provider, checkpoint);
        }
    }
    let mut envelopes = Vec::new();
    for checkpoint in latest.into_values() {
        let committed = manifest.compaction_commit.as_ref().filter(|commit| {
            commit.checkpoint.provider.split('/').next() == Some(checkpoint.provider.as_str())
        });
        if let Some(commit) = committed {
            if checkpoint.generation < commit.identity.generation {
                continue;
            }
            if checkpoint.generation == commit.identity.generation
                && (checkpoint.metadata["codexContinuation"]["model"] != commit.checkpoint.model
                    || checkpoint.metadata["codexContinuation"]["protocol"]
                        != commit.source.protocol)
            {
                return Err(invalid("checkpoint differs from committed generation"));
            }
        }
        if let Some(reset) = &checkpoint.reset {
            if committed.is_none_or(|commit| checkpoint.generation > commit.identity.generation)
                && !checkpoint.metadata["codexContinuation"].is_null()
            {
                envelopes.push(serde_json::to_value(reset)?);
            }
        }
        let mut envelope = serde_json::to_value(&checkpoint.metadata)?;
        envelope["_providerState"] = serde_json::json!({"version":1,"stateId":checkpoint.state_id});
        envelopes.push(envelope);
    }
    Ok(envelopes)
}

pub(crate) fn strip_native_metadata(envelope: &mut Value) {
    if let Some(fields) = envelope.as_object_mut() {
        fields.remove("codexContinuation");
        fields.remove("_codexReset");
        fields.remove("_providerCheckpoint");
        fields.remove("_providerState");
        fields.remove("_committedCompaction");
    }
}
