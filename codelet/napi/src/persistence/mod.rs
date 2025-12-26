//! Session Persistence Module
//!
//! Implements session management with git-like fork/merge operations.
//! All persistence operations are in Rust, exposed to TypeScript via NAPI-RS bindings.
//!
//! File Layout (default ~/.fspec, configurable via set_data_directory):
//! - {data_dir}/messages/    - Content-addressed message store (JSONL)
//! - {data_dir}/sessions/    - Session manifests (JSON)
//! - {data_dir}/blobs/       - Large content blob storage (SHA-256 addressed)
//! - {data_dir}/history.jsonl - Command history (append-only JSONL)

mod blob;
mod blob_processing;
mod history;
mod message_envelope;
mod storage;
mod types;

// NAPI bindings are only available in production mode (not with noop feature)
#[cfg(not(feature = "noop"))]
mod napi_bindings;

#[cfg(test)]
mod tests;

pub use blob::*;
pub use blob_processing::*;
pub use history::*;
pub use message_envelope::*;
pub use storage::*;
pub use types::*;

#[cfg(not(feature = "noop"))]
pub use napi_bindings::*;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

// Global singleton stores (thread-safe)
lazy_static::lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
    static ref HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);
    static ref DATA_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
}

/// Set a custom data directory for persistence
///
/// This should be called before any other persistence operations if you want
/// to use a directory other than the default ~/.fspec
///
/// # Arguments
/// * `dir` - The base directory for persistence data (e.g., ~/.fspec or ~/.codelet)
///
/// # Example
/// ```ignore
/// // Use ~/.fspec for fspec
/// set_data_directory(PathBuf::from(home_dir).join(".fspec"));
///
/// // Use ~/.codelet for codelet REPL
/// set_data_directory(PathBuf::from(home_dir).join(".codelet"));
/// ```
pub fn set_data_directory(dir: PathBuf) -> Result<(), String> {
    let mut data_dir = DATA_DIRECTORY.lock().map_err(|e| e.to_string())?;
    *data_dir = Some(dir);

    // Reset stores so they reinitialize with the new directory
    let mut msg = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    *msg = None;
    drop(msg);

    let mut sess = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    *sess = None;
    drop(sess);

    let mut blob = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    *blob = None;
    drop(blob);

    let mut hist = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    *hist = None;

    Ok(())
}

/// Get the base directory for persistence data
///
/// Returns the custom directory if set via set_data_directory(),
/// otherwise returns ~/.fspec as the default.
pub fn get_data_dir() -> Result<PathBuf, String> {
    // Check for custom directory first
    if let Ok(guard) = DATA_DIRECTORY.lock() {
        if let Some(ref dir) = *guard {
            return Ok(dir.clone());
        }
    }

    // Default to ~/.fspec
    dirs::home_dir()
        .map(|home| home.join(".fspec"))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

/// Ensure all required directories exist
pub fn ensure_directories() -> Result<(), String> {
    let base = get_data_dir()?;

    let dirs = [
        base.join("messages"),
        base.join("sessions"),
        base.join("blobs"),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create directory {:?}: {}", dir, e))?;
    }

    Ok(())
}

/// Initialize all stores (call once at startup)
fn init_stores() -> Result<(), String> {
    let mut msg = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    if msg.is_none() {
        *msg = Some(MessageStore::new()?);
    }

    let mut sess = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    if sess.is_none() {
        *sess = Some(SessionStore::new()?);
    }

    let mut blob = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    if blob.is_none() {
        *blob = Some(BlobStore::new()?);
    }

    let mut hist = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    if hist.is_none() {
        *hist = Some(HistoryStore::new()?);
    }

    Ok(())
}

// ============================================================================
// High-level API functions (used by tests and NAPI bindings)
// ============================================================================

/// Create a new session
pub fn create_session(name: &str, project: &Path) -> Result<SessionManifest, String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create(name, project)
}

/// Create a new session with provider
pub fn create_session_with_provider(
    name: &str,
    project: &Path,
    provider: &str,
) -> Result<SessionManifest, String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create_with_provider(name, project, provider)
}

/// Load a session by ID
pub fn load_session(id: Uuid) -> Result<SessionManifest, String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Session store not initialized")?
        .load(id)
}

