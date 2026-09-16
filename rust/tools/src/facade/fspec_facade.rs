//! Fspec operation facades for different LLM providers.
//!
//! These facades adapt the FspecTool interface for provider-specific
//! tool naming and parameter schemas.
//!
//! TOOL-023: every `map_params` implementation detects the two most
//! common LLM arg mistakes BEFORE deserialization and emits a dedicated,
//! tool-named [`ToolError::Validation`] with a corrected example:
//! * `args`/`arguments` sent as a JSON object instead of a JSON string
//!   (the #1 source of fspec tool failures — LLMs trained on
//!   OpenAI/Anthropic function-calling naturally emit objects), and
//! * `project_root` sent as a non-string.
//!
//! Unknown command names are passed through unchanged — the did-you-mean
//! suggestion is produced by the fspec-core dispatcher.

use super::traits::ToolDefinition;
use crate::fspec::FspecArgs;
use crate::ToolError;
use serde_json::{json, Value};

/// Internal parameters for fspec operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalFspecParams {
    pub command: String,
    pub args: String,
    pub project_root: String,
}

/// TOOL-023: dedicated explanation for the string-vs-object `args` gotcha.
fn args_object_error(tool_name: &str, args_field: &str, raw: &Value) -> ToolError {
    let example = raw
        .as_object()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("backlog");
    ToolError::Validation {
        tool: "fspec",
        message: format!(
            "{tool_name} tool: the \"{args_field}\" parameter must be a string containing JSON, not a JSON object. \
             A corrected example of your call: {{\"command\": \"board\", \"{args_field}\": \"{{\\\"status\\\": \\\"{example}\\\"}}\", \"project_root\": \".\"}}. \
             Pass the same JSON content, but encoded as a string; use an empty string \"{{}}\" when the command takes no arguments."
        ),
    }
}

/// TOOL-023: dedicated explanation for a non-string `project_root`.
fn project_root_type_error(tool_name: &str) -> ToolError {
    ToolError::Validation {
        tool: "fspec",
        message: format!(
            "{tool_name} tool: the \"project_root\" parameter must be a string path (e.g. \".\" or the absolute project directory), not a number or object."
        ),
    }
}

/// TOOL-023: dedicated explanation for a missing `command`, with an
/// example invocation and a pointer at the help command.
fn command_missing_error(tool_name: &str, args_field: &str) -> ToolError {
    ToolError::Validation {
        tool: "fspec",
        message: format!(
            "{tool_name} tool: the \"command\" parameter is required. \
             Example: {{\"command\": \"list-work-units\", \"{args_field}\": \"{{\\\"status\\\": \\\"backlog\\\"}}\", \"project_root\": \".\"}}. \
             Run {{\"command\": \"help\"}} for the available command list, or append \" --help\" to a command name for its full argument reference."
        ),
    }
}

/// Provider-specific tool facade trait for fspec operations (TOOL-023: the
/// canonical trait definition lives in [`super::traits`]; this re-export
/// keeps the single source of truth so the provider facades, the wrapper,
/// and the tests all operate on the same object).
pub use super::traits::{BoxedFspecToolFacade, FspecToolFacade};

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
                "Use command=\"help\" to get detailed documentation on available commands and how to use them. ",
                "Excludes setup commands (bootstrap, init) which should be run via CLI."
            ).to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FspecArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        // TOOL-023: detect the string-vs-object `args` gotcha before serde
        // sees it — the generic serde error for this case is opaque
        // ("invalid type: map, expected a string at line 1 column N").
        if let Some(args) = input.get("args") {
            if args.is_object() || args.is_array() {
                return Err(args_object_error("Fspec", "args", args));
            }
        }
        if let Some(root) = input.get("project_root") {
            if !root.is_string() && !root.is_null() {
                return Err(project_root_type_error("Fspec"));
            }
        }
        let fspec_args: FspecArgs = serde_json::from_value(input).map_err(|e| {
            // TOOL-023: name the tool and show the expected shape.
            ToolError::Validation {
                tool: "fspec",
                message: format!(
                    "Fspec tool: invalid arguments: {e}. \
                     Expected {{\"command\": \"<command>\", \"args\": \"<json string>\", \"project_root\": \"<path>\"}}. \
                     Use command \"help\" for the command list."
                ),
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
                "Use command=\"help\" to get detailed documentation on available commands. ",
                "Core commands: create-work-unit, update-work-unit-status, add-rule, add-example, generate-scenarios."
            ).to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The fspec command to execute (e.g., 'help', 'create-work-unit', 'update-work-unit-status')"
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
        let tool = self.tool_name();
        // TOOL-023: dedicated missing-command error with a usage example.
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| command_missing_error(tool, "args"))?
            .to_string();

        // TOOL-023: detect the string-vs-object `args` gotcha.
        let args = match input.get("args") {
            Some(raw) if raw.is_object() || raw.is_array() => {
                return Err(args_object_error(tool, "args", raw));
            }
            _ => input
                .get("args")
                .and_then(|v| v.as_str())
                .unwrap_or("{}")
                .to_string(),
        };

        // TOOL-023: detect non-string project_root (null is treated as absent).
        let project_root = match input.get("project_root") {
            Some(v) if !v.is_string() && !v.is_null() => return Err(project_root_type_error(tool)),
            _ => input
                .get("project_root")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string(),
        };

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
                "Supports work unit creation, status updates, Example Mapping, and workflow automation. ",
                "Use command=\"help\" to get detailed documentation on available commands and how to use them."
            ).to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FspecArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError> {
        // TOOL-023: detect the string-vs-object `args` gotcha before serde
        // sees it (same failure mode as the Claude facade).
        if let Some(args) = input.get("args") {
            if args.is_object() || args.is_array() {
                return Err(args_object_error("fspec", "args", args));
            }
        }
        if let Some(root) = input.get("project_root") {
            if !root.is_string() && !root.is_null() {
                return Err(project_root_type_error("fspec"));
            }
        }
        let fspec_args: FspecArgs = serde_json::from_value(input).map_err(|e| {
            // TOOL-023: name the tool and show the expected shape.
            ToolError::Validation {
                tool: "fspec",
                message: format!(
                    "fspec tool: invalid arguments: {e}. \
                     Expected {{\"command\": \"<command>\", \"args\": \"<json string>\", \"project_root\": \"<path>\"}}. \
                     Use command \"help\" for the command list."
                ),
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
            description: concat!(
                "Execute fspec ACDD workflow commands. Manage work units, features, and specifications. ",
                "Use command=\"help\" to get detailed documentation on available commands."
            ).to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "fspec command to execute (use 'help' for documentation)"
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
        let tool = self.tool_name();
        // TOOL-023: dedicated missing-command error with a usage example.
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| command_missing_error(tool, "arguments"))?
            .to_string();

        // TOOL-023: detect the string-vs-object `arguments` gotcha.
        let args = match input.get("arguments") {
            Some(raw) if raw.is_object() || raw.is_array() => {
                return Err(args_object_error(tool, "arguments", raw));
            }
            _ => input
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}")
                .to_string(),
        };

        // TOOL-023: detect non-string root_dir.
        let project_root = match input.get("root_dir") {
            Some(v) if !v.is_string() && !v.is_null() => return Err(project_root_type_error(tool)),
            _ => input
                .get("root_dir")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string(),
        };

        Ok(InternalFspecParams {
            command,
            args,
            project_root,
        })
    }
}
