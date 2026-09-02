//! TOOL-022 P2 — deterministic exec-stdin detector + per-agent-session
//! callback registry.
//!
//! Fires when a pipe/PTY child is ALIVE and the session has been quiet
//! for >= `EXEC_STDIN_QUIET_THRESHOLD_SECS` (a pure timing fact — NO
//! output-content inspection). The request carries only the exec
//! session id, the command display, and the quiet seconds: nothing
//! derived from output content crosses the wire.
//!
//! Mirrors `tool_progress.rs`: a `SessionRegistry` of per-agent-session
//! callbacks. The owning agent session (agent-loop / napi agent loop)
//! registers a callback that stores the request on its
//! `BackgroundSession`; the TUI learns of it by probing
//! `get_exec_stdin_request` (there is NO status flip for exec-stdin).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session_registry::SessionRegistry;

use super::process_store::global_store;

/// Quiet threshold before the detector may fire (seconds since last
/// output, floored — same clock the P1 `quiet_seconds` uses).
pub const EXEC_STDIN_QUIET_THRESHOLD_SECS: u64 = 3;

/// Per-exec-session cooldown: re-fire at most every 30s.
pub const EXEC_STDIN_COOLDOWN_SECS: u64 = 30;

/// Detector tick cadence (matches `reaper.rs`'s 2s loop).
const DETECTOR_TICK: Duration = Duration::from_secs(2);

/// TOOL-022 P2: wire-portable exec-stdin prompt request. Tools-internal
/// shape — identical fields to the `codelet_rpc_types::ExecStdinRequest`
/// mirror (the sessions crate maps between the two, like `hitl_mapping`
/// does for HITL). No hint/content field: nothing from output content
/// crosses the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecStdinRequest {
    /// The unified_exec session id (NOT the agent session id).
    pub exec_session_id: String,
    /// Command display string (truncated at spawn, already stored per
    /// ProcessEntry).
    pub command: String,
    /// Seconds since last output when the detector fired (floored).
    pub quiet_seconds: u64,
    /// Detector fire time, Unix epoch milliseconds.
    pub ts_ms: u64,
}

/// Per-agent-session callback type.
pub type ExecStdinRequestCallback = Arc<dyn Fn(ExecStdinRequest) + Send + Sync>;

/// Per-agent-session callback storage (BUG-126-style isolation: no
/// global fallback, concurrent sessions never leak requests to each
/// other).
static EXEC_STDIN_REQUEST_CALLBACKS: Lazy<SessionRegistry<ExecStdinRequestCallback>> =
    Lazy::new(SessionRegistry::new);

/// Register or clear the exec-stdin request callback for a specific
/// agent session.
///
/// Call with `Some(callback)` before agent execution starts; call with
/// `None` on cleanup (mirrors `set_tool_progress_callback`).
pub fn set_exec_stdin_request_callback(session_id: Uuid, callback: Option<ExecStdinRequestCallback>) {
    EXEC_STDIN_REQUEST_CALLBACKS.set(session_id, callback);
}

/// Emit an exec-stdin request for a specific agent session.
///
/// If a callback is registered it is invoked with the request; if not,
/// this is a no-op. Called from the per-exec-session detector task.
pub fn emit_exec_stdin_request(agent_session_id: Uuid, request: ExecStdinRequest) {
    EXEC_STDIN_REQUEST_CALLBACKS.with(&agent_session_id, |callback| {
        callback(request);
    });
}

/// Cooldown tracker — per-exec-session last-fire timestamps. The
/// detector task is the sole writer; the read is a cheap `saturating_sub`.
type CooldownMap = Lazy<std::sync::RwLock<std::collections::HashMap<String, u64>>>;
static LAST_FIRE_MS: CooldownMap = Lazy::new(|| {
    std::sync::RwLock::new(std::collections::HashMap::new())
});

/// Current Unix epoch milliseconds (saturates on clock anomalies — no
/// panic path, per workspace lint policy).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

/// True when `exec_id` may fire again (cooldown elapsed or first fire).
/// A poisoned lock degrades to "eligible" — matching the
/// `SessionRegistry` convention of graceful degradation on poisoning.
fn cooldown_elapsed(exec_id: &str) -> bool {
    let now = now_ms();
    LAST_FIRE_MS
        .read()
        .ok()
        .is_none_or(|last| {
            last.get(exec_id).is_none_or(|&fired| {
                now.saturating_sub(fired) >= EXEC_STDIN_COOLDOWN_SECS * 1_000
            })
        })
}

