use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// NAPI imports for direct function calls (JS-controlled invocation)
use napi::bindgen_prelude::Function;
use super::error::ToolError;

#[derive(Clone, Serialize, Deserialize)]
pub struct FspecTool {
    // No stored callback - JS will provide it at call time
}

impl FspecTool {
    pub fn new() -> Self {
        Self {}
    }

    /// Execute fspec command via JS-provided callback (JS-controlled invocation).
    /// 
    /// The callback signature in TypeScript should be:
    /// `(command: string, argsJson: string, projectRoot: string) => string`
    /// 
    /// This is called directly with the callback provided by JS at execution time.
    pub fn execute_with_js_callback(
        &self,
        command: String,
        args_json: String,
        project_root: String,
        callback: Function<(String, String, String), String>,
    ) -> Result<String, String> {
        callback
            .call((command, args_json, project_root))
            .map_err(|e| e.to_string())
    }
}

impl Default for FspecTool {
    fn default() -> Self {
        Self::new()
    }
}

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
                .unwrap_or_else(|_| serde_json::json!({"type": "object"})),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // NO CLI FALLBACKS - JS-controlled invocation is required
        // The facade wrapper must provide the callback mechanism
        
        Err(ToolError::Execution {
            tool: "fspec",
            message: "FspecTool requires JS-controlled invocation with callback - direct Tool::call not supported".to_string(),
        })
    }
}
