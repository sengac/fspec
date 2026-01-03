//! NAPI bindings for session persistence functions
//!
//! Exposes the Rust persistence API to TypeScript/JavaScript via NAPI-RS.

use super::*;
use napi::bindgen_prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Configuration
// ============================================================================

/// Set the data directory for persistence (e.g., ~/.fspec or ~/.codelet)
///
/// This must be called before any other persistence operations if you want
/// to use a custom directory instead of the default ~/.fspec.
#[napi]
pub fn persistence_set_data_directory(dir: String) -> Result<()> {
    set_data_directory(PathBuf::from(dir)).map_err(Error::from_reason)
}

/// Get the current data directory
#[napi]
pub fn persistence_get_data_directory() -> Result<String> {
    get_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(Error::from_reason)
}

// ============================================================================
// Session Management
// ============================================================================

/// Create a new session
#[napi]
pub fn persistence_create_session(name: String, project: String) -> Result<NapiSessionManifest> {
    create_session(&name, &PathBuf::from(project))
        .map(|s| s.into())
        .map_err(Error::from_reason)
}

/// Create a new session with a specific provider
#[napi]
pub fn persistence_create_session_with_provider(
    name: String,
    project: String,
    provider: String,
) -> Result<NapiSessionManifest> {
    create_session_with_provider(&name, &PathBuf::from(project), &provider)
        .map(|s| s.into())
        .map_err(Error::from_reason)
}

/// Load a session by ID
#[napi]
pub fn persistence_load_session(id: String) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    load_session(uuid)
        .map(|s| s.into())
        .map_err(Error::from_reason)
}

/// Resume the last session for a project
#[napi]
pub fn persistence_resume_last_session(project: String) -> Result<NapiSessionManifest> {
    resume_last_session(&PathBuf::from(project))
        .map(|s| s.into())
        .map_err(Error::from_reason)
}

/// List all sessions for a project
#[napi]
pub fn persistence_list_sessions(project: String) -> Result<Vec<NapiSessionManifest>> {
    list_sessions(&PathBuf::from(project))
        .map(|sessions| sessions.into_iter().map(|s| s.into()).collect())
        .map_err(Error::from_reason)
}

/// Delete a session
#[napi]
pub fn persistence_delete_session(id: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    delete_session(uuid).map_err(Error::from_reason)
}

/// Rename a session
#[napi]
pub fn persistence_rename_session(id: String, new_name: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    rename_session(uuid, &new_name).map_err(Error::from_reason)
}

/// Fork a session at a specific message index
#[napi]
pub fn persistence_fork_session(
    session_id: String,
    at_index: u32,
    name: String,
) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let session = load_session(uuid).map_err(Error::from_reason)?;
    fork_session(&session, at_index as usize, &name)
        .map(|s| s.into())
        .map_err(Error::from_reason)
}

