//! TOOL-022 P2 — deterministic exec-stdin detector + per-agent-session
//! callback registry.
//!
//! Fires when a pipe/PTY child is ALIVE and the session has been quiet
//! for >= `EXEC_STDIN_QUIET_THRESHOLD_SECS` (a pure timing fact — NO
//! output-content inspection). The request carries only the exec
//! session id, the command display, and the quiet seconds: nothing
//! derived from output content crosses the wire.
//!
//! BUG-171: the detector also observes the END of the prompt condition
//! and pushes a CLEAR (`None`) through the same callback — within one
//! detector tick (~2s) when the child exits, the exec session leaves
//! the store, or output resumes (quiet < threshold) after a fire. A
//! non-exit clear resets the per-exec-session cooldown so a fresh
//! quiet period can re-fire; the 30s cooldown still bounds re-fires
//! within one continuous quiet period. The owning agent session
//! (agent-loop / napi agent loop) registers a callback that stores the
//! request on its `BackgroundSession` (or clears it); the TUI learns of
//! both transitions by the PUSH StreamChunks `set_exec_stdin_request`
//! emits (exec-stdin performs NO status flip of its own).

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
///
/// BUG-171: the payload is `Option<ExecStdinRequest>` — `Some` is a
/// detector fire (store the request), `None` is a detector clear (the
/// prompt condition ended: child exit, store removal, or output
/// resumption after a fire). The sessions layer routes both through
/// `BackgroundSession::set_exec_stdin_request`, the sole emission
/// point for the push StreamChunks.
pub type ExecStdinRequestCallback = Arc<dyn Fn(Option<ExecStdinRequest>) + Send + Sync>;

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
pub fn set_exec_stdin_request_callback(
    session_id: Uuid,
    callback: Option<ExecStdinRequestCallback>,
) {
    EXEC_STDIN_REQUEST_CALLBACKS.set(session_id, callback);
}

/// Emit an exec-stdin event for a specific agent session.
///
/// `Some(request)` = detector fire (store the request); `None` =
/// detector clear (BUG-171 — the prompt condition ended after a fire:
/// child exit, store removal, or output resumption). If a callback is
/// registered it is invoked with the payload; if not, this is a no-op.
/// Called from the per-exec-session detector task.
pub fn emit_exec_stdin_request(agent_session_id: Uuid, request: Option<ExecStdinRequest>) {
    EXEC_STDIN_REQUEST_CALLBACKS.with(&agent_session_id, |callback| {
        callback(request);
    });
}

/// Cooldown tracker — per-exec-session last-fire timestamps. The
/// detector task is the sole writer; the read is a cheap `saturating_sub`.
type CooldownMap = Lazy<std::sync::RwLock<std::collections::HashMap<String, u64>>>;
static LAST_FIRE_MS: CooldownMap =
    Lazy::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));

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
    LAST_FIRE_MS.read().ok().is_none_or(|last| {
        last.get(exec_id)
            .is_none_or(|&fired| now.saturating_sub(fired) >= EXEC_STDIN_COOLDOWN_SECS * 1_000)
    })
}

/// Record a fire for `exec_id` (no-op when the lock is poisoned).
fn record_fire(exec_id: &str) {
    if let Ok(mut last) = LAST_FIRE_MS.write() {
        last.insert(exec_id.to_string(), now_ms());
    }
}

/// BUG-171 rule [5]: reset the per-exec-session re-fire cooldown by
/// removing the last-fire record, so the next quiet window can fire
/// IMMEDIATELY without waiting out the previous fire's 30s window.
/// Called on non-exit clears (output resumption): the command resumed,
/// so the previous fire's cooldown no longer guards a fresh prompt.
/// The 30s window still applies within ONE continuous quiet period —
/// a non-exit clear never happens while the session stays quiet, so
/// the cooldown entry only survives continuous quiet.
fn reset_cooldown(exec_id: &str) {
    if let Ok(mut last) = LAST_FIRE_MS.write() {
        last.remove(exec_id);
    }
}

