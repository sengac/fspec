//! Session manifest + on-disk session store, lifted from
//! `rust/napi/src/persistence/{storage,types,mod}.rs` in RPC-033 so
//! codelet-rpc, codelet-rpc-embedded, codelet-rpc-server, and the
//! upcoming codelet-sessions crate can manage session manifests without
//! re-introducing the forbidden `rpc → napi` arrow.
//!
//! On-disk layout (byte-identical with the pre-lift NAPI store):
//!
//! - `{data_dir}/sessions/{uuid}.json` — pretty-printed `SessionManifest`
//!   JSON, one file per session.
//! - `{data_dir}/messages/messages.jsonl` + `messages.idx` — owned by
//!   [`crate::persistence::messages::MessageStore`] (lifted in RPC-032).
//!
//! Singleton ownership: this module holds the global lazy_static
//! `MESSAGE_STORE` and `SESSION_STORE` mutex-protected caches that the
//! free-function facade (`create_session`, `load_session`,
//! `append_message_with_metadata`, …) routes through. They MUST be
//! cleared whenever the data directory changes — see
//! [`reset_stores_for_tests`].

use chrono::{DateTime, Utc};
use codelet_common::token_estimator::count_tokens;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::persistence::messages::{MessageRef, MessageSource, MessageStore, StoredMessage};

// ============================================================================
// Lifted value types (formerly napi::persistence::types)
// ============================================================================

/// Token usage tracking for a session.
///
/// CTX-003: distinguishes between current context size and cumulative
/// billing. The Anthropic API reports `input_tokens` as the TOTAL context
/// size per call (absolute, not incremental); display surfaces should use
/// `current_context_tokens`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TokenUsage {
    /// Current context size (latest input_tokens from API — overwritten,
    /// not accumulated).
    #[serde(default)]
    pub current_context_tokens: u64,
    /// Cumulative billed input tokens (sum across all API calls).
    #[serde(default)]
    pub cumulative_billed_input: u64,
    /// Cumulative billed output tokens (sum across all API calls).
    #[serde(default)]
    pub cumulative_billed_output: u64,
    /// Cache read tokens from current API call.
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Cache creation tokens from current API call.
    #[serde(default)]
    pub cache_creation_tokens: u64,
}

/// Record of a merge operation for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRecord {
    /// Session messages were merged from.
    pub source_session_id: Uuid,
    /// Which message indices were imported.
    pub source_indices: Vec<usize>,
    /// Where they were inserted (None = appended).
    pub inserted_at: Option<usize>,
    /// When the merge occurred.
    pub merged_at: DateTime<Utc>,
}

/// Records when and where a session was forked from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkPoint {
    /// The session this was forked from.
    pub source_session_id: Uuid,
    /// The message index at which the fork occurred (inclusive).
    pub fork_after_index: usize,
    /// When the fork happened.
    pub forked_at: DateTime<Utc>,
}

/// Tracks compaction state for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactionState {
    /// Summary of compacted messages.
    pub summary: String,
    /// Messages 0..compacted_before_index are compacted.
    pub compacted_before_index: usize,
    /// When compaction occurred.
    pub compacted_at: DateTime<Utc>,
}

/// A session manifest — ordered list of message references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    /// Unique session identifier.
    pub id: Uuid,
    /// Human-readable session name.
    pub name: String,
    /// Project path this session belongs to.
    pub project: PathBuf,
    /// Provider used (claude, openai, gemini, codex, etc.).
    #[serde(default)]
    pub provider: String,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last updated.
    pub updated_at: DateTime<Utc>,
    /// Ordered list of message references.
    pub messages: Vec<MessageRef>,
    /// If this session was forked, records the fork point.
    pub forked_from: Option<ForkPoint>,
    /// Record of merges from other sessions.
    #[serde(default)]
    pub merged_from: Vec<MergeRecord>,
    /// If this session has been compacted, records the state.
    pub compaction: Option<CompactionState>,
    /// Token usage statistics.
    #[serde(default)]
    pub token_usage: TokenUsage,
}

impl SessionManifest {
    /// Create a new empty session.
    pub fn new(name: &str, project: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            project,
            provider: String::new(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            forked_from: None,
            merged_from: Vec::new(),
            compaction: None,
            token_usage: TokenUsage::default(),
        }
    }