/// Merge messages from another session
#[napi]
pub fn persistence_merge_messages(
    target_id: String,
    source_id: String,
    indices: Vec<u32>,
) -> Result<NapiSessionManifest> {
    let target_uuid =
        uuid::Uuid::parse_str(&target_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let source_uuid =
        uuid::Uuid::parse_str(&source_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut target = load_session(target_uuid).map_err(Error::from_reason)?;
    let indices_usize: Vec<usize> = indices.into_iter().map(|i| i as usize).collect();
    merge_messages(&mut target, source_uuid, &indices_usize).map_err(Error::from_reason)?;
    Ok(target.into())
}

/// Cherry-pick messages with context
#[napi]
pub fn persistence_cherry_pick(
    target_id: String,
    source_id: String,
    index: u32,
    context: u32,
) -> Result<NapiCherryPickResult> {
    let target_uuid =
        uuid::Uuid::parse_str(&target_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let source_uuid =
        uuid::Uuid::parse_str(&source_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut target = load_session(target_uuid).map_err(Error::from_reason)?;
    let imported = cherry_pick(&mut target, source_uuid, index as usize, context as usize)
        .map_err(Error::from_reason)?;
    Ok(NapiCherryPickResult {
        session: target.into(),
        imported_indices: imported.into_iter().map(|i| i as u32).collect(),
    })
}

// ============================================================================
// Message Operations
// ============================================================================

/// Append a message to a session
#[napi]
pub fn persistence_append_message(
    session_id: String,
    role: String,
    content: String,
) -> Result<NapiAppendResult> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;
    let msg_id = append_message(&mut session, &role, &content).map_err(Error::from_reason)?;
    Ok(NapiAppendResult {
        message_id: msg_id.to_string(),
        session: session.into(),
    })
}

/// Append a message with metadata to a session
///
/// metadata_json should be a JSON object string, e.g. '{"model": "claude-3", "stop_reason": "end_turn"}'
#[napi]
pub fn persistence_append_message_with_metadata(
    session_id: String,
    role: String,
    content: String,
    metadata_json: String,
) -> Result<NapiAppendResult> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;

    let metadata_value: serde_json::Value = serde_json::from_str(&metadata_json)
        .map_err(|e| Error::from_reason(format!("Invalid metadata JSON: {}", e)))?;

    let metadata_map: HashMap<String, serde_json::Value> =
        if let Some(obj) = metadata_value.as_object() {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            HashMap::new()
        };

    let msg_id = append_message_with_metadata(&mut session, &role, &content, metadata_map)
        .map_err(Error::from_reason)?;

    Ok(NapiAppendResult {
        message_id: msg_id.to_string(),
        session: session.into(),
    })
}

/// Get a message by ID
#[napi]
pub fn persistence_get_message(id: String) -> Result<Option<NapiStoredMessage>> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    get_message(uuid)
        .map(|opt| opt.map(|m| m.into()))
        .map_err(Error::from_reason)
}

/// Get all messages for a session
#[napi]
pub fn persistence_get_session_messages(session_id: String) -> Result<Vec<NapiStoredMessage>> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let session = load_session(uuid).map_err(Error::from_reason)?;
    get_session_messages(&session)
        .map(|msgs| msgs.into_iter().map(|m| m.into()).collect())
        .map_err(Error::from_reason)
}

// ============================================================================
// Blob Storage
// ============================================================================

/// Store content in blob storage
#[napi]
pub fn persistence_store_blob(content: Buffer) -> Result<String> {
    store_blob(&content).map_err(Error::from_reason)
}

/// Get content from blob storage
#[napi]
pub fn persistence_get_blob(hash: String) -> Result<Buffer> {
    get_blob(&hash)
        .map(Buffer::from)
        .map_err(Error::from_reason)
}

/// Check if a blob exists
#[napi]
pub fn persistence_blob_exists(hash: String) -> Result<bool> {
    blob_exists(&hash).map_err(Error::from_reason)
}

// ============================================================================
// History
// ============================================================================

/// Add a history entry
#[napi]
pub fn persistence_add_history(display: String, project: String, session_id: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let entry = HistoryEntry::new(display, PathBuf::from(project), uuid);
    add_history_entry(entry).map_err(Error::from_reason)
}

/// Get history entries
#[napi]
pub fn persistence_get_history(
    project: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<NapiHistoryEntry>> {
    let project_path = project.map(PathBuf::from);
    get_history(project_path.as_deref(), limit.map(|l| l as usize))
        .map(|entries| entries.into_iter().map(|e| e.into()).collect())
        .map_err(Error::from_reason)
}

/// Search history entries
#[napi]
pub fn persistence_search_history(
    query: String,
    project: Option<String>,
) -> Result<Vec<NapiHistoryEntry>> {
    let project_path = project.map(PathBuf::from);
    search_history(&query, project_path.as_deref())
        .map(|entries| entries.into_iter().map(|e| e.into()).collect())
        .map_err(Error::from_reason)
}

// ============================================================================
// Token Usage
// ============================================================================

/// Update session token usage (ADDS to existing)
#[napi]
pub fn persistence_update_session_tokens(
    session_id: String,
    input: u32,
    output: u32,
    cache_read: u32,
    cache_create: u32,
) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;
    update_session_tokens(
        &mut session,
        input as u64,
        output as u64,
        cache_read as u64,
        cache_create as u64,
    )
    .map_err(Error::from_reason)?;
    Ok(session.into())
}