/// Resume the last session for a project
pub fn resume_last_session(project: &Path) -> Result<SessionManifest, String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Session store not initialized")?
        .resume_last(project)
}

/// Fork a session at a specific message index
pub fn fork_session(
    session: &SessionManifest,
    at_index: usize,
    name: &str,
) -> Result<SessionManifest, String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .fork(session, at_index, name)
}

/// Merge messages from another session into the target session
pub fn merge_messages(
    target: &mut SessionManifest,
    source_id: Uuid,
    indices: &[usize],
) -> Result<(), String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Session store not initialized")?;

    // Get source session
    let source = store_ref.load(source_id)?;

    // Validate indices
    for &idx in indices {
        if idx >= source.messages.len() {
            return Err(format!(
                "Message index {} is out of range (source has {} messages)",
                idx,
                source.messages.len()
            ));
        }
    }

    // Record the insertion point
    let inserted_at = target.messages.len();

    // Import message references
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
    target.updated_at = chrono::Utc::now();

    // Record the merge operation
    target.record_merge(MergeRecord {
        source_session_id: source_id,
        source_indices: indices.to_vec(),
        inserted_at: Some(inserted_at),
        merged_at: chrono::Utc::now(),
    });

    // Save the updated target session
    drop(store);
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(target)?;

    Ok(())
}

/// Cherry-pick a message with N preceding context messages
pub fn cherry_pick(
    target: &mut SessionManifest,
    source_id: Uuid,
    index: usize,
    context: usize,
) -> Result<Vec<usize>, String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Session store not initialized")?;

    // Get source session
    let source = store_ref.load(source_id)?;

    // Validate index
    if index >= source.messages.len() {
        return Err(format!(
            "Message index {} is out of range (source has {} messages)",
            index,
            source.messages.len()
        ));
    }

    // Calculate actual context (may be less than requested)
    let start_index = index.saturating_sub(context);
    let indices: Vec<usize> = (start_index..=index).collect();

    // Record the insertion point
    let inserted_at = target.messages.len();

    // Import message references
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
    target.updated_at = chrono::Utc::now();

    // Record the merge operation (cherry-pick is a special case of merge)
    target.record_merge(MergeRecord {
        source_session_id: source_id,
        source_indices: indices.clone(),
        inserted_at: Some(inserted_at),
        merged_at: chrono::Utc::now(),
    });

    // Save the updated target session
    drop(store);
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(target)?;

    Ok(indices)
}

/// List all sessions for a project
pub fn list_sessions(project: &Path) -> Result<Vec<SessionManifest>, String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_for_project(project)
        .into_iter()
        .cloned()
        .collect())
}

/// Switch to a different session (returns the session)
pub fn switch_session(id: Uuid) -> Result<SessionManifest, String> {
    load_session(id)
}

/// Delete a session
pub fn delete_session(id: Uuid) -> Result<(), String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .delete(id)
}

/// Rename a session
pub fn rename_session(id: Uuid, new_name: &str) -> Result<(), String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .rename(id, new_name)
}

/// Append a message to a session
pub fn append_message(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
) -> Result<Uuid, String> {
    init_stores()?;

    // Store the message
    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_id = msg_store
        .as_mut()
        .ok_or("Message store not initialized")?
        .store(role, content)?;

    // Add reference to session
    session.add_message(msg_id, MessageSource::Native);

    // Save the session
    drop(msg_store);
    let mut sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    sess_store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(msg_id)
}

/// Append a message with metadata to a session
pub fn append_message_with_metadata(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
    metadata: std::collections::HashMap<String, serde_json::Value>,
) -> Result<Uuid, String> {
    init_stores()?;

    // Store the message with metadata
    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_id = msg_store
        .as_mut()
        .ok_or("Message store not initialized")?
        .store_with_metadata(role, content, metadata)?;

    // Add reference to session
    session.add_message(msg_id, MessageSource::Native);

    // Save the session
    drop(msg_store);
    let mut sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    sess_store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(msg_id)
}

/// Store content in blob storage
pub fn store_blob(content: &[u8]) -> Result<String, String> {
    init_stores()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .store(content)
}

/// Get content from blob storage
pub fn get_blob(hash: &str) -> Result<Vec<u8>, String> {
    init_stores()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .get(hash)
}