    /// Create a new session with provider.
    pub fn with_provider(name: &str, project: PathBuf, provider: &str) -> Self {
        let mut session = Self::new(name, project);
        session.provider = provider.to_string();
        session
    }

    /// Add a message reference to this session.
    pub fn add_message(&mut self, message_id: Uuid, source: MessageSource) {
        self.messages.push(MessageRef { message_id, source });
        self.updated_at = Utc::now();
    }

    /// Record a merge operation.
    pub fn record_merge(&mut self, record: MergeRecord) {
        self.merged_from.push(record);
        self.updated_at = Utc::now();
    }

    /// Number of messages in this session.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Update token usage with dual metrics (CTX-003).
    pub fn update_token_usage(
        &mut self,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_create: u64,
    ) {
        self.token_usage.current_context_tokens = input;
        self.token_usage.cumulative_billed_input += input;
        self.token_usage.cumulative_billed_output += output;
        self.token_usage.cache_read_tokens = cache_read;
        self.token_usage.cache_creation_tokens = cache_create;
    }
}

/// Session lineage information returned by [`get_session_lineage`].
#[derive(Debug, Clone)]
pub struct SessionLineage {
    pub session_id: Uuid,
    pub forked_from: Option<ForkPoint>,
    pub merged_from: Vec<MergeRecord>,
}

// ============================================================================
// SessionStore — on-disk session-manifest cache
// ============================================================================

/// Session store — handles storing and retrieving session manifests.
///
/// Internally caches every manifest in memory plus a per-project "last
/// active session" pointer used by [`SessionStore::resume_last`]. Files
/// are persisted as pretty-printed JSON at
/// `{data_dir}/sessions/{uuid}.json`.
pub struct SessionStore {
    sessions_dir: PathBuf,
    /// In-memory cache of sessions by ID.
    cache: HashMap<Uuid, SessionManifest>,
    /// Track the last active session per project.
    last_session: HashMap<PathBuf, Uuid>,
}

impl SessionStore {
    /// Create a new session store.
    ///
    /// Resolves the data directory via [`codelet_common::get_data_dir`]
    /// and ensures `{data_dir}/sessions/` exists.
    pub fn new() -> Result<Self, String> {
        let data_dir = codelet_common::get_data_dir()?;
        let sessions_dir = data_dir.join("sessions");
        fs::create_dir_all(&sessions_dir)
            .map_err(|e| format!("Failed to create sessions dir {sessions_dir:?}: {e}"))?;
        let mut store = Self {
            sessions_dir,
            cache: HashMap::new(),
            last_session: HashMap::new(),
        };
        store.load_all()?;
        Ok(store)
    }