/// Set session token usage (REPLACES existing - use for cumulative totals)
#[napi]
pub fn persistence_set_session_tokens(
    session_id: String,
    input: u32,
    output: u32,
    cache_read: u32,
    cache_create: u32,
    cumulative_input: u32,
    cumulative_output: u32,
) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;
    set_session_tokens(
        &mut session,
        input as u64,
        output as u64,
        cache_read as u64,
        cache_create as u64,
        cumulative_input as u64,
        cumulative_output as u64,
    )
    .map_err(Error::from_reason)?;
    Ok(session.into())
}

// ============================================================================
// Compaction State
// ============================================================================

/// Set compaction state for a session (after manual or automatic compaction)
#[napi]
pub fn persistence_set_compaction_state(
    session_id: String,
    summary: String,
    compacted_before_index: u32,
) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;
    set_compaction_state(&mut session, summary, compacted_before_index as usize)
        .map_err(Error::from_reason)?;
    Ok(session.into())
}

/// Clear compaction state for a session
#[napi]
pub fn persistence_clear_compaction_state(session_id: String) -> Result<NapiSessionManifest> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;
    clear_compaction_state(&mut session).map_err(Error::from_reason)?;
    Ok(session.into())
}

// ============================================================================
// Cleanup
// ============================================================================

/// Cleanup orphaned messages
#[napi]
pub fn persistence_cleanup_orphaned_messages() -> Result<u32> {
    cleanup_orphaned_messages()
        .map(|count| count as u32)
        .map_err(Error::from_reason)
}

// ============================================================================
// NAPI Types
// ============================================================================

#[napi(object)]
pub struct NapiSessionManifest {
    pub id: String,
    pub name: String,
    pub project: String,
    pub provider: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u32,
    pub forked_from: Option<NapiForkPoint>,
    pub merged_from: Vec<NapiMergeRecord>,
    pub compaction: Option<NapiCompactionState>,
    pub token_usage: NapiTokenUsage,
}

impl From<SessionManifest> for NapiSessionManifest {
    fn from(s: SessionManifest) -> Self {
        Self {
            id: s.id.to_string(),
            name: s.name,
            project: s.project.to_string_lossy().to_string(),
            provider: s.provider,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
            message_count: s.messages.len() as u32,
            forked_from: s.forked_from.map(|f| f.into()),
            merged_from: s.merged_from.into_iter().map(|m| m.into()).collect(),
            compaction: s.compaction.map(|c| c.into()),
            token_usage: s.token_usage.into(),
        }
    }
}

#[napi(object)]
pub struct NapiForkPoint {
    pub source_session_id: String,
    pub fork_after_index: u32,
    pub forked_at: String,
}

impl From<ForkPoint> for NapiForkPoint {
    fn from(f: ForkPoint) -> Self {
        Self {
            source_session_id: f.source_session_id.to_string(),
            fork_after_index: f.fork_after_index as u32,
            forked_at: f.forked_at.to_rfc3339(),
        }
    }
}

#[napi(object)]
pub struct NapiMergeRecord {
    pub source_session_id: String,
    pub source_indices: Vec<u32>,
    pub inserted_at: Option<u32>,
    pub merged_at: String,
}

impl From<MergeRecord> for NapiMergeRecord {
    fn from(m: MergeRecord) -> Self {
        Self {
            source_session_id: m.source_session_id.to_string(),
            source_indices: m.source_indices.into_iter().map(|i| i as u32).collect(),
            inserted_at: m.inserted_at.map(|i| i as u32),
            merged_at: m.merged_at.to_rfc3339(),
        }
    }
}

#[napi(object)]
pub struct NapiCompactionState {
    pub summary: String,
    pub compacted_before_index: u32,
    pub compacted_at: String,
}

impl From<CompactionState> for NapiCompactionState {
    fn from(c: CompactionState) -> Self {
        Self {
            summary: c.summary,
            compacted_before_index: c.compacted_before_index as u32,
            compacted_at: c.compacted_at.to_rfc3339(),
        }
    }
}

