//! Core traits and types for the tool facade pattern.

use crate::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Tool definition that can be sent to LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name as the provider expects it
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for the tool parameters
    pub parameters: Value,
}

/// Internal parameters for web search operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalWebSearchParams {
    /// Perform a web search with the given query
    Search { query: String },
    /// Open and fetch content from a URL
    OpenPage { url: String },
    /// Find a pattern within a page's content
    FindInPage { url: String, pattern: String },
    /// Capture a screenshot of a web page
    CaptureScreenshot {
        url: String,
        output_path: Option<String>,
        full_page: bool,
    },
}

/// Internal parameters for file operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalFileParams {
    /// Read file content
    Read {
        file_path: String,
        offset: Option<usize>,
        limit: Option<usize>,
    },
    /// Write content to file
    Write { file_path: String, content: String },
    /// Edit/replace text in file
    Edit {
        file_path: String,
        old_string: String,
        new_string: String,
    },
}

/// Provider-specific tool facade trait for web search operations.
///
/// Each facade adapts a tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait ToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal parameters
    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError>;
}

/// Type alias for a boxed ToolFacade
pub type BoxedToolFacade = Arc<dyn ToolFacade>;

/// Provider-specific tool facade trait for file operations.
///
/// Each facade adapts a file tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait FileToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal file parameters
    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError>;
}

/// Type alias for a boxed FileToolFacade
pub type BoxedFileToolFacade = Arc<dyn FileToolFacade>;

/// Internal parameters for bash/shell operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalBashParams {
    /// Execute a shell command
    Execute { command: String },
}

/// Provider-specific tool facade trait for bash/shell operations.
///
/// Each facade adapts a bash tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait BashToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal bash parameters
    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError>;
}

/// Type alias for a boxed BashToolFacade
pub type BoxedBashToolFacade = Arc<dyn BashToolFacade>;

/// Internal parameters for search operations (grep/glob).
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalSearchParams {
    /// Search file contents with pattern (grep)
    Grep {
        pattern: String,
        path: Option<String>,
    },
    /// Find files matching glob pattern
    Glob {
        pattern: String,
        path: Option<String>,
    },
}

/// Provider-specific tool facade trait for search operations.
///
/// Each facade adapts a search tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait SearchToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal search parameters
    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError>;
}

/// Type alias for a boxed SearchToolFacade
pub type BoxedSearchToolFacade = Arc<dyn SearchToolFacade>;

/// Internal parameters for directory listing operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalLsParams {
    /// List directory contents
    List { path: Option<String> },
}

/// Provider-specific tool facade trait for directory listing operations.
///
/// Each facade adapts a directory listing tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait LsToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal ls parameters
    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError>;
}

/// Type alias for a boxed LsToolFacade
pub type BoxedLsToolFacade = Arc<dyn LsToolFacade>;
