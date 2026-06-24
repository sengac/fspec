//! Session-scoped loop store — lifted from codelet-napi (RPC-059).
//!
//! In-memory store for `/loop` entries. These are ephemeral (not persisted),
//! scoped to the session that created them.
//!
//! Each entry spawns its own tokio task that sleeps for exactly the
//! configured interval and fires a callback. The LoopStore is an active
//! task manager — not a passive data store polled by the engine tick.
//!
//! This module has NO NAPI dependency — it lives in codelet-core so both
//! the NAPI binary and the pure-Rust codelet-sessions handle impl can
//! share the same process-global singleton.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::info;
use uuid::Uuid;

/// A single session-scoped loop entry.
#[derive(Debug, Clone)]
pub struct LoopEntry {
    pub id: String,
    pub session_id: Uuid,
    pub prompt: String,
    pub interval_seconds: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
}

/// Idle-check callback type: given a session UUID, returns whether it is idle.
pub type IdleCheckFn =
    Arc<dyn Fn(Uuid) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync + 'static>;

/// Shared inner state for LoopStore.
///
/// Extracted into its own struct so spawned tasks can hold an `Arc<Inner>`
/// reference for self-removal on expiry — without raw pointers.
struct Inner {
    /// loop_id → LoopEntry
    entries: RwLock<HashMap<String, LoopEntry>>,
    /// loop_id → JoinHandle for the spawned task
    handles: RwLock<HashMap<String, JoinHandle<()>>>,
}

/// Global loop store — all session-scoped loops across all sessions.
///
/// Active task manager: each entry is paired with a JoinHandle for its
/// spawned tokio task.
pub struct LoopStore {
    inner: Arc<Inner>,
}

static LOOP_STORE: std::sync::OnceLock<LoopStore> = std::sync::OnceLock::new();

impl LoopStore {
    fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                entries: RwLock::new(HashMap::new()),
                handles: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create a non-singleton LoopStore for testing.
    pub fn new_local() -> Self {
        Self::new()
    }

