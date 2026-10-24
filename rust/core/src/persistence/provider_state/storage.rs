use super::super::{
    compute_hash,
    messages::durability::{perform, sync_directory},
    SessionManifest,
};
use super::types::{invalid, ProviderStateError, ProviderStateRef, State};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};
use uuid::Uuid;

fn directory(session_id: Uuid) -> Result<PathBuf, ProviderStateError> {
    Ok(codelet_common::get_data_dir()
        .map_err(ProviderStateError::Storage)?
        .join("provider-state")
        .join(session_id.to_string()))
}

pub(super) fn load(manifest: &SessionManifest) -> Result<Option<State>, ProviderStateError> {
    let Some(reference) = &manifest.provider_state else {
        return Ok(None);
    };
    if reference.version != 1
        || reference.state_id.len() != 64
        || !reference
            .state_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid("manifest state reference"));
    }
    let bytes = fs::read(directory(manifest.id)?.join(format!("{}.json", reference.state_id)))?;
    if compute_hash(&bytes) != reference.state_id {
        return Err(invalid("state root content identity"));
    }
    let state: State = serde_json::from_slice(&bytes)?;
    state.validate(manifest)?;
    Ok(Some(state))
}

pub(super) fn cleanup(manifest: &SessionManifest) -> Result<(), ProviderStateError> {
    let path = directory(manifest.id)?;
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        let extension = path.extension().and_then(|value| value.to_str());
        let identity = path.file_stem().and_then(|value| value.to_str());
        let owned_root = extension == Some("json")
            && identity.is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            });
        let committed = manifest
            .provider_state
            .as_ref()
            .is_some_and(|reference| identity == Some(reference.state_id.as_str()));
        if extension == Some("stage") || (owned_root && !committed) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub(super) fn publish(state: &State) -> Result<ProviderStateRef, ProviderStateError> {
    let bytes = serde_json::to_vec(state)?;
    let state_id = compute_hash(&bytes);
    let directory = directory(state.session_id)?;
    fs::create_dir_all(&directory)?;
    let destination = directory.join(format!("{state_id}.json"));
    let staging = directory.join(format!("{}.stage", Uuid::new_v4()));
    let result = (|| -> Result<(), ProviderStateError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staging)?;
        perform("provider_state.write", || file.write_all(&bytes))?;
        perform("provider_state.sync", || file.sync_all())?;
        perform("provider_state.rename", || {
            fs::rename(&staging, &destination)
        })?;
        perform("provider_state.directory", || {
            sync_directory(&directory)?;
            let parent = directory
                .parent()
                .ok_or_else(|| std::io::Error::other("state directory parent"))?;
            sync_directory(parent)?;
            if let Some(base) = parent.parent() {
                File::open(base)?.sync_all()?;
            }
            Ok(())
        })?;
        Ok(())
    })();
    if staging.exists() {
        fs::remove_file(&staging)?;
    }
    result?;
    Ok(ProviderStateRef {
        version: 1,
        state_id,
    })
}
