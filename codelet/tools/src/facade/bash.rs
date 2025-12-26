//! Bash/Shell operation facades for different LLM providers.
//!
//! These facades adapt the BashTool interface for provider-specific
//! tool naming and parameter schemas.

use super::traits::{BashToolFacade, InternalBashParams, ToolDefinition};
use crate::ToolError;
use serde_json::{json, Value};

/// Gemini-specific facade for shell command execution.
///
/// Maps Gemini's `run_shell_command` tool with flat `{command}` schema
/// to the internal BashTool parameters.
pub struct GeminiRunShellCommandFacade;

impl BashToolFacade for GeminiRunShellCommandFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "run_shell_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_shell_command".to_string(),
            description: "Execute a shell command and return the output".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
        let command = input
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "run_shell_command",
                message: "Missing 'command' field".to_string(),
            })?
            .to_string();

        Ok(InternalBashParams::Execute { command })
    }
}
