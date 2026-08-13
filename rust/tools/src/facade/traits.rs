//! Core traits and types for the tool facade pattern.

use super::fspec_facade::InternalFspecParams;
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
    OpenPage {
        url: String,
        /// If true, runs Chrome in headless mode (no visible UI). If false, shows the browser window.
        headless: bool,
        /// If true, pause after page load for user interaction before returning.
        pause: bool,
    },
    /// Find a pattern within a page's content
    FindInPage {
        url: String,
        pattern: String,
        /// If true, runs Chrome in headless mode (no visible UI). If false, shows the browser window.
        headless: bool,
        /// If true, pause after page load for user interaction before returning.
        pause: bool,
    },
    /// Capture a screenshot of a web page
    CaptureScreenshot {
        url: String,
        output_path: Option<String>,
        full_page: bool,
        /// If true, runs Chrome in headless mode (no visible UI). If false, shows the browser window.
        headless: bool,
        /// If true, pause after page load for user interaction before capturing screenshot.
        pause: bool,
    },
}

/// Parameters for Codex-style indentation-aware block reading.
///
/// When `mode` is `"indentation"`, these parameters control how the read_file
/// tool expands around an anchor line using indentation structure.
/// All fields are optional to match the Codex CLI spec defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalIndentationParams {
    /// Anchor line to center the indentation lookup on (defaults to offset).
    pub anchor_line: Option<usize>,
    /// How many parent indentation levels (smaller indents) to include.
    pub max_levels: Option<usize>,
    /// When true, include additional blocks that share the anchor indentation.
    pub include_siblings: Option<bool>,
    /// Include doc comments or attributes directly above the selected block.
    pub include_header: Option<bool>,
    /// Hard cap on the number of lines returned when using indentation mode.
    pub max_lines: Option<usize>,
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
        /// Optional mode selector: "slice" for simple ranges (default) or "indentation"
        /// to expand around an anchor line.
        mode: Option<String>,
        /// Optional indentation configuration used when mode is "indentation".
        indentation: Option<InternalIndentationParams>,
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
    Execute {
        command: String,
        /// Optional working directory override from the facade.
        /// Session isolation effective_cwd takes precedence over this value.
        cwd: Option<String>,
        /// Optional timeout in milliseconds.
        /// Stored for future use — BashTool does not currently enforce per-command timeouts.
        timeout_ms: Option<u64>,
    },
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
        /// Glob filter to limit which files are searched (e.g., "*.rs").
        /// Maps to ripgrep's `--glob` flag. Used by Codex's `include` param.
        include: Option<String>,
        /// Maximum number of file paths to return.
        /// Used by Codex's `limit` param to cap results.
        limit: Option<usize>,
    },
    /// Find files matching glob pattern
    Glob {
        pattern: String,
        path: Option<String>,
    },
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
pub type BoxedFspecToolFacade = Arc<dyn FspecToolFacade>;

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
    List {
        path: Option<String>,
        /// The entry number to start listing from (1-indexed).
        /// Used by Codex's `offset` param for pagination.
        offset: Option<usize>,
        /// The maximum number of entries to return.
        /// Used by Codex's `limit` param for pagination.
        limit: Option<usize>,
        /// The maximum directory depth to traverse.
        /// Used by Codex's `depth` param. Must be 1 or greater.
        depth: Option<usize>,
    },
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

// ============================================================================
// Exec Tool Facade (TOOL-016: Unified Exec Tool)
// ============================================================================

/// Internal parameters for unified exec operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalExecParams {
    /// Execute a command (one-shot or session-creating)
    Run {
        /// Command as shell string or argv array
        command: Value,
        /// Working directory override
        workdir: Option<String>,
        /// Allocate PTY
        tty: bool,
        /// Yield time in ms before returning
        yield_time_ms: Option<u64>,
        /// Max output tokens
        max_output_tokens: Option<u64>,
        /// Hard timeout in seconds
        timeout_secs: Option<u64>,
    },
    /// Send input to a running session
    Write {
        /// Session identifier
        session_id: String,
        /// Input bytes to send to stdin
        input: String,
        /// Yield time in ms
        yield_time_ms: Option<u64>,
        /// Max output tokens
        max_output_tokens: Option<u64>,
    },
    /// Poll output from a running session (no stdin input)
    Poll {
        /// Session identifier
        session_id: String,
        /// Yield time in ms
        yield_time_ms: Option<u64>,
        /// Max output tokens
        max_output_tokens: Option<u64>,
    },
    /// List active sessions
    List,
    /// Close/terminate a session
    Close {
        /// Session identifier
        session_id: String,
    },
}

/// Provider-specific tool facade trait for unified exec operations.
///
/// Each facade adapts an exec tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
/// Used by Codex facades (BUG-114/BUG-115) to map exec_command/write_stdin/shell
/// to the provider-agnostic unified exec tool (TOOL-016).
pub trait ExecToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "codex")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal exec parameters
    fn map_params(&self, input: Value) -> Result<InternalExecParams, ToolError>;
}

/// Type alias for a boxed ExecToolFacade
pub type BoxedExecToolFacade = Arc<dyn ExecToolFacade>;

// ============================================================================
// HITL Tool Facade (BUG-116: Codex request_user_input facade)
// ============================================================================

use crate::request_user_input::HitlQuestion;

/// Internal parameters for HITL (human-in-the-loop) operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalHitlParams {
    /// Request user input with structured questions
    Request {
        /// Array of 1-3 validated questions to present to the user
        questions: Vec<HitlQuestion>,
    },
}

/// Provider-specific tool facade trait for HITL operations.
///
/// Each facade adapts the HITL tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
/// Used by the Codex facade (BUG-116) to map request_user_input to the
/// provider-agnostic HITL tool (TOOL-017).
pub trait HitlToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "codex")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal HITL parameters
    fn map_params(&self, input: Value) -> Result<InternalHitlParams, ToolError>;
}

/// Type alias for a boxed HitlToolFacade
pub type BoxedHitlToolFacade = Arc<dyn HitlToolFacade>;
