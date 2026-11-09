//! ProcessStore per-session handle accessors (split from
//! `process_store.rs`, REFAC-042).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify};

use super::process_store::{now_micros, ChildHandle, ProcessStore};
use super::types::quiet_secs_since;

impl ProcessStore {
    /// Get the stdin sender for a session.
    pub async fn get_stdin_tx(&self, session_id: &str) -> Option<super::stdin_write::StdinSender> {
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

    /// Check if a process has exited (non-blocking). `Ok(None)` when the
    /// session does not exist, `Ok(Some(None))` while it runs; a failed
    /// status check is an error naming the session (BUG-363).
    pub async fn try_wait(
        &self,
        session_id: &str,
    ) -> Result<Option<Option<super::exec_exit::ExecExit>>, String> {
        let mut entries = self.entries.lock().await;
        let Some(entry) = entries.get_mut(session_id) else {
            return Ok(None);
        };
        let status = entry
            .child
            .try_wait()
            .map_err(|error| format!("cannot check exec session {session_id}: {error}"))?;
        // BUG-368: the real end state (the PTY child's, not the anchor's).
        Ok(Some(status.map(|status| {
            super::exec_exit::ExecExit::resolve(status, entry.pty_exit.as_ref())
        })))
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
        let entry = self.entries.lock().await.remove(session_id);
        match entry {
            Some(mut entry) => {
                // TOOL-022 P4: full process tree (PTY child + anchor /
                // process group), not just the direct child.
                entry.kill_handle.kill();
                entry
                    .child
                    .kill()
                    .await
                    .map_err(|error| format!("cannot kill exec session {session_id}: {error}"))?;
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
