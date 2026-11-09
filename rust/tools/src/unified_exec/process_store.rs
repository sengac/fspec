//! ProcessStore — Session-aware process management with LRU eviction.
//!
//! Stores running processes keyed by session_id. Enforces a maximum capacity
//! with LRU eviction that protects the N most recently used sessions.

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::{Mutex, Notify};
use tokio::time::Instant as TokioInstant;

/// TOOL-022 P4 (G1/G2/G3): platform-agnostic kill handle for a child
/// process. `Child` stays inside the entry (the reaper, the LRU
/// eviction path, and `quiet_secs` all need its `try_wait`); the
/// handle is the means by which an external waiter (the BashTool
/// delegation loop) terminates the whole tree on abort.
#[derive(Clone)]
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
    pub(super) owner_registration: Option<super::owner_scope::ProcessRegistration>,
    /// The child process handle
    pub child: Child,
    /// Channel to send bytes to the process stdin
    pub stdin_tx: super::stdin_write::StdinSender,
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
    /// BUG-368: for a PTY session, where the PTY child's real status is
    /// recorded (`child` is only the liveness anchor); `None` for pipes.
    pub pty_exit: Option<super::exec_exit::PtyExitSlot>,
}

/// Global process store. Thread-safe via tokio::sync::Mutex.
pub struct ProcessStore {
    pub(super) entries: Mutex<HashMap<String, ProcessEntry>>,
    /// TOOL-022 P4: exit statuses reaped by the reaper whose entry was
    /// removed — retained briefly so a poller racing the reaper can
    /// recover the REAL exit code instead of the reaper-race `-1`
    /// (research §9.6). Lazy-pruned on each insert.
    exited: Mutex<HashMap<String, (super::exec_exit::ExecExit, std::time::Instant)>>,
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

    pub(super) async fn close_owned_session(
        &self,
        session_id: &str,
        owner: &super::owner_scope::SharedOwner,
    ) {
        let entry = {
            let mut entries = self.entries.lock().await;
            let owned = entries.get(session_id).is_some_and(|entry| {
                entry
                    .owner_registration
                    .as_ref()
                    .is_some_and(|registration| registration.belongs_to(owner))
            });
            owned.then(|| entries.remove(session_id)).flatten()
        };
        if let Some(mut entry) = entry {
            entry.kill_handle.kill();
            // BUG-363: nothing to return here; report the failure.
            if let Err(error) = entry.child.kill().await {
                tracing::error!(session = session_id, %error, "cannot kill exec session");
            }
        }
    }

    /// TOOL-022 P4: record the exit status of a session whose entry was
    /// removed (reaper / eviction), so a racing poller can recover it.
    pub async fn stash_exit(&self, session_id: &str, status: super::exec_exit::ExecExit) {
        let mut exited = self.exited.lock().await;
        // Lazy prune: drop statuses older than the retention window.
        let now = Instant::now();
        exited.retain(|_, (_, at)| now.duration_since(*at) <= EXIT_STATUS_RETENTION);
        exited.insert(session_id.to_string(), (status, now));
    }

    /// TOOL-022 P4: recover the exit status of a session whose entry is
    /// gone (reaped by the reaper after our `get_output_handles`).
    pub async fn recover_exit(&self, session_id: &str) -> Option<super::exec_exit::ExecExit> {
        let exited = self.exited.lock().await;
        exited
            .get(session_id)
            .filter(|(_, at)| Instant::now().duration_since(*at) <= EXIT_STATUS_RETENTION)
            .map(|(status, _)| status.clone())
    }

    /// Insert a new process entry. Returns the session_id.
    pub async fn insert(&self, session_id: String, entry: ProcessEntry) {
        let mut entries = self.entries.lock().await;
        if entry
            .owner_registration
            .as_ref()
            .is_some_and(super::owner_scope::ProcessRegistration::stopped)
        {
            return;
        }
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
