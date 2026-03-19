//! Session-scoped loop store — SCHED-011
//!
//! In-memory store for `/loop` entries. These are ephemeral (not persisted),
//! scoped to the session that created them, and evaluated by the scheduler
//! engine on each 30-second tick. When due, the prompt is sent directly
//! to the originating session via `session.send_input()`.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;
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

impl LoopEntry {
    /// Check if this loop's interval has elapsed and it should fire.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        if now >= self.expires_at {
            return false;
        }
        let reference = self.last_run_at.unwrap_or(self.created_at);
        let elapsed = now - reference;
        let interval = Duration::seconds(self.interval_seconds as i64);
        elapsed >= interval
    }
}

/// Global loop store — all session-scoped loops across all sessions.
pub struct LoopStore {
    /// loop_id → LoopEntry
    entries: RwLock<HashMap<String, LoopEntry>>,
}

static LOOP_STORE: std::sync::OnceLock<LoopStore> = std::sync::OnceLock::new();

impl LoopStore {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global singleton.
    pub fn instance() -> &'static LoopStore {
        LOOP_STORE.get_or_init(LoopStore::new)
    }

    /// Register a new loop entry.
    pub async fn register(&self, entry: LoopEntry) {
        info!(
            "Loop registered: id={}, session={}, prompt='{}', interval={}s",
            entry.id, entry.session_id, entry.prompt, entry.interval_seconds
        );
        self.entries.write().await.insert(entry.id.clone(), entry);
    }

    /// Cancel a loop by ID. Returns true if it existed.
    pub async fn cancel(&self, loop_id: &str) -> bool {
        let removed = self.entries.write().await.remove(loop_id).is_some();
        if removed {
            info!("Loop cancelled: id={}", loop_id);
        }
        removed
    }

    /// List all loops for a specific session.
    pub async fn list_for_session(&self, session_id: Uuid) -> Vec<LoopEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Get all loops that are due to fire, across all sessions.
    /// Only returns entries whose session is currently idle.
    pub async fn get_due(&self) -> Vec<LoopEntry> {
        let now = Utc::now();
        let entries = self.entries.read().await;

        let mut due = Vec::new();
        for entry in entries.values() {
            if entry.is_due(now) {
                due.push(entry.clone());
            }
        }
        due
    }

    /// Mark a loop as just executed.
    pub async fn mark_executed(&self, loop_id: &str) {
        if let Some(entry) = self.entries.write().await.get_mut(loop_id) {
            entry.last_run_at = Some(Utc::now());
        }
    }

    /// Purge expired entries. Returns the count removed.
    pub async fn purge_expired(&self) -> usize {
        let now = Utc::now();
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, e| now < e.expires_at);
        let purged = before - entries.len();
        if purged > 0 {
            info!("Purged {} expired loop(s)", purged);
        }
        purged
    }

    /// Remove all loops for a session (called on session destroy).
    pub async fn remove_for_session(&self, session_id: Uuid) -> usize {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, e| e.session_id != session_id);
        let removed = before - entries.len();
        if removed > 0 {
            info!(
                "Removed {} loop(s) for destroyed session {}",
                removed, session_id
            );
        }
        removed
    }

    /// Check if the store has any entries at all (fast path for engine tick).
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_entry(id: &str, session: Uuid, interval_sec: u32) -> LoopEntry {
        let now = Utc::now();
        LoopEntry {
            id: id.to_string(),
            session_id: session,
            prompt: format!("check {}", id),
            interval_seconds: interval_sec,
            created_at: now,
            expires_at: now + Duration::days(3),
            last_run_at: None,
        }
    }

    #[test]
    fn test_is_due_never_run() {
        let mut entry = make_entry("a", Uuid::new_v4(), 300);
        // Created 6 minutes ago, never run, 5-minute (300s) interval → due
        entry.created_at = Utc::now() - Duration::minutes(6);
        assert!(entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_not_yet() {
        let entry = make_entry("b", Uuid::new_v4(), 300);
        // Just created → not due
        assert!(!entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_after_execution() {
        let mut entry = make_entry("c", Uuid::new_v4(), 300);
        entry.created_at = Utc::now() - Duration::minutes(20);
        // Last ran 6 minutes ago → due (interval is 300s = 5 min)
        entry.last_run_at = Some(Utc::now() - Duration::minutes(6));
        assert!(entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_recently_executed() {
        let mut entry = make_entry("d", Uuid::new_v4(), 300);
        entry.created_at = Utc::now() - Duration::minutes(20);
        // Last ran 2 minutes ago → not due (interval is 300s = 5 min)
        entry.last_run_at = Some(Utc::now() - Duration::minutes(2));
        assert!(!entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_sub_minute_interval() {
        let mut entry = make_entry("f", Uuid::new_v4(), 5);
        // Created 10 seconds ago, 5-second interval → due
        entry.created_at = Utc::now() - Duration::seconds(10);
        assert!(entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_sub_minute_not_yet() {
        let mut entry = make_entry("g", Uuid::new_v4(), 30);
        // Created 10 seconds ago, 30-second interval → not due
        entry.created_at = Utc::now() - Duration::seconds(10);
        assert!(!entry.is_due(Utc::now()));
    }

    #[test]
    fn test_is_due_expired() {
        let mut entry = make_entry("e", Uuid::new_v4(), 300);
        entry.created_at = Utc::now() - Duration::days(4);
        entry.expires_at = Utc::now() - Duration::hours(1);
        assert!(!entry.is_due(Utc::now()));
    }

    #[tokio::test]
    async fn test_register_and_list() {
        let store = LoopStore { entries: RwLock::new(HashMap::new()) };
        let sid = Uuid::new_v4();
        let other_sid = Uuid::new_v4();

        store.register(make_entry("x1", sid, 300)).await;
        store.register(make_entry("x2", sid, 600)).await;
        store.register(make_entry("x3", other_sid, 900)).await;

        let for_sid = store.list_for_session(sid).await;
        assert_eq!(for_sid.len(), 2);

        let for_other = store.list_for_session(other_sid).await;
        assert_eq!(for_other.len(), 1);
    }

    #[tokio::test]
    async fn test_cancel() {
        let store = LoopStore { entries: RwLock::new(HashMap::new()) };
        store.register(make_entry("y1", Uuid::new_v4(), 300)).await;

        assert!(store.cancel("y1").await);
        assert!(!store.cancel("y1").await); // Already gone
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn test_purge_expired() {
        let store = LoopStore { entries: RwLock::new(HashMap::new()) };
        let mut expired = make_entry("z1", Uuid::new_v4(), 300);
        expired.expires_at = Utc::now() - Duration::hours(1);
        store.register(expired).await;
        store.register(make_entry("z2", Uuid::new_v4(), 300)).await;

        let purged = store.purge_expired().await;
        assert_eq!(purged, 1);
        assert_eq!(store.entries.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_for_session() {
        let store = LoopStore { entries: RwLock::new(HashMap::new()) };
        let sid = Uuid::new_v4();
        store.register(make_entry("w1", sid, 300)).await;
        store.register(make_entry("w2", sid, 600)).await;
        store.register(make_entry("w3", Uuid::new_v4(), 900)).await;

        let removed = store.remove_for_session(sid).await;
        assert_eq!(removed, 2);
        assert_eq!(store.entries.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_executed() {
        let store = LoopStore { entries: RwLock::new(HashMap::new()) };
        let mut entry = make_entry("m1", Uuid::new_v4(), 300);
        entry.created_at = Utc::now() - Duration::minutes(10);
        store.register(entry).await;

        // Should be due
        assert_eq!(store.get_due().await.len(), 1);

        // Mark executed
        store.mark_executed("m1").await;

        // Should no longer be due
        assert_eq!(store.get_due().await.len(), 0);
    }
}
