//! Fspec tool implementation
//!
//! Provides fspec command execution via NAPI callback pattern

use super::error::ToolError;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Fspec tool for executing fspec commands via NAPI callbacks
pub struct FspecTool;

impl FspecTool {
    /// Create a new FspecTool
    pub fn new() -> Self {
        Self
    }

    /// Execute an fspec command by calling the TypeScript callback
    /// 
    /// This delegates to TypeScript to import and execute the actual command modules.
    /// If TypeScript execution fails, returns a structured error response.
    pub fn execute_via_callback<F>(
        &self, 
        command: &str,
        args_json: &str, 
        project_root: &str,
        callback: F
    ) -> Result<String, String>
    where
        F: Fn(String, String, String) -> Result<String, Box<dyn std::error::Error>>,
    {
        // Validate command is supported
        if !self.is_supported_command(command) {
            return Ok(self.create_error_response(
                &format!("Command '{}' not supported via NAPI", command),
                "UnsupportedCommand",
                &[
                    "Use fspec CLI directly for setup commands",
                    "Check 'fspec --help' for available commands"
                ]
            ));
        }

        // Call TypeScript callback to execute real command
        match callback(command.to_string(), args_json.to_string(), project_root.to_string()) {
            Ok(result) => Ok(result),
            Err(error) => Ok(self.create_error_response(
                &format!("Command execution failed: {}", error),
                "CommandExecutionError", 
                &[
                    "Verify command arguments are valid",
                    "Check that required files exist",
                    "Ensure work unit is in correct state"
                ]
            ))
        }
    }

    /// Check if command is supported via NAPI (excludes setup commands)
    fn is_supported_command(&self, command: &str) -> bool {
        let unsupported_commands = [
            "bootstrap", "init", "configure-tools", "sync-version"
        ];
        
        !unsupported_commands.contains(&command)
    }

    /// Create standardized error response JSON
    fn create_error_response(&self, error: &str, error_type: &str, suggestions: &[&str]) -> String {
        let response = serde_json::json!({
            "success": false,
            "error": error,
            "errorType": error_type,
            "suggestions": suggestions
        });
        response.to_string()
    }
}

impl Default for FspecTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Arguments for Fspec tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FspecArgs {
    /// The fspec command to execute
    pub command: String,
    /// JSON string containing command arguments
    #[serde(default)]
    pub args: String,
    /// Project root directory path
    #[serde(default = "default_project_root")]
    pub project_root: String,
}

fn default_project_root() -> String {
    ".".to_string()
}

impl Tool for FspecTool {
    const NAME: &'static str = "Fspec";

    type Error = ToolError;
    type Args = FspecArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate arguments
        if args.command.is_empty() {
            return Err(ToolError::Validation {
                tool: "fspec",
                message: "command parameter is required".to_string(),
            });
        }

        // TODO: This needs to be connected to the actual NAPI callFspecCommand function
        // For now, we'll return a placeholder that indicates the integration point
        let result = json!({
            "success": false,
            "error": "FspecTool Tool trait implementation requires NAPI integration",
            "errorType": "IntegrationRequired",
            "command": args.command,
            "args": args.args,
            "projectRoot": args.project_root,
            "suggestions": [
                "FspecTool must be connected to NAPI callFspecCommand function",
                "Tool trait call() method needs to use the existing execute_via_callback with proper NAPI bridge",
                "Implementation requires wiring to TypeScript command modules in src/commands/"
            ]
        });

        Ok(result.to_string())
    }
}