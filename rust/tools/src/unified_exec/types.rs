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
}

impl std::fmt::Display for UnifiedExecResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default())
    }
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
