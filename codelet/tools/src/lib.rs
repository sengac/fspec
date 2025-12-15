//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.

pub mod astgrep;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod limits;
pub mod read;
pub mod truncation;
pub mod validation;
pub mod write;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use astgrep::AstGrepTool;
pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use read::ReadTool;
pub use write::WriteTool;

/// Tool parameters schema for LLM tool definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    /// JSON Schema type (always "object" for tool parameters)
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Parameter properties
    pub properties: serde_json::Map<String, Value>,
    /// Required parameter names
    #[serde(default)]
    pub required: Vec<String>,
}

impl Default for ToolParameters {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: serde_json::Map::new(),
            required: Vec::new(),
        }
    }
}

/// Tool definition for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool input parameters
    pub input_schema: Value,
}

/// Tool execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Output content
    pub content: String,
    /// Whether output was truncated
    pub truncated: bool,
    /// Whether this is an error response
    pub is_error: bool,
}

impl ToolOutput {
    /// Create a successful output
    pub fn success(content: String) -> Self {
        Self {
            content,
            truncated: false,
            is_error: false,
        }
    }

    /// Create an error output
    pub fn error(message: String) -> Self {
        Self {
            content: message,
            truncated: false,
            is_error: true,
        }
    }
}

/// Tool trait for implementing tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// Tool parameters schema
    fn parameters(&self) -> &ToolParameters;

    /// Execute the tool with given arguments
    async fn execute(&self, args: Value) -> Result<ToolOutput>;
}

/// Tool registry for managing available tools
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: std::collections::HashMap::new(),
        }
    }

    /// Create a registry with all core tools pre-registered
    pub fn with_core_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(AstGrepTool::new()));
        registry.register(Box::new(BashTool::new()));
        registry.register(Box::new(ReadTool::new()));
        registry.register(Box::new(WriteTool::new()));
        registry.register(Box::new(EditTool::new()));
        registry.register(Box::new(GrepTool::new()));
        registry.register(Box::new(GlobTool::new()));
        registry
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Execute a tool by name with given arguments
    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolOutput> {
        match self.get(name) {
            Some(tool) => tool.execute(args).await,
            None => Ok(ToolOutput::error(format!("Unknown tool: {name}"))),
        }
    }

    /// List available tool names
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(std::string::String::as_str).collect()
    }

    /// Get tool count
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Get tool definitions for API requests
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: serde_json::to_value(tool.parameters())
                    .unwrap_or(serde_json::json!({})),
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_core_tools()
    }
}
