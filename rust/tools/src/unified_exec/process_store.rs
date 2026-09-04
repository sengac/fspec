//! ProcessStore — Session-aware process management with LRU eviction.
//!
//! Stores running processes keyed by session_id. Enforces a maximum capacity
//! with LRU eviction that protects the N most recently used sessions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::time::Instant as TokioInstant;

use super::types::quiet_secs_since;
use super::{LRU_PROTECT_COUNT, MAX_UNIFIED_EXEC_PROCESSES};

/// TOOL-022 P4 (G1/G2/G3): platform-agnostic kill handle for a child
/// process. `Child` stays inside the entry (the reaper, the LRU
/// eviction path, and `quiet_secs` all need its `try_wait`); the
/// handle is the means by which an external waiter (the BashTool
/// delegation loop) terminates the whole tree on abort.
pub struct ChildHandle {
    pub(crate) kill: Arc<dyn Fn() + Send + Sync>,
}

impl ChildHandle {
    /// Kill the entire process tree (process group on Unix,
    /// `taskkill /T` on Windows).
    pub fn kill(&self) {
        (self.kill)();
    }
}

/// TOOL-022: monotonic epoch for quiet-time measurement.
///
/// Quiet time is measured in this clock's microseconds (deterministic,
/// saturating — u64 microseconds cannot wrap within any practical run).
static OUTPUT_CLOCK_EPOCH: once_cell::sync::Lazy<TokioInstant> =
    once_cell::sync::Lazy::new(TokioInstant::now);

/// Current monotonic clock time in microseconds from the output epoch.
///
/// Saturates at `u64::MAX` microseconds (≈584,942 millennia) — no panic
/// path, per workspace lint policy.
pub fn now_micros() -> u64 {
    let micros = TokioInstant::now()
        .duration_since(*OUTPUT_CLOCK_EPOCH)
        .as_micros();
    micros.min(u64::MAX as u128) as u64
}

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
    /// TOOL-022: last-output timestamp (monotonic micros, shared with the
    /// reader task). The single source of truth for `quiet_seconds` —
    /// updated on spawn and on every output read.
    pub last_output_micros: Arc<AtomicU64>,
    /// TOOL-022 P4: platform kill handle for the whole process tree
    /// (process group / taskkill), used by the BashTool delegation
    /// loop on ESC abort.
    pub kill_handle: ChildHandle,
}

/// Global process store. Thread-safe via tokio::sync::Mutex.
pub struct ProcessStore {
    entries: Mutex<HashMap<String, ProcessEntry>>,
    /// TOOL-022 P4: exit statuses reaped by the reaper whose entry was
    /// removed — retained briefly so a poller racing the reaper can
    /// recover the REAL exit code instead of the reaper-race `-1`
    /// (research §9.6). Lazy-pruned on each insert.
    exited: Mutex<HashMap<String, (std::process::ExitStatus, std::time::Instant)>>,
}

/// How long a reaped-but-removed session's exit status is retained for
/// `recover_exit`. Long enough to cover a full LLM poll window
/// (`MAX_YIELD_TIME_MS` = 30s).
const EXIT_STATUS_RETENTION: std::time::Duration = std::time::Duration::from_secs(60);

