/// Append a message to a session.
pub fn append_message(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
) -> Result<Uuid, String> {
    super::manifest_updates::append(session, role, content, HashMap::new())
}

/// Append a message with metadata to a session.
pub fn append_message_with_metadata(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
    metadata: HashMap<String, serde_json::Value>,
) -> Result<Uuid, String> {
    super::manifest_updates::append(session, role, content, metadata)
}

/// Update metadata on a previously stored message.
pub fn update_message_metadata(
    id: Uuid,
    entries: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    init_message_store()?;
    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    msg_store
        .as_mut()
        .ok_or("Message store not initialized")?
        .update_metadata(id, entries)
}

#[path = "message_cleanup.rs"]
mod message_cleanup;
pub use message_cleanup::cleanup_orphaned_messages;

/// Get a stored message by ID.
pub fn get_message(id: Uuid) -> Result<Option<StoredMessage>, String> {
    init_message_store()?;
    let store = codelet_common::read_work::lock(&MESSAGE_STORE)?;
    let message = store
        .as_ref()
        .ok_or("Message store not initialized")?
        .get(id);
    codelet_common::read_work::checkpoint("message.loaded").map_err(|error| error.to_string())?;
    Ok(message)
}

/// Get all messages for a session, respecting compaction state.
///
/// If the session has been compacted, this returns a synthetic summary
/// message followed by the post-boundary messages. Otherwise it loads
/// every referenced message.
pub fn get_session_messages(session: &SessionManifest) -> Result<Vec<StoredMessage>, String> {
    init_message_store()?;
    let store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Message store not initialized")?;
    let mut messages = Vec::new();
    if let Some(ref compaction) = session.compaction {
        let summary_content = format!("[Previous conversation summary]\n\n{}", compaction.summary);
        let summary_tokens = count_tokens(&summary_content) as u32;
        let mut meta = HashMap::new();
        meta.insert("_synthetic".to_string(), serde_json::json!(true));
        meta.insert("_compactionSummary".to_string(), serde_json::json!(true));
        meta.insert(
            "_compactedBeforeIndex".to_string(),
            serde_json::json!(compaction.compacted_before_index),
        );
        messages.push(StoredMessage {
            id: Uuid::nil(),
            content_hash: String::new(),
            created_at: compaction.compacted_at,
            role: "user".to_string(),
            content: summary_content,
            token_count: Some(summary_tokens),
            blob_refs: Vec::new(),
            metadata: meta,
        });
        for msg_ref in session
            .messages
            .iter()
            .skip(compaction.compacted_before_index)
        {
            messages.push(store_ref.get(msg_ref.message_id).ok_or_else(|| {
                format!("missing committed message reference {}", msg_ref.message_id)
            })?);
        }
    } else {
        for msg_ref in &session.messages {
            messages.push(store_ref.get(msg_ref.message_id).ok_or_else(|| {
                format!("missing committed message reference {}", msg_ref.message_id)
            })?);
        }
    }
    Ok(messages)
}

/// Get ALL messages for a session, ignoring compaction state.
pub fn get_session_messages_full(session: &SessionManifest) -> Result<Vec<StoredMessage>, String> {
    let mut messages = Vec::new();
    for (position, msg_ref) in session.messages.iter().enumerate() {
        if position % 32 == 0 {
            codelet_common::read_work::checkpoint("history.batch")
                .map_err(|error| error.to_string())?;
        }
        if let Some(msg) = get_message(msg_ref.message_id)? {
            messages.push(msg);
        }
        codelet_common::read_work::checkpoint("history.message")
            .map_err(|error| error.to_string())?;
    }
    Ok(messages)
}

/// RPC-049: get all messages for a session as JSON envelope strings,
/// with blob references rehydrated and compaction-summary entries
/// rendered as synthetic envelopes. Mirrors the body of
/// `rust/napi/src/persistence/napi_bindings.rs::persistence_get_session_message_envelopes`
/// minus the napi::Error wrapping — used by `SessionManagerHandle::resume_session`
/// to feed `restore_session_messages`.
pub fn get_session_message_envelopes(session_id: Uuid) -> Result<Vec<String>, String> {
    super::session_replay::get_session_message_envelopes(session_id)
}