/// Token usage with dual metrics (CTX-003)
///
/// - `current_context_tokens`: Latest context size (for display and threshold checks)
/// - `cumulative_billed_input`: Sum of all API calls (for billing analytics)
/// - `cumulative_billed_output`: Sum of all API output tokens (for billing analytics)
#[napi(object)]
pub struct NapiTokenUsage {
    /// Current context size (latest input_tokens from API - overwritten, not accumulated)
    /// CTX-003: This is what should be displayed to users and used for threshold checks
    pub current_context_tokens: u32,
    /// Cumulative billed input tokens (sum of all API calls - for billing analytics)
    /// CTX-003: This is the total billed by Anthropic across all API calls
    pub cumulative_billed_input: u32,
    /// Cumulative billed output tokens (sum of all API calls)
    pub cumulative_billed_output: u32,
    /// Cache read tokens from current API call
    pub cache_read_tokens: u32,
    /// Cache creation tokens from current API call
    pub cache_creation_tokens: u32,
}

impl From<TokenUsage> for NapiTokenUsage {
    fn from(t: TokenUsage) -> Self {
        Self {
            current_context_tokens: t.current_context_tokens as u32,
            cumulative_billed_input: t.cumulative_billed_input as u32,
            cumulative_billed_output: t.cumulative_billed_output as u32,
            cache_read_tokens: t.cache_read_tokens as u32,
            cache_creation_tokens: t.cache_creation_tokens as u32,
        }
    }
}

#[napi(object)]
pub struct NapiStoredMessage {
    pub id: String,
    pub content_hash: String,
    pub created_at: String,
    pub role: String,
    pub content: String,
    pub token_count: Option<u32>,
    pub blob_refs: Vec<String>,
    /// Metadata as a JSON string
    pub metadata_json: String,
}

impl From<StoredMessage> for NapiStoredMessage {
    fn from(m: StoredMessage) -> Self {
        let metadata_value = serde_json::Value::Object(
            m.metadata
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        );
        Self {
            id: m.id.to_string(),
            content_hash: m.content_hash,
            created_at: m.created_at.to_rfc3339(),
            role: m.role,
            content: m.content,
            token_count: m.token_count,
            blob_refs: m.blob_refs,
            metadata_json: serde_json::to_string(&metadata_value).unwrap_or_default(),
        }
    }
}

#[napi(object)]
pub struct NapiHistoryEntry {
    pub display: String,
    pub timestamp: String,
    pub project: String,
    pub session_id: String,
    pub has_pasted_content: bool,
}

impl From<HistoryEntry> for NapiHistoryEntry {
    fn from(e: HistoryEntry) -> Self {
        Self {
            display: e.display,
            timestamp: e.timestamp.to_rfc3339(),
            project: e.project.to_string_lossy().to_string(),
            session_id: e.session_id.to_string(),
            has_pasted_content: e.pasted_content.is_some(),
        }
    }
}

#[napi(object)]
pub struct NapiAppendResult {
    pub message_id: String,
    pub session: NapiSessionManifest,
}

#[napi(object)]
pub struct NapiCherryPickResult {
    pub session: NapiSessionManifest,
    pub imported_indices: Vec<u32>,
}

// ============================================================================
// Message Envelope Types (NAPI-008: Claude Code Format)
// ============================================================================

// Re-export blob processing functions from the pure Rust module for use in this module
use super::blob_processing::{
    process_envelope_for_blob_storage as process_envelope_impl,
    rehydrate_envelope_blobs as rehydrate_envelope_impl,
};

