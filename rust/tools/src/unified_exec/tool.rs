//! UnifiedExecTool — rig::tool::Tool implementation for process execution.
//!
//! Dispatches to run/write/poll/list/close actions based on the `action` parameter.
//! Types live in `types.rs`; spawning, output, and reaper in their own modules.

use super::exec_stdin::spawn_exec_stdin_detector;
use super::output::{collect_output_until_deadline_interruptible, truncate_output_str};
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
    fn resolve_workdir(&self, explicit: Option<&str>) -> Option<String> {
        // Session isolation takes precedence
        if let Some(cwd) = get_effective_cwd(self.session_id) {
            return Some(cwd.to_string_lossy().to_string());
        }
        explicit.map(String::from)
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
// Action Handlers
// ============================================================================

impl UnifiedExecTool {
    /// Handle the `run` action — spawn a process, collect output, return result.
    async fn handle_run(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
        let command_val = params.get("command").ok_or(ToolError::Validation {
            tool: "unified_exec",
            message: "command is required for run action".to_string(),
        })?;
        let command = ExecCommand::from_value(command_val)?;

        let tty = params
            .get("tty")
            .and_then(crate::facade::param_extract::value_as_bool_lenient)
            .unwrap_or(false);
        let yield_time_ms = params
            .get("yield_time_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient)
            .unwrap_or(DEFAULT_YIELD_TIME_MS);
        let yield_time_ms = clamp_yield_time(yield_time_ms);
        let workdir = self.resolve_workdir(params.get("workdir").and_then(Value::as_str));

        // Evict if at capacity
        let store = global_store();
        store.evict_lru_if_full().await;

        // TOOL-022 P4: `skip_blocklist` — the BashTool delegation path has
        // already run the Bash-layer blocklist/pause check; skip the
        // unified_exec internal check to avoid double-prompting.
        let skip_blocklist = params
            .get("skip_blocklist")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !skip_blocklist {
            let check_str = command.blocklist_check_string();
            if let Err(blocked) = check_bash_command(&check_str, self.session_id) {
                return Err(ToolError::Blocked {
                    tool: "unified_exec",
                    message: blocked.to_string(),
                });
            }
        }

        // Spawn the process
        let (mut child, stdin_tx, output_buffer, output_notify, last_output_micros, kill_handle) =
            if tty {
                spawn_pty_process(&command, workdir.as_deref())?
            } else {
                spawn_pipe_process(&command, workdir.as_deref())?
            };

        let start = Instant::now();
        let output = collect_output_until_deadline_interruptible(
            &output_buffer,
            &output_notify,
            yield_time_ms,
            &self.interrupt(),
        )
        .await;

        // Check if process exited
        let exit_status = child.try_wait().map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to check process status: {e}"),
        })?;

        let wall_time = start.elapsed().as_secs_f64();

        let raw = output.clone();
        let output = String::from_utf8_lossy(&raw).into_owned();
        let clean = strip_stderr_markers(&output);
        match exit_status {
            Some(status) => {
                // Process exited — return exit_code (backward-compatible one-shot)
                Ok(UnifiedExecResult {
                    exit_code: Some(status.code().unwrap_or(-1)),
                    session_id: None,
                    output: Some(truncate_output_str(&clean)),
                    wall_time_seconds: Some(wall_time),
                    sessions: None,
                    error: None,
                    quiet_seconds: None,
                    raw_output: Some(raw),
                })
            }
            _ => {
                // Process still running — store and return session_id
                let session_id = generate_session_id();
                let entry = ProcessEntry {
                    child,
                    stdin_tx,
                    output_buffer,
                    output_notify,
                    last_used: Instant::now(),
                    tty,
                    command_display: command.display(),
                    last_output_micros,
                    kill_handle,
                };
                store.insert(session_id.clone(), entry).await;
                spawn_reaper(session_id.clone());
                // TOOL-022 P2: deterministic quiet detector — pushes an
                // ExecStdinRequest to the owning agent session's callback
                // once the child is alive and quiet >= 3s (no status flip).
                spawn_exec_stdin_detector(self.session_id, session_id.clone(), command.display());

                let quiet_secs = store.quiet_secs(&session_id).await.unwrap_or(0);
                let output_str = truncate_output_str(&clean);
                Ok(UnifiedExecResult {
                    exit_code: None,
                    session_id: Some(session_id),
                    output: Some(still_running_output(&output_str)),
                    wall_time_seconds: Some(wall_time),
                    sessions: None,
                    error: None,
                    quiet_seconds: Some(quiet_secs),
                    raw_output: Some(raw),
                })
            }
        }
    }

    /// Handle the `write` action — send input to stdin, poll for output.
    async fn handle_write(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
        let session_id =
            params
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or(ToolError::Validation {
                    tool: "unified_exec",
                    message: "session_id is required for write action".to_string(),
                })?;
        let input = params.get("input").and_then(Value::as_str).unwrap_or("");

        let store = global_store();
        let stdin_tx = store
            .get_stdin_tx(session_id)
            .await
            .ok_or(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Unknown session: {session_id}"),
            })?;

        // Send input
        if !input.is_empty() {
            let _ = stdin_tx.send(input.as_bytes().to_vec()).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Empty write uses poll-level minimum yield time
        let yield_time_ms = params
            .get("yield_time_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient)
            .unwrap_or(DEFAULT_YIELD_TIME_MS);
        let yield_time_ms = if input.is_empty() {
            clamp_poll_yield_time(yield_time_ms)
        } else {
            clamp_yield_time(yield_time_ms)
        };

        poll_session(session_id, yield_time_ms).await
    }

    /// Handle the `poll` action — check for output without sending input.
    async fn handle_poll(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
        let session_id =
            params
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or(ToolError::Validation {
                    tool: "unified_exec",
                    message: "session_id is required for poll action".to_string(),
                })?;

        let yield_time_ms = params
            .get("yield_time_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient)
            .unwrap_or(DEFAULT_YIELD_TIME_MS);
        let yield_time_ms = clamp_poll_yield_time(yield_time_ms);

        poll_session(session_id, yield_time_ms).await
    }

    /// Handle the `list` action — enumerate active sessions.
    async fn handle_list(&self) -> Result<UnifiedExecResult, ToolError> {
        let store = global_store();
        let infos = store.list_sessions().await;
        let sessions: Vec<SessionListEntry> =
            infos.into_iter().map(SessionListEntry::from).collect();

        Ok(UnifiedExecResult {
            exit_code: None,
            session_id: None,
            output: None,
            wall_time_seconds: None,
            sessions: Some(sessions),
            error: None,
            quiet_seconds: None,
            raw_output: None,
        })
    }

    /// Handle the `close` action — terminate a session.
    async fn handle_close(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
        let session_id =
            params
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or(ToolError::Validation {
                    tool: "unified_exec",
                    message: "session_id is required for close action".to_string(),
                })?;

        let store = global_store();
        let killed =
            store
                .close_session(session_id)
                .await
                .map_err(|message| ToolError::Validation {
                    tool: "unified_exec",
                    message,
                })?;
        if !killed {
            return Err(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Unknown session: {session_id}"),
            });
        }

        Ok(UnifiedExecResult {
            exit_code: None,
            session_id: None,
            output: Some(format!("Session {session_id} closed")),
            wall_time_seconds: None,
            sessions: None,
            error: None,
            quiet_seconds: None,
            raw_output: None,
        })
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

    match store.try_wait(session_id).await {
        Some(Some(status)) => {
            // Process exited — clean up and return exit_code
            store.remove(session_id).await;
            Ok(UnifiedExecResult {
                exit_code: status.code().or(Some(-1)),
                session_id: None,
                output: Some(truncate_output_str(&clean)),
                wall_time_seconds: Some(wall_time),
                sessions: None,
                error: None,
                quiet_seconds: None,
                raw_output: Some(raw),
            })
        }
        Some(None) => {
            // Process still running
            let quiet_secs = store.quiet_secs(session_id).await.unwrap_or(0);
            let output_str = truncate_output_str(&clean);
            Ok(UnifiedExecResult {
                exit_code: None,
                session_id: Some(session_id.to_string()),
                output: Some(still_running_output(&output_str)),
                wall_time_seconds: Some(wall_time),
                sessions: None,
                error: None,
                quiet_seconds: Some(quiet_secs),
                raw_output: Some(raw),
            })
        }
        None => {
            // Session no longer in store — the reaper removed it after
            // the process exited. `recover_exit` retrieves the real
            // status the reaper stashed (or -1 when unavailable), so a
            // successful command does NOT degrade to the reaper-race
            // -1 code (research §9.6).
            let exit_code = store
                .recover_exit(session_id)
                .await
                .map(|status| status.code().unwrap_or(-1))
                .unwrap_or(-1);
            Ok(UnifiedExecResult {
                exit_code: Some(exit_code),
                session_id: None,
                output: Some(truncate_output_str(&clean)),
                wall_time_seconds: Some(wall_time),
                sessions: None,
                error: None,
                quiet_seconds: None,
                raw_output: Some(raw),
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
