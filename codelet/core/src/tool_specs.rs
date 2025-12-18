// Copied from Codex /tmp/codex/codex-rs/core/src/client_common.rs lines 191-213
// Using astgrep research to copy exact ToolSpec implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum ToolSpec {
    #[serde(rename = "function")]
    Function(ResponsesApiTool),
    #[serde(rename = "local_shell")]
    LocalShell {},
    // Copied from Codex: Native OpenAI web_search tool
    #[serde(rename = "web_search")]
    WebSearch {},
    #[serde(rename = "custom")]
    Freeform(FreeformTool),
}

impl ToolSpec {
    pub fn name(&self) -> &str {
        match self {
            ToolSpec::Function(tool) => tool.name.as_str(),
            ToolSpec::LocalShell {} => "local_shell",
            ToolSpec::WebSearch {} => "web_search",
            ToolSpec::Freeform(tool) => tool.name.as_str(),
        }
    }
}

// Simplified tool structures for compilation - will need full implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponsesApiTool {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreeformTool {
    pub name: String,
    pub description: String,
}
