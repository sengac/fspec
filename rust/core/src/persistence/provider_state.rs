pub(crate) mod legacy;
pub(crate) mod replay;
mod storage;
mod types;

use super::{load_session, update_session_manifest, ManifestCommitError, SessionManifest};
use serde_json::Value;
use std::collections::HashMap;
pub use types::{ProviderStateError, ProviderStateRef};
use uuid::Uuid;

fn transaction_error(error: ProviderStateError) -> ManifestCommitError {
    ManifestCommitError::Store(error.to_string())
}

pub fn migrate_provider_state(session_id: Uuid) -> Result<(), ProviderStateError> {
    let manifest = load_session(session_id).map_err(ProviderStateError::Storage)?;
    if storage::load(&manifest)?.is_some() || legacy::collect(&manifest)?.checkpoints.is_empty() {
        return Ok(());
    }
    update_session_manifest(session_id, |current| {
        let state = match storage::load(current).map_err(transaction_error)? {
            Some(_) => return Ok(()),
            None => legacy::collect(current).map_err(transaction_error)?,
        };
        storage::cleanup(current).map_err(transaction_error)?;
        current.provider_state = Some(storage::publish(&state).map_err(transaction_error)?);
        Ok(())
    })?;
    Ok(())
}

pub fn persist_provider_state(
    session_id: Uuid,
    provider: &str,
    mut metadata: HashMap<String, Value>,
) -> Result<SessionManifest, ProviderStateError> {
    metadata.insert("provider".into(), Value::String(provider.into()));
    Ok(update_session_manifest(session_id, |current| {
        let mut state = match storage::load(current).map_err(transaction_error)? {
            Some(state) => state,
            None => legacy::collect(current).map_err(transaction_error)?,
        };
        state
            .upsert(
                current,
                metadata.into_iter().collect(),
                current.messages.len(),
            )
            .map_err(transaction_error)?;
        storage::cleanup(current).map_err(transaction_error)?;
        current.provider_state = Some(storage::publish(&state).map_err(transaction_error)?);
        Ok(())
    })?)
}
