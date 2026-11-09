//! UnifiedExecTool — rig::tool::Tool implementation for process execution.
//!
//! Dispatches to run/write/poll/list/close actions based on the `action` parameter.
//! Types live in `types.rs`; spawning, output, and reaper in their own modules.

use super::exec_stdin::spawn_exec_stdin_detector;
use super::output::{collect_output_until_deadline_interruptible, truncate_output_str};
use super::owner_scope::{capture_owner, ProcessRegistration};
use super::process_store::{global_store, ProcessEntry};
use super::reaper::{generate_session_id, spawn_reaper};
use super::spawning::{spawn_pipe_process, spawn_pty_process};
use super::types::{
    strip_stderr_markers, ExecCommand, SessionListEntry, UnifiedExecArgs, UnifiedExecResult,
    STILL_RUNNING_STEERING,
};
use super::{clamp_poll_yield_time, clamp_yield_time, DEFAULT_YIELD_TIME_MS};
use crate::blocklist::check_bash_command;
use crate::error::ToolError;
use crate::facade::get_effective_cwd;
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

// ============================================================================
// UnifiedExecTool
// ============================================================================

/// The unified exec tool.
pub struct UnifiedExecTool {
    session_id: Uuid,
    /// TOOL-022 P4: optional abort check (the BashTool delegation passes
    /// its per-session abort flag). When set, the `run` output window
    /// observes it every ~50ms and exits early.
    abort_check: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl UnifiedExecTool {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            abort_check: None,
        }
    }

    /// TOOL-022 P4: attach the abort check the `run` output window
    /// consults every ~50ms (the BashTool delegation's per-session
    /// abort flag).
    pub fn with_abort_check(mut self, f: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        self.abort_check = Some(Arc::new(f));
        self
    }

    /// The interrupt closure for the output drain (no-op when no abort
    /// check is attached). Returns a shared Arc (owned by the drain's
    /// lifetime) so the `Send` future bound is satisfied.
    fn interrupt(&self) -> Arc<dyn Fn() -> bool + Send + Sync> {
        match &self.abort_check {
            Some(check) => Arc::clone(check),
            None => Arc::new(|| false),
        }
    }

    /// Determine the effective working directory.
    fn resolve_workdir(&self, explicit: Option<&str>) -> Result<Option<String>, ToolError> {
        // Session isolation takes precedence (BUG-404: a failed session
        // lookup refuses the command).
        let isolated = get_effective_cwd(self.session_id)
            .map_err(|message| crate::facade::isolation_lookup_error("unified_exec", message))?;
        Ok(isolated
            .map(|cwd| cwd.to_string_lossy().to_string())
            .or_else(|| explicit.map(String::from)))
    }
}

// ============================================================================
// Tool Implementation
// ============================================================================

impl Tool for UnifiedExecTool {
    const NAME: &'static str = "unified_exec";

    type Error = ToolError;
    type Args = UnifiedExecArgs;
    type Output = UnifiedExecResult;

    fn name(&self) -> String {
        "unified_exec".to_string()
    }

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "unified_exec".to_string(),
            description: "Execute commands with session management. Supports one-shot execution and interactive PTY sessions with yield-and-resume.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["run", "write", "poll", "list", "close"],
                        "description": "Action to perform. Inferred from other params when omitted."
                    },
                    "command": {
                        "anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}],
                        "description": "Command as shell string or argv array (for run action)."
                    },
                    "input": {
                        "type": "string",
                        "description": "stdin content to send (for write action)."
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID (for write/poll/close actions)."
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Working directory for the command."
                    },
                    "tty": {
                        "type": "boolean",
                        "description": "Allocate a PTY for the process (default: false)."
                    },
                    "yield_time_ms": {
                        "type": "integer",
                        "description": "Time to wait for output before yielding (ms). Default: 10000."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let params = &args.0;

        // Infer action from parameters if not explicitly provided
        let action = if let Some(a) = params.get("action").and_then(Value::as_str) {
            a.to_string()
        } else if params.get("command").is_some() {
            "run".to_string()
        } else if params.get("input").is_some() {
            "write".to_string()
        } else if params.get("session_id").is_some() {
            "poll".to_string()
        } else {
            "list".to_string()
        };

        match action.as_str() {
            "run" => self.handle_run(params).await,
            "write" => self.handle_write(params).await,
            "poll" => self.handle_poll(params).await,
            "list" => self.handle_list().await,
            "close" => self.handle_close(params).await,
            _ => Err(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Unknown action: {action}. Valid: run, write, poll, list, close"),
            }),
        }
    }
}