    /// Load all sessions from disk.
    fn load_all(&mut self) -> Result<(), String> {
        if !self.sessions_dir.exists() {
            debug!(
                sessions_dir = %self.sessions_dir.display(),
                "SessionStore::load_all: sessions directory does not exist, nothing to load"
            );
            return Ok(());
        }

        for entry in fs::read_dir(&self.sessions_dir)
            .map_err(|e| format!("Failed to read sessions dir: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "json") {
                let content = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Skipping unreadable session file {:?}: {}", path, e);
                        continue;
                    }
                };
                let session: SessionManifest = match serde_json::from_str(&content) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Skipping incompatible session file {:?}: {}", path, e);
                        continue;
                    }
                };

                self.last_session
                    .insert(session.project.clone(), session.id);
                self.cache.insert(session.id, session);
            }
        }

        info!(
            sessions_dir = %self.sessions_dir.display(),
            loaded_count = self.cache.len(),
            "SessionStore::load_all: finished loading sessions from disk"
        );

        Ok(())
    }

    /// Create a new session.
    pub fn create(&mut self, name: &str, project: &Path) -> Result<SessionManifest, String> {
        let session = SessionManifest::new(name, project.to_path_buf());
        self.save(&session)?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Create a new session with provider.
    pub fn create_with_provider(
        &mut self,
        name: &str,
        project: &Path,
        provider: &str,
    ) -> Result<SessionManifest, String> {
        let session = SessionManifest::with_provider(name, project.to_path_buf(), provider);
        info!(
            session_id = %session.id,
            provider = %provider,
            name = %name,
            "SessionStore::create_with_provider: created new session manifest"
        );
        self.save(&session)?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Save a session to disk.
    pub fn save(&mut self, session: &SessionManifest) -> Result<(), String> {
        let filename = format!("{}.json", session.id);
        let path = self.sessions_dir.join(&filename);

        tracing::info!(
            session_id = %session.id,
            session_name = %session.name,
            provider = %session.provider,
            project = %session.project.display(),
            message_count = session.messages.len(),
            save_path = %path.display(),
            "SessionStore::save: persisting session manifest to disk"
        );

        let json = serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to serialize session: {e}"))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write session file: {e}"))?;

        tracing::info!(
            session_id = %session.id,
            save_path = %path.display(),
            "SessionStore::save: session manifest written to disk successfully"
        );

        self.cache.insert(session.id, session.clone());
        self.last_session
            .insert(session.project.clone(), session.id);

        Ok(())
    }

    /// Get a session by ID.
    pub fn get(&self, id: Uuid) -> Option<&SessionManifest> {
        self.cache.get(&id)
    }

    /// Get a mutable reference to a session.
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut SessionManifest> {
        self.cache.get_mut(&id)
    }

    /// Load a session by ID (returns owned value).
    pub fn load(&self, id: Uuid) -> Result<SessionManifest, String> {
        self.cache
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("Session {id} not found"))
    }

    /// Get the last active session for a project.
    pub fn get_last_session(&self, project: &Path) -> Option<&SessionManifest> {
        self.last_session
            .get(project)
            .and_then(|id| self.cache.get(id))
    }

    /// Resume the last session for a project.
    pub fn resume_last(&self, project: &Path) -> Result<SessionManifest, String> {
        self.get_last_session(project)
            .cloned()
            .ok_or_else(|| format!("No session found for project {project:?}"))
    }

    /// List all sessions for a project.
    pub fn list_for_project(&self, project: &Path) -> Vec<&SessionManifest> {
        self.cache
            .values()
            .filter(|s| s.project == project)
            .collect()
    }

    /// List all sessions.
    pub fn list_all(&self) -> Vec<&SessionManifest> {
        self.cache.values().collect()
    }

    /// Delete a session.
    pub fn delete(&mut self, id: Uuid) -> Result<(), String> {
        let session = self.cache.remove(&id);
        if let Some(session) = session {
            if self.last_session.get(&session.project) == Some(&id) {
                self.last_session.remove(&session.project);
            }
        }

        // Always attempt the file delete — idempotent if absent.
        let filename = format!("{id}.json");
        let path = self.sessions_dir.join(&filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete session file: {e}"))?;
        }
        Ok(())
    }

    /// Rename a session.
    pub fn rename(&mut self, id: Uuid, new_name: &str) -> Result<(), String> {
        let session = self
            .cache
            .get_mut(&id)
            .ok_or_else(|| format!("Session {id} not found"))?;
        session.name = new_name.to_string();
        session.updated_at = Utc::now();

        let session_clone = session.clone();
        self.save(&session_clone)?;
        Ok(())
    }

    /// Fork a session at a specific message index.
    pub fn fork(
        &mut self,
        source: &SessionManifest,
        at_index: usize,
        name: &str,
    ) -> Result<SessionManifest, String> {
        if at_index >= source.messages.len() {
            return Err(format!(
                "Fork index {} is out of range (session has {} messages)",
                at_index,
                source.messages.len()
            ));
        }

        if let Some(ref compaction) = source.compaction {
            if at_index < compaction.compacted_before_index {
                return Err(format!(
                    "Cannot fork at index {} which is before compaction boundary {}. \
                     Compacted messages cannot be individually accessed. \
                     Fork at index {} or later.",
                    at_index, compaction.compacted_before_index, compaction.compacted_before_index
                ));
            }
        }

        let mut new_session = if source.provider.is_empty() {
            SessionManifest::new(name, source.project.clone())
        } else {
            SessionManifest::with_provider(name, source.project.clone(), &source.provider)
        };

        new_session.forked_from = Some(ForkPoint {
            source_session_id: source.id,
            fork_after_index: at_index,
            forked_at: Utc::now(),
        });

        for (i, msg_ref) in source.messages.iter().enumerate() {
            if i > at_index {
                break;
            }
            new_session.messages.push(MessageRef {
                message_id: msg_ref.message_id,
                source: MessageSource::Forked {
                    from_session: source.id,
                },
            });
        }

        if let Some(ref compaction) = source.compaction {
            if at_index >= compaction.compacted_before_index {
                new_session.compaction = Some(compaction.clone());
            }
        }

        self.save(&new_session)?;
        self.cache.insert(new_session.id, new_session.clone());

        Ok(new_session)
    }
}

