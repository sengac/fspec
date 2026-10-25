use super::super::SessionManifest;
use super::types::{invalid, State};
use super::{legacy, storage, ProviderStateError, ProviderStateRef};

pub(crate) fn publish(
    source: &SessionManifest,
    child: &SessionManifest,
) -> Result<Option<ProviderStateRef>, ProviderStateError> {
    let state = match storage::load(source)? {
        Some(state) => state,
        None => legacy::collect(source)?,
    };
    if state.checkpoints.is_empty() {
        return if source.provider.split('/').next() == Some("codex") {
            Err(invalid(
                "unsupported fork boundary: missing native checkpoint",
            ))
        } else {
            Ok(None)
        };
    }
    if source.compaction.is_some() || source.compaction_commit.is_some() {
        return Err(invalid(
            "unsupported fork boundary: compacted native lineage",
        ));
    }
    let checkpoint = state
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.prefix_len == child.messages.len())
        .max_by_key(|checkpoint| checkpoint.generation)
        .ok_or_else(|| invalid("unsupported fork boundary: no exact native checkpoint"))?;
    let mut fork = State::empty(child.id);
    fork.upsert(child, checkpoint.metadata.clone(), child.messages.len())?;
    if let Some(reanchored) = fork.checkpoints.last_mut() {
        reanchored.reset = checkpoint.reset.clone();
    }
    fork.validate(child)?;
    Ok(Some(storage::publish(&fork)?))
}
