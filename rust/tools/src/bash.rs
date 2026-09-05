//! Bash tool implementation
//!
//! Executes shell commands with output truncation.
//! Supports streaming output to UI while buffering complete output for LLM.
//!
//! # Process Management
//!
//! Commands are spawned in their own process group using `process_group(0)` on Unix.
//! This allows killing the entire process tree (shell + children) when interrupted.
//!
//! When the async task is cancelled (e.g., user presses ESC), the `ProcessGroupKiller`
//! guard sends SIGKILL to the entire process group, ensuring all child processes are
//! terminated. This is necessary because `kill_on_drop(true)` only kills the direct
//! child process, not processes spawned by the shell.
//!
//! # Output Formatting
//!
//! Output is returned without "Stdout:" or "Stderr:" labels for cleaner LLM consumption.
//! - On success: stdout content, with stderr appended if present
//! - On failure: Clear error message with exit code, followed by any output

use super::blocklist::check_bash_command;
use super::error::ToolError;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

// Re-export public items so external users can still use `bash::*`
pub use crate::bash_abort::{
    clear_bash_abort, is_bash_abort_requested, request_bash_abort, unregister_bash_abort_flag,
};
pub use crate::bash_output::STDERR_MARKER;
pub use crate::bash_streams::StreamCallback;

/// Arguments for Bash tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BashArgs {
    /// The bash command to execute
    pub command: String,
    /// Optional working directory for command execution.
    /// If provided, the command will run in this directory instead of inheriting from the parent process.
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Bash tool for executing shell commands
pub struct BashTool {
    /// Session ID for worktree isolation support.
    /// The tool looks up the effective_cwd for the session to execute commands in the correct directory.
    session_id: uuid::Uuid,
}

impl BashTool {
    /// Create a new Bash tool instance with session awareness.
    ///
    /// The session_id is used to look up the effective_cwd (worktree path for
    /// isolated sessions) so commands execute in the correct directory.
    pub fn new(session_id: uuid::Uuid) -> Self {
        Self { session_id }
    }

    /// Get the effective cwd for this tool instance.
    ///
    /// Looks up the effective_cwd via the global callback.
    /// Returns None if no callback registered (non-isolated session).
    fn get_effective_cwd(&self) -> Option<std::path::PathBuf> {
        crate::facade::get_effective_cwd(self.session_id)
    }

    /// Resolve the working directory: session isolation takes precedence over args.cwd.
    fn resolve_cwd(&self, args_cwd: Option<String>) -> Option<String> {
        let effective_cwd = self.get_effective_cwd();
        effective_cwd
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .or(args_cwd)
    }

    /// Resolve CWD and update the footer registry so the poller shows the actual directory.
    fn resolve_and_track_cwd(&self, args_cwd: Option<String>) -> Option<String> {
        let resolved = self.resolve_cwd(args_cwd);
        // Write the actual CWD to the per-session footer registry.
        // If resolved is None, the command inherits process CWD — record that.
        let cwd_for_footer = resolved.clone().or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });
        if let Some(cwd) = cwd_for_footer {
            crate::footer_cwd::update_footer_cwd(self.session_id, cwd);
        }
        resolved
    }

    /// Execute command with streaming output to UI.
    ///
    /// Streams output line-by-line via callback while buffering complete output for LLM.
    /// UI sees full output in real-time; LLM gets truncated buffered result.
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `stream_callback` - Optional callback for streaming output chunks
    ///
    /// # Returns
    /// Complete buffered output (truncated if necessary) for LLM consumption
    pub async fn call_with_streaming(
        &self,
        args: BashArgs,
        stream_callback: Option<StreamCallback>,
    ) -> Result<String, ToolError> {
        if args.command.is_empty() {
            return Err(ToolError::Validation {
                tool: "bash",
                message: "command parameter is required".to_string(),
            });
        }

        // If no streaming callback, use the non-streaming path
        let Some(callback) = stream_callback else {
            return self.call(args).await;
        };

        // Check command against blocklist before execution
        if let Err(blocked) = check_bash_command(&args.command, self.session_id) {
            return Err(ToolError::Blocked {
                tool: "bash",
                message: blocked.to_string(),
            });
        }

        // TOOL-013: Determine effective working directory
        // TUI-091: Also update footer CWD registry for dynamic display
        let cwd = self.resolve_and_track_cwd(args.cwd);

        // TOOL-022 P4: delegate to the unified exec session machinery —
        // live session (stdin piped, pager env, abort-flag aware) with
        // the TOOL-022 P2 quiet detector armed for the TUI overlay.
        // stdout lines stream to the callback; stderr stays in the
        // final result (pre-P4 streaming contract).
        clear_bash_abort(self.session_id);
        let result = crate::bash_session::run_bash_session(
            self.session_id,
            &args.command,
            cwd.as_deref(),
            crate::bash_session::BashUiStream::Callback(callback),
        )
        .await?;
        crate::bash_session::finalize_bash_result(result)
    }
}

impl rig::tool::Tool for BashTool {
    const NAME: &'static str = "Bash";

    type Error = ToolError;
    type Args = BashArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Bash".to_string(),
            description: "Execute a bash command. Returns stdout or error message with stderr."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BashArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-017: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            "Bash",
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Bash",
                message: reason,
            });
        }

        if args.command.is_empty() {
            return Err(ToolError::Validation {
                tool: "bash",
                message: "command parameter is required".to_string(),
            });
        }

        // Check command against blocklist before execution
        if let Err(blocked) = check_bash_command(&args.command, self.session_id) {
            return Err(ToolError::Blocked {
                tool: "bash",
                message: blocked.to_string(),
            });
        }

        // TOOL-013: Determine effective working directory
        // TUI-091: Also update footer CWD registry for dynamic display
        let cwd = self.resolve_and_track_cwd(args.cwd);

        // TOOL-022 P4: delegate to the unified exec session machinery —
        // live session (stdin piped, pager env, abort-flag aware) with
        // the TOOL-022 P2 quiet detector armed for the TUI overlay.
        // stdout AND stderr stream to the UI via tool progress (red
        // styling for stderr — pre-P4 contract preserved).
        clear_bash_abort(self.session_id);
        let result = crate::bash_session::run_bash_session(
            self.session_id,
            &args.command,
            cwd.as_deref(),
            crate::bash_session::BashUiStream::ToolProgress,
        )
        .await?;
        crate::bash_session::finalize_bash_result(result)
    }
}