// ============================================================================
// Global singleton stores
// ============================================================================
//
// MESSAGE_STORE and SESSION_STORE are the process-wide caches used by the
// free-function facade below. They mirror the pre-RPC-033 NAPI layout so
// the on-disk format stays byte-identical and the existing 80+ persistence
// tests can be exercised through the NAPI re-export shim.
//
// BLOB_STORE lives in the sibling `blob` module (lifted by RPC-034); the
// [`reset_stores_for_tests`] helper above resets it alongside MESSAGE_STORE
// and SESSION_STORE.

lazy_static::lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
}

fn init_message_store() -> Result<(), String> {
    let mut store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() {
        *store = Some(MessageStore::new()?);
    }
    Ok(())
}

fn init_session_store() -> Result<(), String> {
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() {
        *store = Some(SessionStore::new()?);
    }
    Ok(())
}

/// Reset the lifted MESSAGE_STORE, SESSION_STORE, BLOB_STORE and history singletons.
///
/// Called by `codelet_napi::persistence::set_data_directory` after it
/// updates the shared `codelet_common` data dir, so the next persistence
/// operation re-initialises against the new directory. Also used by
/// codelet-core integration tests that swap the data dir under the
/// process's feet.
pub fn reset_stores_for_tests() {
    if let Ok(mut msg) = MESSAGE_STORE.lock() {
        *msg = None;
    }
    if let Ok(mut sess) = SESSION_STORE.lock() {
        *sess = None;
    }
    // history.rs owns its own singleton and reset helper (RPC-025).
    crate::persistence::history::reset_for_tests();
    // blob.rs owns its own singleton and reset helper (RPC-034).
    crate::persistence::blob::reset_blob_store_for_tests();
}

/// Reset only the MESSAGE_STORE singleton.
///
/// Used by the BUG-122 lazy-init tests in codelet-napi to force the
/// MessageStore to re-load from disk between operations without
/// disturbing the SESSION_STORE cache.
pub fn reset_message_store_for_tests() {
    if let Ok(mut msg) = MESSAGE_STORE.lock() {
        *msg = None;
    }
}

/// Reset only the SESSION_STORE singleton.
///
/// Symmetric counterpart to [`reset_message_store_for_tests`].
pub fn reset_session_store_for_tests() {
    if let Ok(mut sess) = SESSION_STORE.lock() {
        *sess = None;
    }
}

/// Test-only accessor: is the lifted MESSAGE_STORE singleton currently
/// initialised?
///
/// Used by `codelet_napi::persistence::lazy_init_tests` to verify
/// BUG-122's per-store lazy initialisation invariants without poking the
/// lazy_static globals directly. Matches the pattern set by
/// [`crate::persistence::history::is_initialized_for_tests`].
pub fn is_message_store_initialized_for_tests() -> bool {
    MESSAGE_STORE.lock().map(|s| s.is_some()).unwrap_or(false)
}

/// Test-only accessor: is the lifted SESSION_STORE singleton currently
/// initialised?
pub fn is_session_store_initialized_for_tests() -> bool {
    SESSION_STORE.lock().map(|s| s.is_some()).unwrap_or(false)
}

// ============================================================================
// Free-function facade (formerly napi::persistence::mod)
// ============================================================================

/// Create a new session.
pub fn create_session(name: &str, project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create(name, project)
}

/// Create a new session with provider.
pub fn create_session_with_provider(
    name: &str,
    project: &Path,
    provider: &str,
) -> Result<SessionManifest, String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create_with_provider(name, project, provider)
}

/// Save a session manifest to disk (for externally-created manifests).
pub fn save_session(session: &SessionManifest) -> Result<(), String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)
}

/// Load a session by ID.
pub fn load_session(id: Uuid) -> Result<SessionManifest, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Session store not initialized")?
        .load(id)
}

/// Resume the last session for a project.
pub fn resume_last_session(project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Session store not initialized")?
        .resume_last(project)
}

