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
use std::time::{Duration, SystemTime};

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
/// registered it is invoked with the payload; if not (BUG-387) the event
/// went nowhere and that is returned as an error naming the session.
pub fn emit_exec_stdin_request(
    agent_session_id: Uuid,
    request: Option<ExecStdinRequest>,
) -> Result<(), String> {
    EXEC_STDIN_REQUEST_CALLBACKS
        .with(&agent_session_id, |callback| callback(request))
        .ok_or_else(|| {
            format!("agent session {agent_session_id} has no exec-stdin request callback")
        })
}

mod cooldown;
use cooldown::*;

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
    let mut observer = EXEC_STDIN_REQUEST_CALLBACKS.get(&agent_session_id);
    let exec_id = exec_session_id.clone();
    tokio::spawn(async move {
        let mut emit = move |request: Option<ExecStdinRequest>| {
            if observer.is_none() {
                observer = EXEC_STDIN_REQUEST_CALLBACKS.get(&agent_session_id);
            }
            match &observer {
                Some(callback) => callback(request),
                // BUG-376: a detector for a session nobody listens to is a
                // wiring fault; report each undeliverable fire or clear.
                None => tracing::error!(
                    %agent_session_id,
                    exec_session_id = %exec_id,
                    fire = request.is_some(),
                    "exec-stdin request has no registered callback for its agent session; the prompt cannot reach the user"
                ),
            }
        };
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
                    emit(None);
                }
                break;
            }
            // `Some(Some(_))` = child exited (the reaper removes the
            // store entry on its next tick); `None` = session already
            // removed (between the `contains` check above and here).
            match store.try_wait(&exec_session_id).await {
                Ok(Some(None)) => {
                    // Still running, keep watching.
                }
                Err(error) => {
                    // BUG-363: report it; the detector cannot watch a
                    // session it cannot check, so it clears and stops.
                    tracing::error!(session = %exec_session_id, %error, "exec-stdin detector cannot check exec session");
                    if fired {
                        emit(None);
                    }
                    break;
                }
                _ => {
                    // (a) child exited (or the entry vanished) — clear,
                    // then stop.
                    if fired {
                        emit(None);
                    }
                    break;
                }
            }
            let Some(quiet) = store.quiet_secs(&exec_session_id).await else {
                // Entry gone between checks — same as (b).
                if fired {
                    emit(None);
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
                    emit(None);
                    fired = false;
                }
                continue;
            }
            if !cooldown_elapsed(&exec_session_id) {
                continue;
            }
            let ts_ms = match epoch_ms(SystemTime::now()) {
                Ok(ts_ms) => ts_ms,
                Err(error) => {
                    // BUG-387: no 0 timestamp; report it and skip this fire.
                    tracing::error!(session = %exec_session_id, %error, "exec-stdin request not raised");
                    continue;
                }
            };
            let request = ExecStdinRequest {
                exec_session_id: exec_session_id.clone(),
                command: command_display.clone(),
                quiet_seconds: quiet,
                ts_ms,
            };
            record_fire(&exec_session_id);
            fired = true;
            emit(Some(request));
        }
    });
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod clock_and_emit_tests;

#[cfg(test)]
mod fault_reporting_tests;
