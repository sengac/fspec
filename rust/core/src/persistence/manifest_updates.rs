use super::manifest::*;
use super::manifest_commit::ManifestCommitError;
use chrono::Utc;
use std::cell::Cell;
use uuid::Uuid;

thread_local! {
    static ACTIVE_WRITER: Cell<bool> = const { Cell::new(false) };
}

pub(super) struct SessionWriterGuard;

pub(super) fn ensure_session_access() -> Result<(), String> {
    if ACTIVE_WRITER.get() {
        return Err(
            "session ownership reentry is unavailable during a manifest writer transaction".into(),
        );
    }
    Ok(())
}

pub(super) fn ensure_writer_access(session_id: Uuid) -> Result<(), ManifestCommitError> {
    ensure_session_access().map_err(|reason| ManifestCommitError::Ownership { session_id, reason })
}

impl SessionWriterGuard {
    pub(super) fn enter(session_id: Uuid) -> Result<Self, ManifestCommitError> {
        ensure_writer_access(session_id)?;
        ACTIVE_WRITER.set(true);
        Ok(Self)
    }
}

impl Drop for SessionWriterGuard {
    fn drop(&mut self) {
        ACTIVE_WRITER.set(false);
    }
}

pub fn update_session_manifest(
    session_id: Uuid,
    update: impl FnOnce(&mut SessionManifest) -> Result<(), ManifestCommitError>,
) -> Result<SessionManifest, ManifestCommitError> {
    ensure_writer_access(session_id)?;
    init_session_store().map_err(ManifestCommitError::Store)?;
    SESSION_STORE
        .lock()
        .map_err(|error| ManifestCommitError::Store(error.to_string()))?
        .as_mut()
        .ok_or_else(|| ManifestCommitError::Store("not initialized".into()))?
        .update(session_id, update)
}

pub(super) fn append(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
    metadata: std::collections::HashMap<String, serde_json::Value>,
) -> Result<Uuid, String> {
    init_message_store()?;
    let mut message_id = Uuid::nil();
    let committed = update_session_manifest(session.id, |current| {
        let mut store = MESSAGE_STORE
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        message_id = store
            .as_mut()
            .ok_or_else(|| ManifestCommitError::Store("message store not initialized".into()))?
            .store_with_metadata(role, content, metadata)
            .map_err(ManifestCommitError::Store)?;
        current.add_message(message_id, super::messages::MessageSource::Native);
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    *session = committed;
    Ok(message_id)
}

pub(super) fn update_tokens(
    session: &mut SessionManifest,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
    reasoning_tokens: u64,
) -> Result<(), String> {
    *session = update_session_manifest(session.id, |current| {
        current.update_token_usage(input, output, cache_read, cache_create);
        current.token_usage.reasoning_tokens = reasoning_tokens;
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn set_tokens(session: &mut SessionManifest, usage: &TokenUsage) -> Result<(), String> {
    *session = update_session_manifest(session.id, |current| {
        current.token_usage = usage.clone();
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn set_compaction(
    session: &mut SessionManifest,
    summary: String,
    compacted_before_index: usize,
) -> Result<(), String> {
    let expected_revision = session.revision;
    *session = update_session_manifest(session.id, |current| {
        if current.revision != expected_revision {
            return Err(ManifestCommitError::Conflict {
                session_id: current.id,
                expected: expected_revision,
                actual: current.revision,
            });
        }
        current.compaction = Some(CompactionState {
            summary,
            compacted_before_index,
            compacted_at: Utc::now(),
        });
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn clear_compaction(session: &mut SessionManifest) -> Result<(), String> {
    *session = update_session_manifest(session.id, |current| {
        current.compaction = None;
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}