    /// Get the global singleton.
    pub fn instance() -> &'static LoopStore {
        LOOP_STORE.get_or_init(LoopStore::new)
    }

    /// Cancel a loop by ID. Returns true if it existed.
    ///
    /// Also aborts the spawned JoinHandle if present.
    pub async fn cancel(&self, loop_id: &str) -> bool {
        let removed = self.inner.entries.write().await.remove(loop_id).is_some();
        if removed {
            if let Some(handle) = self.inner.handles.write().await.remove(loop_id) {
                handle.abort();
            }
            info!("Loop cancelled: id={}", loop_id);
        }
        removed
    }

    /// List all loops for a specific session.
    pub async fn list_for_session(&self, session_id: Uuid) -> Vec<LoopEntry> {
        self.inner
            .entries
            .read()
            .await
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Remove all loops for a session (called on session destroy).
    ///
    /// Also aborts all JoinHandles for that session.
    pub async fn remove_for_session(&self, session_id: Uuid) -> usize {
        let mut entries = self.inner.entries.write().await;
        let removed_ids: Vec<String> = entries
            .iter()
            .filter(|(_, e)| e.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &removed_ids {
            entries.remove(id);
        }
        drop(entries);

        if !removed_ids.is_empty() {
            let mut handles = self.inner.handles.write().await;
            for id in &removed_ids {
                if let Some(handle) = handles.remove(id) {
                    handle.abort();
                }
            }
        }

        let removed = removed_ids.len();
        if removed > 0 {
            info!(
                "Removed {} loop(s) for destroyed session {}",
                removed, session_id
            );
        }
        removed
    }

    /// Check if the store has any entries at all (fast path).
    pub async fn is_empty(&self) -> bool {
        self.inner.entries.read().await.is_empty()
    }

    /// Register a loop entry and spawn a tokio task that fires `on_fire`
    /// every `entry.interval_seconds`. The task auto-terminates on expiry
    /// and self-removes from the store.
    pub async fn register_with_task(
        &self,
        entry: LoopEntry,
        on_fire: Arc<dyn Fn(String) + Send + Sync + 'static>,
    ) {
        let idle_check: IdleCheckFn = Arc::new(|_session_id: Uuid| Box::pin(async { true }));
        self.register_with_task_and_idle_check(entry, on_fire, idle_check)
            .await;
    }

    /// Like `register_with_task` but validates interval >= 1 second first.
    /// Returns Err if interval is < 1 second.
    pub async fn try_register_with_task(
        &self,
        entry: LoopEntry,
        on_fire: Arc<dyn Fn(String) + Send + Sync + 'static>,
    ) -> Result<(), String> {
        if entry.interval_seconds < 1 {
            return Err(format!(
                "Minimum loop interval is 1 second, got {}",
                entry.interval_seconds
            ));
        }
        self.register_with_task(entry, on_fire).await;
        Ok(())
    }

    /// Like `register_with_task_and_idle_check` but validates interval >= 1
    /// second first. Returns Err if interval is < 1 second.
    ///
    /// This is the recommended entry point for production callers — it
    /// enforces the minimum-1s-interval rule before spawning any task.
    pub async fn try_register_with_task_and_idle_check(
        &self,
        entry: LoopEntry,
        on_fire: Arc<dyn Fn(String) + Send + Sync + 'static>,
        idle_check: IdleCheckFn,
    ) -> Result<(), String> {
        if entry.interval_seconds < 1 {
            return Err(format!(
                "Minimum loop interval is 1 second, got {}",
                entry.interval_seconds
            ));
        }
        self.register_with_task_and_idle_check(entry, on_fire, idle_check)
            .await;
        Ok(())
    }

    /// Register a loop entry with an idle-check gate. The spawned task
    /// calls `idle_check(session_id)` before each firing; if the session
    /// is not idle, the task skips that tick and retries after the next
    /// interval.
    pub async fn register_with_task_and_idle_check(
        &self,
        entry: LoopEntry,
        on_fire: Arc<dyn Fn(String) + Send + Sync + 'static>,
        idle_check: IdleCheckFn,
    ) {
        let loop_id = entry.id.clone();
        let session_id = entry.session_id;
        let prompt = entry.prompt.clone();
        let interval_secs = entry.interval_seconds;
        let expires_at = entry.expires_at;

        info!(
            "Loop registered (task): id={}, session={}, prompt='{}', interval={}s",
            loop_id, session_id, prompt, interval_secs
        );

        // Store the entry
        self.inner
            .entries
            .write()
            .await
            .insert(loop_id.clone(), entry);

        // Clone the Arc<Inner> so the spawned task can self-remove on expiry
        let inner = Arc::clone(&self.inner);
        let loop_id_for_handle = loop_id.clone();

        let handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(interval_secs as u64);

            loop {
                tokio::time::sleep(interval).await;

                // Check expiry after waking
                if Utc::now() >= expires_at {
                    info!("Loop {} expired, self-terminating", loop_id);
                    inner.entries.write().await.remove(&loop_id);
                    inner.handles.write().await.remove(&loop_id);
                    return;
                }

                // Check if session is idle
                if !idle_check(session_id).await {
                    info!(
                        "Loop {}: session {} is busy, skipping this tick",
                        loop_id, session_id
                    );
                    continue;
                }

                // Fire the callback
                on_fire(prompt.clone());

                // Update last_run_at so loop_list() reports accurate timing
                if let Some(entry) = inner.entries.write().await.get_mut(&loop_id) {
                    entry.last_run_at = Some(Utc::now());
                }
            }
        });

        self.inner
            .handles
            .write()
            .await
            .insert(loop_id_for_handle, handle);
    }

    /// Check if a loop entry has an active (non-finished) spawned task.
    pub async fn has_active_task(&self, loop_id: &str) -> bool {
        let handles = self.inner.handles.read().await;
        match handles.get(loop_id) {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }
}