/// Store a message envelope as JSON
///
/// This is the primary function for storing Claude Code format messages.
/// It handles blob storage for large content automatically.
#[napi]
pub fn persistence_store_message_envelope(
    session_id: String,
    envelope_json: String,
) -> Result<NapiAppendResult> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let mut session = load_session(uuid).map_err(Error::from_reason)?;

    // Parse and validate the envelope
    let envelope: super::MessageEnvelope = serde_json::from_str(&envelope_json)
        .map_err(|e| Error::from_reason(format!("Invalid message envelope JSON: {}", e)))?;

    // CRITICAL FIX: Calculate actual token count BEFORE blob storage processing.
    // This ensures we track the real content size, not the blob reference size.
    // Without this, tool results like file reads get counted as ~42 tokens (blob ref)
    // instead of their actual ~6000+ tokens, causing context tracking to be wildly
    // underestimated and leading to "Payload Too Large" API errors.
    let actual_token_count = calculate_envelope_tokens(&envelope);

    // Process envelope for blob storage (extracts large content)
    let (processed_envelope, blob_refs) =
        process_envelope_impl(&envelope).map_err(Error::from_reason)?;

    // Determine role from envelope
    let role = processed_envelope.message_type.clone();

    // Store the message with the full envelope as metadata
    let mut metadata_map: HashMap<String, serde_json::Value> =
        serde_json::from_str(&serde_json::to_string(&processed_envelope).unwrap())
            .map_err(|e| Error::from_reason(format!("Failed to serialize envelope: {}", e)))?;

    // Add the actual token count (calculated from original content before blob processing)
    // This will be used by store_with_metadata to set the correct token_count
    metadata_map.insert(
        "_actualTokenCount".to_string(),
        serde_json::json!(actual_token_count),
    );

    // Add blob references to metadata if any content was stored in blobs
    if !blob_refs.is_empty() {
        let blob_refs_json: serde_json::Value = blob_refs
            .into_iter()
            .map(|(k, v)| serde_json::json!({"key": k, "hash": v}))
            .collect();
        metadata_map.insert("_blobRefs".to_string(), blob_refs_json);
    }

    // Extract a text summary for the content field
    let content_summary = extract_content_summary(&processed_envelope);

    let msg_id = append_message_with_metadata(&mut session, &role, &content_summary, metadata_map)
        .map_err(Error::from_reason)?;

    Ok(NapiAppendResult {
        message_id: msg_id.to_string(),
        session: session.into(),
    })
}

/// Get a message as a full envelope JSON with blob content rehydrated
#[napi]
pub fn persistence_get_message_envelope(id: String) -> Result<Option<String>> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    let msg = get_message(uuid).map_err(Error::from_reason)?;

    match msg {
        Some(stored_msg) => {
            // Reconstruct envelope from stored metadata
            let metadata_value = serde_json::Value::Object(
                stored_msg
                    .metadata
                    .into_iter()
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            );
            let envelope_json = serde_json::to_string(&metadata_value).unwrap_or_default();

            // Rehydrate blob references to restore original content
            let rehydrated = rehydrate_envelope_impl(&envelope_json).map_err(Error::from_reason)?;
            Ok(Some(rehydrated))
        }
        None => Ok(None),
    }
}

/// Get a message envelope WITHOUT blob rehydration (returns blob references as-is)
/// Use this when you want to inspect the raw stored format with blob:sha256: references.
#[napi]
pub fn persistence_get_message_envelope_raw(id: String) -> Result<Option<String>> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    let msg = get_message(uuid).map_err(Error::from_reason)?;

    match msg {
        Some(stored_msg) => {
            // Reconstruct envelope from stored metadata (no rehydration)
            let metadata_value = serde_json::Value::Object(
                stored_msg
                    .metadata
                    .into_iter()
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            );
            Ok(Some(
                serde_json::to_string(&metadata_value).unwrap_or_default(),
            ))
        }
        None => Ok(None),
    }
}

/// Get all messages for a session as envelope JSON array with blob content rehydrated
#[napi]
pub fn persistence_get_session_message_envelopes(session_id: String) -> Result<Vec<String>> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let session = load_session(uuid).map_err(Error::from_reason)?;
    let messages = get_session_messages(&session).map_err(Error::from_reason)?;

    let mut envelopes: Vec<String> = Vec::with_capacity(messages.len());
    for stored_msg in messages {
        let metadata_value = serde_json::Value::Object(
            stored_msg
                .metadata
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        );
        let envelope_json = serde_json::to_string(&metadata_value).unwrap_or_default();

        // Rehydrate blob references to restore original content
        let rehydrated = rehydrate_envelope_impl(&envelope_json).map_err(Error::from_reason)?;
        envelopes.push(rehydrated);
    }

    Ok(envelopes)
}