/// Check if a blob exists
pub fn blob_exists(hash: &str) -> Result<bool, String> {
    init_stores()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .exists(hash))
}

/// Cleanup orphaned messages (not referenced by any session)
pub fn cleanup_orphaned_messages() -> Result<usize, String> {
    init_stores()?;

    // Get all sessions
    let sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let sessions: Vec<SessionManifest> = sess_store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_all()
        .into_iter()
        .cloned()
        .collect();
    drop(sess_store);

    // Get referenced message IDs
    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_store_ref = msg_store.as_mut().ok_or("Message store not initialized")?;
    let referenced = msg_store_ref.get_referenced_ids(&sessions);

    // Cleanup orphans
    msg_store_ref.cleanup_orphans(&referenced)
}

/// Add a history entry
pub fn add_history_entry(entry: HistoryEntry) -> Result<(), String> {
    init_stores()?;
    let mut store = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("History store not initialized")?
        .add(entry)
}

/// Get history entries
pub fn get_history(
    project: Option<&Path>,
    limit: Option<usize>,
) -> Result<Vec<HistoryEntry>, String> {
    init_stores()?;
    let store = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("History store not initialized")?
        .get(project, limit)
        .into_iter()
        .cloned()
        .collect())
}

/// Search history entries
pub fn search_history(query: &str, project: Option<&Path>) -> Result<Vec<HistoryEntry>, String> {
    init_stores()?;
    let store = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("History store not initialized")?
        .search(query, project)
        .into_iter()
        .cloned()
        .collect())
}

/// Get a stored message by ID
pub fn get_message(id: Uuid) -> Result<Option<StoredMessage>, String> {
    init_stores()?;
    let store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Message store not initialized")?
        .get(id)
        .cloned())
}

/// Get all messages for a session
pub fn get_session_messages(session: &SessionManifest) -> Result<Vec<StoredMessage>, String> {
    init_stores()?;
    let store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Message store not initialized")?;

    let mut messages = Vec::new();
    for msg_ref in &session.messages {
        if let Some(msg) = store_ref.get(msg_ref.message_id) {
            messages.push(msg.clone());
        }
    }
    Ok(messages)
}

/// Update session token usage (ADDS to existing)
pub fn update_session_tokens(
    session: &mut SessionManifest,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
) -> Result<(), String> {
    session.update_token_usage(input, output, cache_read, cache_create);

    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Set session token usage (REPLACES existing - for restore scenarios)
///
/// CTX-003: Uses dual-metric fields for proper token tracking:
/// - `current_context_tokens`: Set to input (current context size for display)
/// - `cumulative_billed_input`: Separate param for billing analytics
/// - `cumulative_billed_output`: Separate param for billing analytics
pub fn set_session_tokens(
    session: &mut SessionManifest,
    input: u64,
    _output: u64, // Not used - persistence only tracks cumulative output
    cache_read: u64,
    cache_create: u64,
    cumulative_input: u64,
    cumulative_output: u64,
) -> Result<(), String> {
    // CTX-003: Use new dual-metric fields
    session.token_usage.current_context_tokens = input;
    session.token_usage.cumulative_billed_input = cumulative_input;
    session.token_usage.cumulative_billed_output = cumulative_output;
    session.token_usage.cache_read_tokens = cache_read;
    session.token_usage.cache_creation_tokens = cache_create;

    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Set compaction state for a session
pub fn set_compaction_state(
    session: &mut SessionManifest,
    summary: String,
    compacted_before_index: usize,
) -> Result<(), String> {
    session.compaction = Some(CompactionState {
        summary,
        compacted_before_index,
        compacted_at: chrono::Utc::now(),
    });

    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Clear compaction state for a session (e.g., when messages are added after compaction)
pub fn clear_compaction_state(session: &mut SessionManifest) -> Result<(), String> {
    session.compaction = None;

    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Get session lineage information
pub fn get_session_lineage(session: &SessionManifest) -> SessionLineage {
    SessionLineage {
        session_id: session.id,
        forked_from: session.forked_from.clone(),
        merged_from: session.merged_from.clone(),
    }
}

/// Session lineage information
#[derive(Debug, Clone)]
pub struct SessionLineage {
    pub session_id: Uuid,
    pub forked_from: Option<ForkPoint>,
    pub merged_from: Vec<MergeRecord>,
}