impl ProcessStore {
    /// Create a new empty ProcessStore.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            exited: Mutex::new(HashMap::new()),
        }
    }

    /// TOOL-022 P4: record the exit status of a session whose entry was
    /// removed (reaper / eviction), so a racing poller can recover it.
    pub async fn stash_exit(&self, session_id: &str, status: std::process::ExitStatus) {
        let mut exited = self.exited.lock().await;
        // Lazy prune: drop statuses older than the retention window.
        let now = Instant::now();
        exited.retain(|_, (_, at)| now.duration_since(*at) <= EXIT_STATUS_RETENTION);
        exited.insert(session_id.to_string(), (status, now));
    }

    /// TOOL-022 P4: recover the exit status of a session whose entry is
    /// gone (reaped by the reaper after our `get_output_handles`).
    pub async fn recover_exit(&self, session_id: &str) -> Option<std::process::ExitStatus> {
        let exited = self.exited.lock().await;
        exited
            .get(session_id)
            .filter(|(_, at)| Instant::now().duration_since(*at) <= EXIT_STATUS_RETENTION)
            .map(|(status, _)| *status)
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

    /// True when the session's `last_used` is within `within` of now
    /// (i.e., an active poller is touching it right now).
    pub async fn is_recently_used(&self, session_id: &str, within: std::time::Duration) -> bool {
        let entries = self.entries.lock().await;
        entries
            .get(session_id)
            .is_some_and(|e| e.last_used.elapsed() <= within)
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
        entries
            .iter()
            .map(|(id, entry)| SessionInfo {
                session_id: id.clone(),
                command: entry.command_display.clone(),
                tty: entry.tty,
            })
            .collect()
    }

    /// Evict the least recently used session (not in the top N most recent).
    /// Returns the evicted session_id, or None if store is under capacity.
    pub async fn evict_lru_if_full(&self) -> Option<String> {
        let mut entries = self.entries.lock().await;
        if entries.len() < MAX_UNIFIED_EXEC_PROCESSES {
            return None;
        }

        // Build metadata for the pure selection function
        let meta: Vec<(String, Instant, bool)> = entries
            .iter_mut()
            .map(|(id, e)| {
                let has_exited = e.child.try_wait().map(|s| s.is_some()).unwrap_or(false);
                (id.clone(), e.last_used, has_exited)
            })
            .collect();

        let victim_id = session_id_to_evict(&meta);

        if let Some(ref id) = victim_id {
            let mut entry = entries.remove(id);
            // Kill the process (TOOL-022 P4: the full process tree via
            // the kill handle — for PTY sessions the direct `Child` is
            // only the liveness anchor, NOT the PTY shell).
            if let Some(ref mut e) = entry {
                e.kill_handle.kill();
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
    pub async fn get_output_handles(
        &self,
        session_id: &str,
    ) -> Option<(Arc<Mutex<Vec<u8>>>, Arc<Notify>)> {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(session_id) {
            entry.last_used = Instant::now();
            Some((
                Arc::clone(&entry.output_buffer),
                Arc::clone(&entry.output_notify),
            ))
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

    /// TOOL-022: seconds since the session's last output (floored).
    ///
    /// None when the session does not exist. Pure timestamp arithmetic —
    /// no output content is inspected.
    pub async fn quiet_secs(&self, session_id: &str) -> Option<u64> {
        let entries = self.entries.lock().await;
        let entry = entries.get(session_id)?;
        Some(quiet_secs_since(
            entry.last_output_micros.load(Ordering::Relaxed),
            now_micros(),
        ))
    }

    /// TOOL-022 P4: the platform kill handle for the session's child
    /// (cloned — the handle is an `Arc<dyn Fn>`). `None` when the
    /// session does not exist.
    pub async fn kill_handle(&self, session_id: &str) -> Option<ChildHandle> {
        let entries = self.entries.lock().await;
        entries.get(session_id).map(|e| ChildHandle {
            kill: Arc::clone(&e.kill_handle.kill),
        })
    }

    /// TOOL-022 P4: terminate the session and remove it from the store
    /// (the BashTool abort path). `Ok(true)` when the session existed
    /// and was killed, `Ok(false)` when it was already gone (reaper /
    /// poll drained it first) — NOT an error.
    pub async fn close_session(&self, session_id: &str) -> Result<bool, String> {
        let mut entries = self.entries.lock().await;
        match entries.remove(session_id) {
            Some(mut entry) => {
                // TOOL-022 P4: full process tree (PTY child + anchor /
                // process group), not just the direct child.
                entry.kill_handle.kill();
                let _ = entry.child.kill().await;
                Ok(true)
            }
            None => Ok(false),
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
    if let Some((id, _, _)) = lru
        .iter()
        .find(|(id, _, exited)| !protected.contains(id.as_str()) && *exited)
    {
        return Some(id.clone());
    }

    // Fall back to oldest unprotected session
    lru.into_iter()
        .find(|(id, _, _)| !protected.contains(id.as_str()))
        .map(|(id, _, _)| id.clone())
}
