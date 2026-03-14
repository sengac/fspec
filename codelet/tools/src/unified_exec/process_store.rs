//! ProcessStore — Session-aware process management with LRU eviction.
//!
//! Stores running processes keyed by session_id. Enforces a maximum capacity
//! with LRU eviction that protects the N most recently used sessions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex, Notify};

use super::{MAX_UNIFIED_EXEC_PROCESSES, LRU_PROTECT_COUNT};

/// A single managed process entry in the store.
pub struct ProcessEntry {
    /// The child process handle
    pub child: Child,
    /// Channel to send bytes to the process stdin
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
    /// Buffered output from the process (stdout + stderr interleaved)
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    /// Notification when new output is available
    pub output_notify: Arc<Notify>,
    /// Last time this session was accessed (for LRU)
    pub last_used: Instant,
    /// Whether this session uses a PTY
    pub tty: bool,
    /// The command that was executed (for list display)
    pub command_display: String,
}

/// Global process store. Thread-safe via tokio::sync::Mutex.
pub struct ProcessStore {
    entries: Mutex<HashMap<String, ProcessEntry>>,
}

impl ProcessStore {
    /// Create a new empty ProcessStore.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Insert a new process entry. Returns the session_id.
    pub async fn insert(&self, session_id: String, entry: ProcessEntry) {
        let mut entries = self.entries.lock().await;
        entries.insert(session_id, entry);
    }

    /// Get mutable access to an entry, updating last_used.
    pub async fn touch(&self, session_id: &str) -> bool {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(session_id) {
            entry.last_used = Instant::now();
            true
        } else {
            false
        }
    }

    /// Check if a session exists.
    pub async fn contains(&self, session_id: &str) -> bool {
        let entries = self.entries.lock().await;
        entries.contains_key(session_id)
    }

    /// Remove a session, returning the entry for cleanup.
    pub async fn remove(&self, session_id: &str) -> Option<ProcessEntry> {
        let mut entries = self.entries.lock().await;
        entries.remove(session_id)
    }

    /// Number of active sessions.
    pub async fn len(&self) -> usize {
        let entries = self.entries.lock().await;
        entries.len()
    }

    /// Whether the store has no active sessions.
    pub async fn is_empty(&self) -> bool {
        let entries = self.entries.lock().await;
        entries.is_empty()
    }

    /// List all session IDs with metadata.
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let entries = self.entries.lock().await;
        entries.iter().map(|(id, entry)| {
            SessionInfo {
                session_id: id.clone(),
                command: entry.command_display.clone(),
                tty: entry.tty,
            }
        }).collect()
    }

    /// Evict the least recently used session (not in the top N most recent).
    /// Returns the evicted session_id, or None if store is under capacity.
    pub async fn evict_lru_if_full(&self) -> Option<String> {
        let mut entries = self.entries.lock().await;
        if entries.len() < MAX_UNIFIED_EXEC_PROCESSES {
            return None;
        }

        // Build metadata for the pure selection function
        let meta: Vec<(String, Instant, bool)> = entries.iter_mut().map(|(id, e)| {
            let has_exited = e.child.try_wait().map(|s| s.is_some()).unwrap_or(false);
            (id.clone(), e.last_used, has_exited)
        }).collect();

        let victim_id = session_id_to_evict(&meta);

        if let Some(ref id) = victim_id {
            let mut entry = entries.remove(id);
            // Kill the process
            if let Some(ref mut e) = entry {
                let _ = e.child.kill().await;
            }
        }

        victim_id
    }

    /// Get the stdin sender for a session.
    pub async fn get_stdin_tx(&self, session_id: &str) -> Option<mpsc::Sender<Vec<u8>>> {
        let entries = self.entries.lock().await;
        entries.get(session_id).map(|e| e.stdin_tx.clone())
    }

    /// Get the output buffer and notify handle for a session.
    pub async fn get_output_handles(&self, session_id: &str) -> Option<(Arc<Mutex<Vec<u8>>>, Arc<Notify>)> {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(session_id) {
            entry.last_used = Instant::now();
            Some((Arc::clone(&entry.output_buffer), Arc::clone(&entry.output_notify)))
        } else {
            None
        }
    }

    /// Check if a process has exited (non-blocking).
    pub async fn try_wait(&self, session_id: &str) -> Option<Option<std::process::ExitStatus>> {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(session_id) {
            match entry.child.try_wait() {
                Ok(status) => Some(status),
                Err(_) => Some(None),
            }
        } else {
            None
        }
    }

    /// Get all session IDs (for reaper iteration).
    pub async fn session_ids(&self) -> Vec<String> {
        let entries = self.entries.lock().await;
        entries.keys().cloned().collect()
    }

}

impl Default for ProcessStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Session info returned by the list action.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub command: String,
    pub tty: bool,
}

// ============================================================================
// Global singleton
// ============================================================================

use once_cell::sync::Lazy;

/// Global ProcessStore singleton.
static GLOBAL_STORE: Lazy<ProcessStore> = Lazy::new(ProcessStore::new);

/// Get a reference to the global ProcessStore.
pub fn global_store() -> &'static ProcessStore {
    &GLOBAL_STORE
}

// ============================================================================
// LRU Eviction Policy (pure function, testable without processes)
// ============================================================================

/// Select the session ID to evict from a list of (id, last_used, has_exited) metadata.
///
/// Policy (matches Codex reference):
/// 1. Protect the 8 most recently used sessions from eviction.
/// 2. Among unprotected sessions, prefer evicting already-exited ones first.
/// 3. If no exited sessions outside protected set, evict the least recently used.
/// 4. Returns None if the metadata is empty or all sessions are protected.
pub fn session_id_to_evict(meta: &[(String, Instant, bool)]) -> Option<String> {
    if meta.is_empty() {
        return None;
    }

    // Sort by recency (most recent first) to identify the protected set
    let mut by_recency: Vec<(String, Instant, bool)> = meta.to_vec();
    by_recency.sort_by_key(|(_, last_used, _)| std::cmp::Reverse(*last_used));

    let protected: std::collections::HashSet<&str> = by_recency
        .iter()
        .take(LRU_PROTECT_COUNT)
        .map(|(id, _, _)| id.as_str())
        .collect();

    // Sort by LRU (oldest first)
    let mut lru: Vec<&(String, Instant, bool)> = meta.iter().collect();
    lru.sort_by_key(|(_, last_used, _)| *last_used);

    // Prefer evicting exited sessions outside the protected set
    if let Some((id, _, _)) = lru.iter().find(|(id, _, exited)| !protected.contains(id.as_str()) && *exited) {
        return Some(id.clone());
    }

    // Fall back to oldest unprotected session
    lru.into_iter()
        .find(|(id, _, _)| !protected.contains(id.as_str()))
        .map(|(id, _, _)| id.clone())
}
