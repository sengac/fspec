//! ProcessStore LRU eviction (split from `process_store.rs`, REFAC-042).

use std::time::Instant;

use super::process_store::ProcessStore;
use super::LRU_PROTECT_COUNT;
use super::MAX_UNIFIED_EXEC_PROCESSES;

impl ProcessStore {
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
                // BUG-363: a failed check is reported; the session is then
                // treated as live (never evicted as if it had exited).
                let has_exited = match e.child.try_wait() {
                    Ok(status) => status.is_some(),
                    Err(error) => {
                        tracing::error!(session = %id, %error, "cannot check exec session");
                        false
                    }
                };
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
                if let Err(error) = e.child.kill().await {
                    tracing::error!(session = %id, %error, "cannot kill evicted exec session");
                }
            }
        }

        victim_id
    }
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
