//! Fspec operation facades for different LLM providers.
//!
//! These facades adapt the FspecTool interface for provider-specific
//! tool naming and parameter schemas.

use super::traits::ToolDefinition;
use crate::ToolError;
use serde_json::{json, Value};
use crate::fspec::FspecArgs;

/// Internal parameters for fspec operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalFspecParams {
    pub command: String,
    pub args: String,
    pub project_root: String,
}

/// Provider-specific tool facade trait for fspec operations.
///
/// Each facade adapts the fspec tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait FspecToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal parameters
    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError>;
}

/// Type alias for a boxed FspecToolFacade
pub type BoxedFspecToolFacade = std::sync::Arc<dyn FspecToolFacade>;

/// Claude-specific facade for fspec command execution.
///
/// Maps Claude's `Fspec` tool with standard schema to internal FspecTool parameters.
pub struct ClaudeFspecFacade;

impl FspecToolFacade for ClaudeFspecFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn tool_name(&self) -> &'static str {
        "Fspec"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "Fspec".to_string(),
            description: concat!(
                "Execute fspec commands for Acceptance Criteria Driven Development (ACDD). ",
                "Manages Gherkin feature files, work units, and project specifications. ",
                "Supports work unit creation, status updates, Example Mapping, and workflow automation. ",
                "Excludes setup commands (bootstrap, init) which should be run via CLI."
            ).to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FspecArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        let fspec_args: FspecArgs = serde_json::from_value(input).map_err(|e| {
            ToolError::Validation {
                tool: "fspec",
                message: format!("Invalid arguments: {e}"),
            }
        })?;

        Ok(InternalFspecParams {
            command: fspec_args.command,
            args: fspec_args.args,
            project_root: fspec_args.project_root,
        })
    }
}

/// Gemini-specific facade for fspec command execution.
///
/// Maps Gemini's `fspec_command` tool with snake_case schema to internal FspecTool parameters.
pub struct GeminiFspecFacade;

impl FspecToolFacade for GeminiFspecFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "fspec_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fspec_command".to_string(),
            description: concat!(
                "Execute fspec commands for ACDD workflow management. ",
                "Handle work units, feature files, Example Mapping, and Gherkin specifications. ",
                "Core commands: create-work-unit, update-work-unit-status, add-rule, add-example, generate-scenarios."
            ).to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The fspec command to execute (e.g., 'create-work-unit', 'update-work-unit-status')"
                    },
                    "args": {
                        "type": "string", 
                        "description": "JSON string containing command arguments",
                        "default": "{}"
                    },
                    "project_root": {
                        "type": "string",
                        "description": "Project root directory path",
                        "default": "."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        let command = input.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "fspec_command",
                message: "Missing 'command' field".to_string(),
            })?
            .to_string();

        let args = input.get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("{}")
            .to_string();

        let project_root = input.get("project_root")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();

        Ok(InternalFspecParams {
            command,
            args,
            project_root,
        })
    }
}

/// OpenAI-specific facade for fspec command execution.
///
/// Maps OpenAI's `fspec` tool with standard schema to internal FspecTool parameters.
pub struct OpenAIFspecFacade;

impl FspecToolFacade for OpenAIFspecFacade {
    fn provider(&self) -> &'static str {
        "openai"
    }

    fn tool_name(&self) -> &'static str {
        "fspec"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fspec".to_string(),
            description: concat!(
                "Execute fspec commands for Acceptance Criteria Driven Development (ACDD). ",
                "Manages Gherkin feature files, work units, and project specifications. ",
                "Supports work unit creation, status updates, Example Mapping, and workflow automation."
            ).to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FspecArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        let fspec_args: FspecArgs = serde_json::from_value(input).map_err(|e| {
            ToolError::Validation {
                tool: "fspec",
                message: format!("Invalid arguments: {e}"),
            }
        })?;

        Ok(InternalFspecParams {
            command: fspec_args.command,
            args: fspec_args.args,
            project_root: fspec_args.project_root,
        })
    }
}

/// Z.AI-specific facade for fspec command execution.
///
/// Maps Z.AI's `run_fspec` tool with snake_case schema to internal FspecTool parameters.
pub struct ZAIFspecFacade;

impl FspecToolFacade for ZAIFspecFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "run_fspec"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_fspec".to_string(),
            description: "Execute fspec ACDD workflow commands. Manage work units, features, and specifications.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "fspec command to execute"
                    },
                    "arguments": {
                        "type": "string",
                        "description": "JSON arguments for command",
                        "default": "{}"
                    },
                    "root_dir": {
                        "type": "string", 
                        "description": "project root directory",
                        "default": "."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        let command = input.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "run_fspec",
                message: "Missing 'command' field".to_string(),
            })?
            .to_string();

        let args = input.get("arguments")
            .and_then(|v| v.as_str())
            .unwrap_or("{}")
            .to_string();

        let project_root = input.get("root_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();

        Ok(InternalFspecParams {
            command,
            args,
            project_root,
        })
    }
}