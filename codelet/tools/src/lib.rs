//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.
//! All tools implement rig::tool::Tool trait for use with RigAgent.

pub mod astgrep;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod limits;
pub mod ls;
pub mod read;
pub mod truncation;
pub mod validation;
pub mod write;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use astgrep::AstGrepTool;
pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use read::ReadTool;
pub use write::WriteTool;

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

/// Tool execution output (used by validation helpers)
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
