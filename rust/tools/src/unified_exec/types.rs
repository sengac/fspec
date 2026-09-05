//! Type definitions for the unified exec tool.
//!
//! Contains the command representation, result types, and argument wrapper
//! shared across the tool implementation and facades.

use super::process_store::SessionInfo;
use crate::error::ToolError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// ExecCommand — String or Argv
// ============================================================================

/// The command to execute — either a shell string or an argv array.
#[derive(Debug, Clone)]
pub enum ExecCommand {
    /// A shell command string (passed to `sh -c`)
    Shell(String),
    /// An argv array (passed directly to execvp)
    Argv(Vec<String>),
}

impl ExecCommand {
    /// Parse from JSON value.
    pub fn from_value(v: &Value) -> Result<Self, ToolError> {
        match v {
            Value::String(s) => Ok(ExecCommand::Shell(s.clone())),
            Value::Array(arr) => {
                let args: Result<Vec<String>, _> = arr
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .map(String::from)
                            .ok_or_else(|| ToolError::Validation {
                                tool: "unified_exec",
                                message: "command array must contain only strings".to_string(),
                            })
                    })
                    .collect();
                let args = args?;
                if args.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "unified_exec",
                        message: "command array must not be empty".to_string(),
                    });
                }
                Ok(ExecCommand::Argv(args))
            }
            _ => Err(ToolError::Validation {
                tool: "unified_exec",
                message: "command must be a string or array of strings".to_string(),
            }),
        }
    }

    /// Display string for session listing (truncated to 60 chars).
    pub fn display(&self) -> String {
        match self {
            ExecCommand::Shell(s) => {
                if s.len() > 60 {
                    format!("{}...", &s[..57])
                } else {
                    s.clone()
                }
            }
            ExecCommand::Argv(args) => {
                let joined = args.join(" ");
                if joined.len() > 60 {
                    format!("{}...", &joined[..57])
                } else {
                    joined
                }
            }
        }
    }

    /// Get the command string for blocklist checking.
    pub fn blocklist_check_string(&self) -> String {
        match self {
            ExecCommand::Shell(s) => s.clone(),
            ExecCommand::Argv(args) => args.join(" "),
        }
    }
}

// ============================================================================
// UnifiedExecResult
// ============================================================================

/// Fixed steering line appended to every still-running exec result.
///
/// TOOL-022 P1 (deterministic, vtcode-aligned): a pure next-step directive —
/// NO output-content inspection. Mirrors vtcode's `next_wait_args` +
/// `next_action_hint` (`attach_long_command_wait_steering`).
pub const STILL_RUNNING_STEERING: &str = "Command still running. \
If it needs input, send it via the write action. \
Poll with a short yield_time_ms to check for new output.";

/// Result type returned to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedExecResult {
    /// Present when process exited
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Present when process is still running
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Command output (may be truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Wall clock time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_time_seconds: Option<f64>,
    /// Sessions list (for list action only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<SessionListEntry>>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// TOOL-022 P1: seconds since the last output, floored to a whole
    /// number — a deterministic timing fact. Present (and the fixed
    /// steering line attached to `output`) ONLY while the process is
    /// still running; absent when it has exited. No output-content
    /// inspection is involved (vtcode-aligned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_seconds: Option<u64>,
    /// TOOL-022 P4: the RAW merged stdout+stderr bytes captured in this
    /// result's window (stderr lines carry the `⚠stderr⚠` marker).
    /// NEVER serialized for the LLM (`#[serde(skip)]`) — it exists so
    /// the BashTool delegation can split the streams back out
    /// (`bash_output::split_merged_output`) and run the BUG-142 binary
    /// guard on the raw stdout bytes.
    #[serde(skip)]
    pub raw_output: Option<Vec<u8>>,
}

/// TOOL-022 P4: strip the `⚠stderr⚠` line markers from a merged
/// stdout+stderr string for LLM/UI consumption. Line-based; a genuine
/// stdout line that literally starts with the marker is accepted as
/// stripped (the marker is a non-ASCII sentinel no shell command
/// emits in practice — same convention as
/// `bash_output::split_merged_output`).
pub fn strip_stderr_markers(merged: &str) -> String {
    let mut out = String::with_capacity(merged.len());
    for (i, line) in merged.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(
            line.strip_prefix(crate::bash_output::STDERR_MARKER)
                .unwrap_or(line),
        );
    }
    if merged.ends_with('\n') {
        out.push('\n');
    }
    out
}

impl std::fmt::Display for UnifiedExecResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default())
    }
}

/// TOOL-022 P1: seconds since the last output, floored to a whole number.
///
/// `last_output_micros` and `now_micros` are tokio monotonic clock
/// microseconds. This is a pure, deterministic computation — no output
/// content is inspected. The result is used as `UnifiedExecResult.quiet_seconds`
/// and, for the P2 detector, as the quiet-time measurement.
pub fn quiet_secs_since(last_output_micros: u64, now_micros: u64) -> u64 {
    let elapsed = now_micros.saturating_sub(last_output_micros);
    elapsed / 1_000_000
}

// ============================================================================
// SessionListEntry
// ============================================================================

/// Entry in the sessions list returned by the `list` action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListEntry {
    pub session_id: String,
    pub command: String,
    pub tty: bool,
}

impl From<SessionInfo> for SessionListEntry {
    fn from(info: SessionInfo) -> Self {
        Self {
            session_id: info.session_id,
            command: info.command,
            tty: info.tty,
        }
    }
}

// ============================================================================
// UnifiedExecArgs
// ============================================================================

/// Arguments wrapper — accepts raw JSON for flexible dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnifiedExecArgs(pub Value);
