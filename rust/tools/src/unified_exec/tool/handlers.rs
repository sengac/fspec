//! UnifiedExecTool action handlers (REFAC-046: moved from `tool.rs`).

use super::*;
use crate::unified_exec::stdin_write::write_stdin;

// ============================================================================
// Action Handlers
// ============================================================================

impl UnifiedExecTool {
    /// Handle the `run` action — spawn a process, collect output, return result.
    pub(super) async fn handle_run(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
        let owner = capture_owner(self.session_id);
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
        let workdir = self.resolve_workdir(params.get("workdir").and_then(Value::as_str))?;

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
        let (
            mut child,
            stdin_tx,
            output_buffer,
            output_notify,
            last_output_micros,
            kill_handle,
            pty_exit,
        ) = if tty {
            spawn_pty_process(&command, workdir.as_deref())?
        } else {
            spawn_pipe_process(&command, workdir.as_deref())?
        };
        let session_id = generate_session_id();
        let owner_registration = owner
            .map(|owner| ProcessRegistration::new(owner, session_id.clone(), kill_handle.clone()));

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
                // Process exited — report how it really ended (BUG-368:
                // the PTY child's status, a signal, or unavailable).
                let exit = crate::unified_exec::ExecExit::resolve(status, pty_exit.as_ref());
                Ok(UnifiedExecResult {
                    output: Some(truncate_output_str(&clean)),
                    wall_time_seconds: Some(wall_time),
                    raw_output: Some(raw),
                    ..UnifiedExecResult::exited(&exit)
                })
            }
            _ => {
                // Process still running — store and return session_id
                let entry = ProcessEntry {
                    owner_registration,
                    child,
                    stdin_tx,
                    output_buffer,
                    output_notify,
                    last_used: Instant::now(),
                    tty,
                    command_display: command.display(),
                    last_output_micros,
                    kill_handle,
                    pty_exit,
                };
                store.insert(session_id.clone(), entry).await;
                spawn_reaper(session_id.clone());
                // TOOL-022 P2: deterministic quiet detector — pushes an
                // ExecStdinRequest to the owning agent session's callback
                // once the child is alive and quiet >= 3s (no status flip).
                spawn_exec_stdin_detector(self.session_id, session_id.clone(), command.display());

                // BUG-368: no guessed quiet time.
                let quiet_secs = store.quiet_secs(&session_id).await;
                let output_str = truncate_output_str(&clean);
                Ok(UnifiedExecResult {
                    exit_code: None,
                    signal: None,
                    exit_status_unavailable: None,
                    session_id: Some(session_id),
                    output: Some(still_running_output(&output_str)),
                    wall_time_seconds: Some(wall_time),
                    sessions: None,
                    error: None,
                    quiet_seconds: quiet_secs,
                    raw_output: Some(raw),
                })
            }
        }
    }

    /// Handle the `write` action — send input to stdin, poll for output.
    pub(super) async fn handle_write(
        &self,
        params: &Value,
    ) -> Result<UnifiedExecResult, ToolError> {
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
            // BUG-362: the write is acknowledged with its real io result.
            write_stdin(&stdin_tx, session_id, input.as_bytes().to_vec())
                .await
                .map_err(|message| ToolError::Execution {
                    tool: "unified_exec",
                    message,
                })?;
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
    pub(super) async fn handle_poll(&self, params: &Value) -> Result<UnifiedExecResult, ToolError> {
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
    pub(super) async fn handle_list(&self) -> Result<UnifiedExecResult, ToolError> {
        let store = global_store();
        let infos = store.list_sessions().await;
        let sessions: Vec<SessionListEntry> =
            infos.into_iter().map(SessionListEntry::from).collect();

        Ok(UnifiedExecResult {
            exit_code: None,
            signal: None,
            exit_status_unavailable: None,
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
    pub(super) async fn handle_close(
        &self,
        params: &Value,
    ) -> Result<UnifiedExecResult, ToolError> {
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
            signal: None,
            exit_status_unavailable: None,
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
