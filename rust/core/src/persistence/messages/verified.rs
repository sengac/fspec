//! Optional completeness proof for a strictly scanned derived index.
//!
//! Both file generations must match. Index edits, data edits, cleanup and legacy
//! writes invalidate the proof; a missing/corrupt proof uses normal recovery.
//! This is a cache of validation, never authority for message contents or IDs.
use super::durability::{perform, sync_parent};
use super::{MessageStore, StoreGeneration};
use std::io::Write;

#[derive(serde::Serialize, serde::Deserialize)]
struct Certificate {
    version: u8,
    generation: StoreGeneration,
}

pub(super) fn matches(store: &MessageStore, generation: &StoreGeneration) -> bool {
    let path = store.index_file.with_extension("verified");
    let Ok(metadata) = std::fs::metadata(&path) else {
        return false;
    };
    if metadata.len() > 16 * 1024 {
        return false;
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Certificate>(&bytes).ok())
        .is_some_and(|proof| proof.version == 1 && proof.generation == *generation)
}

pub(super) fn publish(store: &MessageStore, generation: &StoreGeneration) {
    // Failure is conservative: the previous proof cannot match the new index
    // generation. A later caller recovers normally; message admission is intact.
    if let Err(error) = save(store, generation) {
        tracing::warn!("Unable to save message-index completeness cache: {error}");
    }
}

fn save(store: &MessageStore, generation: &StoreGeneration) -> Result<(), String> {
    let path = store.index_file.with_extension("verified");
    let bytes = serde_json::to_vec(&Certificate {
        version: 1,
        generation: generation.clone(),
    })
    .map_err(|error| error.to_string())?;
    let mut file = tempfile::Builder::new()
        .prefix("messages.verified.")
        .suffix(".tmp")
        .tempfile_in(&store.messages_dir)
        .map_err(|error| error.to_string())?;
    perform("verification.write", || file.write_all(&bytes)).map_err(|error| error.to_string())?;
    perform("verification.sync", || file.as_file().sync_all())
        .map_err(|error| error.to_string())?;
    perform("verification.rename", || {
        file.persist(&path).map(|_| ()).map_err(|error| error.error)
    })
    .map_err(|error| error.to_string())?;
    perform("verification.directory", || sync_parent(&path)).map_err(|error| error.to_string())
}