/// Fork a session at a specific message index.
pub fn fork_session(
    session: &SessionManifest,
    at_index: usize,
    name: &str,
) -> Result<SessionManifest, String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .fork(session, at_index, name)
}

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
/// 2. If the global `SESSION_STORE` cache happens to be initialised,
///    also evicts the entry + last-session bookkeeping so the in-memory
///    state matches the on-disk state.
pub fn delete_session(id: Uuid) -> Result<(), String> {
    // (1) Always remove the on-disk file using the current data dir.
    let sessions_dir = codelet_common::get_data_dir()?.join("sessions");
    let path = sessions_dir.join(format!("{id}.json"));
    if path.exists() {
        tracing::info!(
            session_id = %id,
            manifest_path = %path.display(),
            "delete_session: removing session manifest from disk"
        );
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete session file {}: {e}", path.display()))?;
        tracing::info!(
            session_id = %id,
            "delete_session: session manifest removed from disk"
        );
    } else {
        tracing::debug!(
            session_id = %id,
            manifest_path = %path.display(),
            "delete_session: manifest file does not exist, nothing to delete"
        );
    }

    // (2) Best-effort in-memory cache eviction if the singleton happens
    // to already exist. We do NOT initialise the store just to delete
    // from it — that would force a full scan of `sessions/` on a path
    // that has nothing useful for us.
    if let Ok(mut store) = SESSION_STORE.lock() {
        if let Some(store) = store.as_mut() {
            let removed = store.cache.remove(&id);
            if let Some(session) = removed {
                tracing::info!(
                    session_id = %id,
                    session_name = %session.name,
                    project = %session.project.display(),
                    "delete_session: evicted session from in-memory cache"
                );
                if store.last_session.get(&session.project) == Some(&id) {
                    store.last_session.remove(&session.project);
                }
            } else {
                tracing::debug!(
                    session_id = %id,
                    "delete_session: session not found in in-memory cache"
                );
            }
        }
    }

    Ok(())
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

/// Append a message to a session.
pub fn append_message(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
) -> Result<Uuid, String> {
    init_message_store()?;
    init_session_store()?;

    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_id = msg_store
        .as_mut()
        .ok_or("Message store not initialized")?
        .store(role, content)?;

    session.add_message(msg_id, MessageSource::Native);

    drop(msg_store);
    let mut sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    sess_store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(msg_id)
}

/// Append a message with metadata to a session.
pub fn append_message_with_metadata(
    session: &mut SessionManifest,
    role: &str,
    content: &str,
    metadata: HashMap<String, serde_json::Value>,
) -> Result<Uuid, String> {
    init_message_store()?;
    init_session_store()?;

    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_id = msg_store
        .as_mut()
        .ok_or("Message store not initialized")?
        .store_with_metadata(role, content, metadata)?;

    session.add_message(msg_id, MessageSource::Native);

    drop(msg_store);
    let mut sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    sess_store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(msg_id)
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

/// Cleanup orphaned messages (not referenced by any session).
pub fn cleanup_orphaned_messages() -> Result<usize, String> {
    init_message_store()?;
    init_session_store()?;

    let sess_store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    let sessions: Vec<SessionManifest> = sess_store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_all()
        .into_iter()
        .cloned()
        .collect();
    drop(sess_store);

    let mut msg_store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let msg_store_ref = msg_store.as_mut().ok_or("Message store not initialized")?;
    let referenced: HashSet<Uuid> = sessions
        .iter()
        .flat_map(|s| s.messages.iter().map(|m| m.message_id))
        .collect();

    msg_store_ref.cleanup_orphans(&referenced)
}

/// Get a stored message by ID.
pub fn get_message(id: Uuid) -> Result<Option<StoredMessage>, String> {
    init_message_store()?;
    let store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Message store not initialized")?
        .get(id))
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
            if let Some(msg) = store_ref.get(msg_ref.message_id) {
                messages.push(msg);
            }
        }
    } else {
        for msg_ref in &session.messages {
            if let Some(msg) = store_ref.get(msg_ref.message_id) {
                messages.push(msg);
            }
        }
    }

    Ok(messages)
}