/// Get all messages for a session WITHOUT blob rehydration (returns blob references as-is)
#[napi]
pub fn persistence_get_session_message_envelopes_raw(session_id: String) -> Result<Vec<String>> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| Error::from_reason(e.to_string()))?;
    let session = load_session(uuid).map_err(Error::from_reason)?;
    let messages = get_session_messages(&session).map_err(Error::from_reason)?;

    let envelopes: Vec<String> = messages
        .into_iter()
        .map(|stored_msg| {
            let metadata_value = serde_json::Value::Object(
                stored_msg
                    .metadata
                    .into_iter()
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            );
            serde_json::to_string(&metadata_value).unwrap_or_default()
        })
        .collect();

    Ok(envelopes)
}

/// Truncate a string to a maximum number of characters (UTF-8 safe)
fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// Calculate the actual token count from the ORIGINAL envelope content before blob processing.
/// This is critical for accurate context tracking - blob references are much shorter than actual content.
fn calculate_envelope_tokens(envelope: &super::MessageEnvelope) -> u32 {
    use codelet_common::token_estimator::count_tokens;

    let mut total_tokens: usize = 0;

    match &envelope.message {
        super::MessagePayload::User(user_msg) => {
            for content in &user_msg.content {
                match content {
                    super::UserContent::Text { text } => {
                        total_tokens += count_tokens(text);
                    }
                    super::UserContent::ToolResult { content, .. } => {
                        // This is the critical fix - count tokens from actual tool result content
                        total_tokens += count_tokens(content);
                    }
                    super::UserContent::Image { .. } => {
                        // Images are sent as base64, estimate ~85 tokens for image content block
                        // (actual tokens depend on image size, but base64 is large)
                        total_tokens += 85;
                    }
                    super::UserContent::Document { .. } => {
                        // Documents similarly estimated
                        total_tokens += 100;
                    }
                }
            }
        }
        super::MessagePayload::Assistant(assistant_msg) => {
            for content in &assistant_msg.content {
                match content {
                    super::AssistantContent::Text { text } => {
                        total_tokens += count_tokens(text);
                    }
                    super::AssistantContent::ToolUse { input, .. } => {
                        // Count tokens in tool input JSON
                        let input_str = serde_json::to_string(input).unwrap_or_default();
                        total_tokens += count_tokens(&input_str);
                    }
                    super::AssistantContent::Thinking { thinking, .. } => {
                        total_tokens += count_tokens(thinking);
                    }
                }
            }
        }
    }

    total_tokens as u32
}

/// Helper function to extract a text summary from a message envelope
fn extract_content_summary(envelope: &super::MessageEnvelope) -> String {
    match &envelope.message {
        super::MessagePayload::User(user_msg) => {
            if let Some(content) = user_msg.content.first() {
                match content {
                    super::UserContent::Text { text } => return text.clone(),
                    super::UserContent::ToolResult { content, .. } => {
                        return truncate_chars(content, 200);
                    }
                    super::UserContent::Image { .. } => return "[image]".to_string(),
                    super::UserContent::Document { .. } => return "[document]".to_string(),
                }
            }
            "[empty user message]".to_string()
        }
        super::MessagePayload::Assistant(assistant_msg) => {
            if let Some(content) = assistant_msg.content.first() {
                match content {
                    super::AssistantContent::Text { text } => return text.clone(),
                    super::AssistantContent::ToolUse { name, .. } => {
                        return format!("[tool_use: {name}]");
                    }
                    super::AssistantContent::Thinking { thinking, .. } => {
                        let summary = truncate_chars(thinking, 200);
                        if summary.ends_with("...") {
                            return format!("[thinking: {summary}]");
                        }
                        return format!("[thinking: {thinking}]");
                    }
                }
            }
            "[empty assistant message]".to_string()
        }
    }
}