/// Record a fire for `exec_id` (no-op when the lock is poisoned).
fn record_fire(exec_id: &str) {
    if let Ok(mut last) = LAST_FIRE_MS.write() {
        last.insert(exec_id.to_string(), now_ms());
    }
}

/// Spawn the per-exec-session detector task.
///
/// ~2s cadence (matches the reaper). Each tick:
/// - session gone from the store → stop;
/// - child exited → stop;
/// - quiet time unknown → stop;
/// - quiet < threshold → wait for the next tick;
/// - cooldown not elapsed → skip;
/// - otherwise build + emit the request and record the fire.
pub fn spawn_exec_stdin_detector(agent_session_id: Uuid, exec_session_id: String, command_display: String) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(DETECTOR_TICK).await;
            let store = global_store();

            if !store.contains(&exec_session_id).await {
                break;
            }
            // `Some(Some(_))` = child exited (the reaper removes the
            // store entry on its next tick); `None` = session already
            // removed (by the reaper or a `close`). Stop watching.
            if !matches!(store.try_wait(&exec_session_id).await, Some(None)) {
                break;
            }
            let Some(quiet) = store.quiet_secs(&exec_session_id).await else {
                break;
            };
            if quiet < EXEC_STDIN_QUIET_THRESHOLD_SECS {
                continue;
            }
            if !cooldown_elapsed(&exec_session_id) {
                continue;
            }
            let request = ExecStdinRequest {
                exec_session_id: exec_session_id.clone(),
                command: command_display.clone(),
                quiet_seconds: quiet,
                ts_ms: now_ms(),
            };
            record_fire(&exec_session_id);
            emit_exec_stdin_request(agent_session_id, request);
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_emit_with_no_callback_is_noop() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();
        set_exec_stdin_request_callback(session_id, None);
        let request = ExecStdinRequest {
            exec_session_id: "exec-1".to_string(),
            command: "cmd".to_string(),
            quiet_seconds: 3,
            ts_ms: 0,
        };
        // Should not panic.
        emit_exec_stdin_request(session_id, request);
    }

    #[test]
    fn test_emit_with_callback_and_clear() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_exec_stdin_request_callback(
            session_id,
            Some(Arc::new(move |req: ExecStdinRequest| {
                captured_clone.lock().unwrap().push(req);
            })),
        );

        let request = ExecStdinRequest {
            exec_session_id: "exec-1".to_string(),
            command: "git commit".to_string(),
            quiet_seconds: 5,
            ts_ms: 1234,
        };
        emit_exec_stdin_request(session_id, request.clone());
        set_exec_stdin_request_callback(session_id, None);
        emit_exec_stdin_request(session_id, request.clone());

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], request);
    }

    #[test]
    fn test_session_isolation() {
        let _guard = TEST_LOCK.lock().unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_exec_stdin_request_callback(
            a,
            Some(Arc::new(move |req: ExecStdinRequest| {
                captured_clone.lock().unwrap().push(req);
            })),
        );
        set_exec_stdin_request_callback(b, None);

        let request = ExecStdinRequest {
            exec_session_id: "exec-x".to_string(),
            command: "cmd".to_string(),
            quiet_seconds: 3,
            ts_ms: 1,
        };
        // Emitted for b (no callback): no-op. Emitted for a: captured.
        emit_exec_stdin_request(b, request.clone());
        emit_exec_stdin_request(a, request);

        assert_eq!(captured.lock().unwrap().len(), 1);
        set_exec_stdin_request_callback(a, None);
    }

    #[test]
    fn test_cooldown_state_machine() {
        let _guard = TEST_LOCK.lock().unwrap();
        // @step Given an agent session detector fired for exec session "exec-cool" 10 seconds ago
        let now = now_ms();
        {
            let mut last = LAST_FIRE_MS.write().unwrap();
            last.insert("exec-cool".to_string(), now.saturating_sub(10 * 1000));
        }
        // @step When the exec session is quiet again while running
        // @step Then the detector does not fire again before the 30 second cooldown elapses
        assert!(!cooldown_elapsed("exec-cool"));
        // @step Given an agent session detector fired for exec session "exec-cool" 60 seconds ago
        {
            let mut last = LAST_FIRE_MS.write().unwrap();
            last.insert("exec-cool".to_string(), now.saturating_sub(60 * 1000));
        }
        // @step When the exec session is quiet again while running
        // @step Then the detector fires again
        assert!(cooldown_elapsed("exec-cool"));
        // Never-fired session: eligible.
        assert!(cooldown_elapsed("exec-never"));
        // Cleanup.
        let mut last = LAST_FIRE_MS.write().unwrap();
        last.clear();
    }
}