// ============================================================================
// Shared Session Polling
// ============================================================================

/// Poll a session for output, check exit status, and return a result.
///
/// Shared by both `handle_write` (after sending input) and `handle_poll`.
/// Implements the yield-and-resume pattern: collect output for `yield_time_ms`,
/// then check if the process exited. If exited, remove from store and return
/// `exit_code`. If still running, return `session_id`.
///
/// TOOL-022 P4: `pub` so the BashTool delegation loop
/// (`crate::bash_session`) can poll with its own (abort-cadence) yield
/// window without going through the LLM-facing action clamps.
pub async fn poll_session(
    session_id: &str,
    yield_time_ms: u64,
) -> Result<UnifiedExecResult, ToolError> {
    let no_interrupt: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(|| false);
    poll_session_interruptible(session_id, yield_time_ms, &no_interrupt).await
}

/// TOOL-022 P4: like [`poll_session`], but the output drain consults
/// `interrupt` every ~50ms and stops early when it returns `true`
/// (the BashTool delegation loop's abort flag). Early-stopped polls
/// return the partial output captured so far + the (still) running
/// session id — the caller continues polling.
pub async fn poll_session_interruptible(
    session_id: &str,
    yield_time_ms: u64,
    interrupt: &Arc<dyn Fn() -> bool + Send + Sync>,
) -> Result<UnifiedExecResult, ToolError> {
    let store = global_store();

    let (output_buffer, output_notify) =
        store
            .get_output_handles(session_id)
            .await
            .ok_or(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Unknown session: {session_id}"),
            })?;

    let start = Instant::now();
    let output_bytes = collect_output_until_deadline_interruptible(
        &output_buffer,
        &output_notify,
        yield_time_ms,
        interrupt,
    )
    .await;
    let output = String::from_utf8_lossy(&output_bytes).into_owned();
    let clean = strip_stderr_markers(&output);
    let raw = output_bytes.clone();
    let wall_time = start.elapsed().as_secs_f64();

    // BUG-363: a failed status check is a tool error, never "running".
    let state = store
        .try_wait(session_id)
        .await
        .map_err(|message| ToolError::Execution {
            tool: "unified_exec",
            message,
        })?;
    match state {
        Some(Some(exit)) => {
            // Process exited — clean up and report how it ended (BUG-368).
            store.remove(session_id).await;
            Ok(UnifiedExecResult {
                output: Some(truncate_output_str(&clean)),
                wall_time_seconds: Some(wall_time),
                raw_output: Some(raw),
                ..UnifiedExecResult::exited(&exit)
            })
        }
        Some(None) => {
            // Process still running (BUG-368: no guessed quiet time).
            let quiet_secs = store.quiet_secs(session_id).await;
            let output_str = truncate_output_str(&clean);
            Ok(UnifiedExecResult {
                exit_code: None,
                signal: None,
                exit_status_unavailable: None,
                session_id: Some(session_id.to_string()),
                output: Some(still_running_output(&output_str)),
                wall_time_seconds: Some(wall_time),
                sessions: None,
                error: None,
                quiet_seconds: quiet_secs,
                raw_output: Some(raw),
            })
        }
        None => {
            // Session no longer in store — the reaper removed it after
            // the process exited. `recover_exit` retrieves the status the
            // reaper stashed; without one the status is reported as
            // unavailable, never as a -1 code (BUG-368).
            let exit = store
                .recover_exit(session_id)
                .await
                .unwrap_or_else(crate::unified_exec::ExecExit::unavailable);
            Ok(UnifiedExecResult {
                output: Some(truncate_output_str(&clean)),
                wall_time_seconds: Some(wall_time),
                raw_output: Some(raw),
                ..UnifiedExecResult::exited(&exit)
            })
        }
    }
}

/// TOOL-022 P1: append the fixed steering line to a still-running result's
/// output. Deterministic — no output-content inspection (vtcode's
/// `next_action_hint` analogue).
fn still_running_output(output: &str) -> String {
    if output.is_empty() {
        format!("{STILL_RUNNING_STEERING}\n")
    } else {
        format!("{output}\n\n{STILL_RUNNING_STEERING}")
    }
}

mod handlers;