/// Spawn the per-exec-session detector task.
///
/// ~2s cadence (matches the reaper). Each tick:
/// - session gone from the store → clear (if fired), stop;
/// - child exited → clear (if fired), stop;
/// - quiet time unknown → clear (if fired), stop;
/// - quiet < threshold → clear + reset cooldown (if fired), keep
///   watching (BUG-171 rule [4]c / [5]);
/// - cooldown not elapsed → skip;
/// - otherwise build + emit the request, record the fire, and mark
///   fired so the exit paths know a clear is owed.
///
/// BUG-171 rule [4]: once the detector has fired for this exec
/// session, EVERY way the prompt condition ends — (a) child exit,
/// (b) store removal, (c) output resumption — pushes a clear
/// (`None`) to the agent-session callback within one tick, flowing
/// through `set_exec_stdin_request(None)` → `ExecStdinRequestCleared`.
/// Before the first fire nothing is ever cleared (nothing to clear),
/// so the never-fired path stays silent.
pub fn spawn_exec_stdin_detector(
    agent_session_id: Uuid,
    exec_session_id: String,
    command_display: String,
) {
    tokio::spawn(async move {
        // True after this exec session has fired at least once — the
        // gate for the clear (BUG-171 rule [4]: "If the detector had
        // not fired yet, no clear is emitted").
        let mut fired = false;
        loop {
            tokio::time::sleep(DETECTOR_TICK).await;
            let store = global_store();

            if !store.contains(&exec_session_id).await {
                // (b) session removed from the store (reaper or
                // `close`) — the clear is owed exactly once.
                if fired {
                    emit_exec_stdin_request(agent_session_id, None);
                }
                break;
            }
            // `Some(Some(_))` = child exited (the reaper removes the
            // store entry on its next tick); `None` = session already
            // removed (between the `contains` check above and here).
            match store.try_wait(&exec_session_id).await {
                Some(None) => {
                    // Still running, keep watching.
                }
                _ => {
                    // (a) child exited (or the entry vanished) — clear,
                    // then stop.
                    if fired {
                        emit_exec_stdin_request(agent_session_id, None);
                    }
                    break;
                }
            }
            let Some(quiet) = store.quiet_secs(&exec_session_id).await else {
                // Entry gone between checks — same as (b).
                if fired {
                    emit_exec_stdin_request(agent_session_id, None);
                }
                break;
            };
            if quiet < EXEC_STDIN_QUIET_THRESHOLD_SECS {
                if fired {
                    // (c) output resumed after a fire — clear within
                    // this tick and RESET the cooldown (rule [5]) so
                    // the next quiet window can re-fire without waiting
                    // out the previous fire's 30s window. Keep
                    // watching: the detector is NOT done with this
                    // session.
                    reset_cooldown(&exec_session_id);
                    emit_exec_stdin_request(agent_session_id, None);
                    fired = false;
                }
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
            fired = true;
            emit_exec_stdin_request(agent_session_id, Some(request));
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
        // Should not panic (fire AND clear are both no-ops without a callback).
        emit_exec_stdin_request(session_id, Some(request));
        emit_exec_stdin_request(session_id, None);
    }

    #[test]
    fn test_emit_with_callback_and_clear() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_exec_stdin_request_callback(
            session_id,
            Some(Arc::new(move |req: Option<ExecStdinRequest>| {
                captured_clone.lock().unwrap().push(req);
            })),
        );

        let request = ExecStdinRequest {
            exec_session_id: "exec-1".to_string(),
            command: "git commit".to_string(),
            quiet_seconds: 5,
            ts_ms: 1234,
        };
        emit_exec_stdin_request(session_id, Some(request.clone()));
        emit_exec_stdin_request(session_id, None); // BUG-171 clear
        set_exec_stdin_request_callback(session_id, None);
        emit_exec_stdin_request(session_id, Some(request.clone()));

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 2, "fire + clear, then no-op after unregister");
        assert_eq!(events[0], Some(request));
        assert_eq!(events[1], None, "the clear must arrive as None");
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
            Some(Arc::new(move |req: Option<ExecStdinRequest>| {
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
        // Emitted for b (no callback): no-op (fire AND clear). Emitted for a: captured.
        emit_exec_stdin_request(b, Some(request.clone()));
        emit_exec_stdin_request(b, None);
        emit_exec_stdin_request(a, Some(request));

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

    #[test]
    fn test_reset_cooldown_allows_immediate_refire() {
        let _guard = TEST_LOCK.lock().unwrap();
        // A fire 10s ago is still inside the 30s window…
        let now = now_ms();
        {
            let mut last = LAST_FIRE_MS.write().unwrap();
            last.insert("exec-reset".to_string(), now.saturating_sub(10 * 1000));
        }
        assert!(
            !cooldown_elapsed("exec-reset"),
            "cooldown must still apply after a recent fire"
        );
        // …but a non-exit clear (output resumption) resets it, so a
        // fresh quiet period can fire immediately (BUG-171 rule [5]).
        reset_cooldown("exec-reset");
        assert!(
            cooldown_elapsed("exec-reset"),
            "a non-exit clear must reset the re-fire cooldown"
        );
        // Cleanup.
        let mut last = LAST_FIRE_MS.write().unwrap();
        last.clear();
    }
}