/// Get ALL messages for a session, ignoring compaction state.
pub fn get_session_messages_full(session: &SessionManifest) -> Result<Vec<StoredMessage>, String> {
    init_message_store()?;
    let store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_ref().ok_or("Message store not initialized")?;

    let mut messages = Vec::new();
    for msg_ref in &session.messages {
        if let Some(msg) = store_ref.get(msg_ref.message_id) {
            messages.push(msg);
        }
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
    let session = load_session(session_id)?;
    let messages = get_session_messages(&session)?;

    let mut envelopes: Vec<String> = Vec::with_capacity(messages.len());
    for stored_msg in messages {
        // Handle synthetic compaction summary messages — MessageEnvelope
        // uses #[serde(rename_all = "camelCase")] + #[serde(rename = "type")].
        // Required fields: uuid, timestamp, type, provider, message.
        if stored_msg.id == Uuid::nil() {
            let synthetic = serde_json::json!({
                "uuid": "00000000-0000-0000-0000-000000000000",
                "parentUuid": null,
                "timestamp": stored_msg.created_at.to_rfc3339(),
                "type": "user",
                "provider": "compaction",
                "message": {
                    "role": "user",
                    "content": [{"type": "text", "text": stored_msg.content}]
                },
                "requestId": null,
                "_synthetic": true,
                "_compactionSummary": true
            });
            envelopes.push(serde_json::to_string(&synthetic)
                .map_err(|e| format!("Failed to serialize synthetic envelope: {e}"))?);
            continue;
        }

        // RPC-422: Construct a proper MessageEnvelope from StoredMessage fields.
        // When messages are stored via persist_assistant_message_internal, the
        // metadata contains the original structured message content (with
        // thinking/text/tool_use blocks). Use that when available so the
        // restore path preserves the original block structure.
        // When metadata is empty (simple append_message), fall back to
        // constructing from StoredMessage's own fields.
        let message_content = stored_msg
            .metadata
            .get("message")
            .and_then(|m| m.get("content"))
            .cloned();

        let content_value = match message_content {
            Some(c) => c,
            None => {
                // No structured metadata — fall back to single text block.
                serde_json::json!([{"type": "text", "text": stored_msg.content}])
            }
        };

        let envelope_json = serde_json::json!({
            "uuid": stored_msg.id.to_string(),
            "parentUuid": null,
            "timestamp": stored_msg.created_at.to_rfc3339(),
            "type": stored_msg.role,
            "provider": stored_msg.metadata.get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            "message": {
                "role": stored_msg.role,
                "content": content_value
            },
            "requestId": null
        });
        let envelope_str = serde_json::to_string(&envelope_json)
            .map_err(|e| format!("Failed to serialize envelope: {e}"))?;
        let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&envelope_str)?;
        envelopes.push(rehydrated);
    }

    Ok(envelopes)
}

/// Update session token usage (ADDS to existing).
pub fn update_session_tokens(
    session: &mut SessionManifest,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
) -> Result<(), String> {
    session.update_token_usage(input, output, cache_read, cache_create);

    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Set session token usage (REPLACES existing — restore scenario).
pub fn set_session_tokens(
    session: &mut SessionManifest,
    input: u64,
    _output: u64,
    cache_read: u64,
    cache_create: u64,
    cumulative_input: u64,
    cumulative_output: u64,
) -> Result<(), String> {
    session.token_usage.current_context_tokens = input;
    session.token_usage.cumulative_billed_input = cumulative_input;
    session.token_usage.cumulative_billed_output = cumulative_output;
    session.token_usage.cache_read_tokens = cache_read;
    session.token_usage.cache_creation_tokens = cache_create;

    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Set compaction state for a session.
pub fn set_compaction_state(
    session: &mut SessionManifest,
    summary: String,
    compacted_before_index: usize,
) -> Result<(), String> {
    session.compaction = Some(CompactionState {
        summary,
        compacted_before_index,
        compacted_at: Utc::now(),
    });

    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Clear compaction state for a session.
pub fn clear_compaction_state(session: &mut SessionManifest) -> Result<(), String> {
    session.compaction = None;

    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)?;

    Ok(())
}

/// Get session lineage information.
pub fn get_session_lineage(session: &SessionManifest) -> SessionLineage {
    SessionLineage {
        session_id: session.id,
        forked_from: session.forked_from.clone(),
        merged_from: session.merged_from.clone(),
    }
}

/// List all sessions for a specific project (owned values).
pub fn list_sessions_for_project(project: &Path) -> Result<Vec<SessionManifest>, String> {
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

/// List all sessions across all projects.
pub fn list_all_sessions() -> Result<Vec<SessionManifest>, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_all()
        .into_iter()
        .cloned()
        .collect())
}
