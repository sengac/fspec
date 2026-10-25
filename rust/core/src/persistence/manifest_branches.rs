/// Merge messages from another session into the target session.
pub fn merge_messages(
    target: &mut SessionManifest,
    source_id: Uuid,
    indices: &[usize],
) -> Result<(), String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Session store not initialized")?;
    let source = store_ref.load(source_id)?;
    for &idx in indices {
        if idx >= source.messages.len() {
            return Err(format!(
                "Message index {} is out of range (source has {} messages)",
                idx,
                source.messages.len()
            ));
        }
    }
    let inserted_at = target.messages.len();
    for &idx in indices {
        let msg_ref = &source.messages[idx];
        target.messages.push(MessageRef {
            message_id: msg_ref.message_id,
            source: MessageSource::Imported {
                from_session: source_id,
                original_index: idx,
            },
        });
    }
    target.updated_at = Utc::now();
    target.record_merge(MergeRecord {
        source_session_id: source_id,
        source_indices: indices.to_vec(),
        inserted_at: Some(inserted_at),
        merged_at: Utc::now(),
    });
    drop(store);
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(target)?;
    Ok(())
}

/// Cherry-pick a message with N preceding context messages.
pub fn cherry_pick(
    target: &mut SessionManifest,
    source_id: Uuid,
    index: usize,
    context: usize,
) -> Result<Vec<usize>, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Session store not initialized")?;
    let source = store_ref.load(source_id)?;
    if index >= source.messages.len() {
        return Err(format!(
            "Message index {} is out of range (source has {} messages)",
            index,
            source.messages.len()
        ));
    }
    let start_index = index.saturating_sub(context);
    let indices: Vec<usize> = (start_index..=index).collect();
    let inserted_at = target.messages.len();
    for &idx in &indices {
        let msg_ref = &source.messages[idx];
        target.messages.push(MessageRef {
            message_id: msg_ref.message_id,
            source: MessageSource::Imported {
                from_session: source_id,
                original_index: idx,
            },
        });
    }
    target.updated_at = Utc::now();
    target.record_merge(MergeRecord {
        source_session_id: source_id,
        source_indices: indices.clone(),
        inserted_at: Some(inserted_at),
        merged_at: Utc::now(),
    });
    drop(store);
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(target)?;
    Ok(indices)
}

/// List all sessions for a project.
pub fn list_sessions(project: &Path) -> Result<Vec<SessionManifest>, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_for_project(project)
        .into_iter()
        .cloned()
        .collect())
}

/// Switch to a different session (returns the session).
pub fn switch_session(id: Uuid) -> Result<SessionManifest, String> {
    load_session(id)
}

/// Delete a session.
///
/// RPC-033: replaces the RPC-026 indirection. Performs two operations
/// against the CURRENTLY-configured data directory:
///
/// 1. Removes the on-disk manifest at
///    `codelet_common::get_data_dir()/sessions/{id}.json` (idempotent —
///    silently succeeds when the file is absent). This path is resolved
///    fresh on every call so cross-transport callers (codelet-rpc,
///    codelet-rpc-server) that swap the data dir mid-process get
///    consistent behaviour even if a stale `SessionStore` singleton is
///    still cached against an older directory.
/// 2. Under writer ownership, evicts the entry + last-session bookkeeping
///    from an initialised global `SESSION_STORE` cache after disk deletion
///    succeeds so the in-memory state matches the on-disk state.
pub fn delete_session(id: Uuid) -> Result<(), String> {
    super::manifest_delete::delete_current_session(id)
}

/// Rename a session.
pub fn rename_session(id: Uuid, new_name: &str) -> Result<(), String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .rename(id, new_name)
}
