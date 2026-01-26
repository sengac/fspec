//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.
//! All tools implement rig::tool::Tool trait for use with RigAgent.

pub mod astgrep;
pub mod astgrep_refactor;
pub mod bash;
pub mod chrome_browser;
pub mod edit;
pub mod error;
pub mod facade;
pub mod file_type;
pub mod glob;
pub mod grep;
pub mod limits;
pub mod ls;
pub mod page_fetcher;
pub mod pdf;
pub mod read;
pub mod search_engine;
pub mod tool_pause;
pub mod tool_progress;
pub mod truncation;
pub mod validation;
pub mod web_search;
pub mod write;

pub use error::ToolError;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use astgrep::AstGrepTool;
pub use astgrep_refactor::AstGrepRefactorTool;
pub use bash::BashTool;
pub use bash::{clear_bash_abort, request_bash_abort};
pub use chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use page_fetcher::{Heading, Link, PageContent, PageFetcher};
pub use read::{ReadOutput, ReadTool};
pub use search_engine::{SearchEngine, SearchResult};
pub use tool_pause::{
    has_pause_handler, pause_for_user, with_pause_handler, with_pause_handler_async,
    PauseHandler, PauseKind, PauseRequest, PauseResponse, PauseState, PAUSE_HANDLER,
};
pub use tool_progress::{emit_tool_progress, set_tool_progress_callback, ToolProgressCallback};
pub use web_search::{install_browser_cleanup_handler, shutdown_browser, WebSearchTool};
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
