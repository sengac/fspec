//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.
//! All tools implement rig::tool::Tool trait for use with RigAgent.
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS

pub mod astgrep;
pub mod astgrep_refactor;
pub mod apply_patch;
pub mod bash;
pub mod blocklist;
pub mod bridge;
pub mod stage_permissions;
pub mod bridge_handler;
pub mod bridge_relay;
pub mod chrome_browser;
pub mod deep_search;
pub mod edit;
pub mod error;
pub mod facade;
pub mod file_type;
pub mod fspec;
pub mod fspec_handler;
pub mod fspec_workflow_guidance;
pub mod inject_summary;

pub mod glob;
pub mod grep;
pub mod image_dimensions;
pub mod limits;
pub mod ls;
pub mod mcp;
pub mod page_fetcher;
pub mod pdf;
pub mod read;
pub mod search_engine;
pub mod tool_pause;
pub mod tool_progress;
pub mod truncation;
pub mod validation;
pub mod view_image;
pub mod web_search;
pub mod session_search;
pub mod write;

// Test fixtures for integration tests
#[cfg(test)]
pub mod bridge_test_fixtures;

// Integration tests using real WebSocket fixtures
#[cfg(test)]
mod bridge_integration_tests;

pub use error::ToolError;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use astgrep::AstGrepTool;
pub use astgrep_refactor::AstGrepRefactorTool;
pub use apply_patch::ApplyPatchTool;
pub use bash::BashTool;
pub use bash::{clear_bash_abort, request_bash_abort};
pub use blocklist::{
    check_bash_command, check_command_raw, check_file_path, init_blocklist, load_blocklist_config,
    project_config_path, reload_blocklist, system_config_path, BlockedError,
    BlocklistAction, BlocklistConfig, BlocklistMatcher, BlocklistRule, CheckResult,
};
pub use stage_permissions::{
    check_write_permission, check_write_raw, init_stage_permissions, load_stage_permissions_config,
    reload_stage_permissions, project_config_path as stage_permissions_project_config_path,
    system_config_path as stage_permissions_system_config_path, FileCategory, StageBlockedError,
    StageCheckResult, StagePermission, StagePermissionsConfig, StagePermissionsMatcher,
};
pub use bridge::{
    get_or_create_bridge_manager, remove_bridge_manager, BridgeAction, BridgeConnectionInfo,
    BridgeConnectionState, BridgeManager, BridgeToolArgs, BridgeResult, BridgeTool,
};
pub use bridge_handler::{
    execute_bridge_command, handle_bridge_action, has_bridge_handler_for_session,
    set_bridge_handler, set_bridge_session_context, remove_bridge_session_context,
    BridgeHandler, BridgeRequest, BroadcastReceiverFactory,
};
pub use bridge_relay::{spawn_relay_task, InputInjector, InjectedInput, ImageData, ControlHandler, CommandEmitter};
pub use chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
pub use edit::EditTool;
pub use fspec::FspecTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use page_fetcher::{Heading, Link, PageContent, PageFetcher};
pub use read::{ReadOutput, ReadTool};
pub use view_image::ViewImageTool;
pub use search_engine::{SearchEngine, SearchResult};
pub use tool_pause::{
    has_pause_handler, pause_for_user, set_pause_handler,
    PauseHandler, PauseKind, PauseRequest, PauseResponse, PauseState,
};
pub use fspec_handler::{
    execute_fspec_command_for_session,
    has_fspec_handler_for_session,
    set_fspec_handler_for_session,
    clear_all_fspec_handlers,
    FspecHandler, FspecRequest as FspecHandlerRequest, FspecResult as FspecHandlerResult,
};
pub use fspec_workflow_guidance::{get_fspec_workflow_guidance, FSPEC_WORKFLOW_GUIDANCE};
pub use mcp::{
    cleanup_mcp_session, connect_http, connect_stdio, disconnect_mcp,
    gather_mcp_tool_registrations, gather_mcp_tool_wrappers, get_mcp_connections,
    init_mcp_session, new_mcp_connection_map, parse_mcp_tool_name,
    qualified_mcp_tool_name, route_mcp_tool_call, set_mcp_tool_server_handle,
    ConnectMcpTool, DynMcpHandler,
    McpAction, McpConnectArgs, McpConnectResult, McpConnection, McpConnectionMap,
    McpConnectionSummary, McpInjection, McpInjectionTx, McpServerInfo, McpToolDef,
    McpToolRegistration, McpToolWrapper, McpTransport,
};
pub use tool_progress::{emit_tool_progress, set_tool_progress_callback, ToolProgressCallback};
pub use web_search::{install_browser_cleanup_handler, shutdown_browser, WebSearchTool};
pub use write::WriteTool;
pub use session_search::{
    SessionSearchTool, SessionSearchHandler,
    set_session_search_handler, has_session_search_handler,
    clear_all_session_search_handlers,
};
pub use inject_summary::{
    InjectSummaryTool, InjectSummaryHandler, InjectSummaryResult,
    set_inject_summary_handler, has_inject_summary_handler,
    execute_inject_summary, clear_all_inject_summary_handlers,
};
pub use deep_search::{
    DeepSearchTool, DeepSearchArgs, DeepSearchHandler,
    DEFAULT_DEEP_SEARCH_MAX_DEPTH, SUB_AGENT_TOOL_NAMES, SUB_AGENT_TOOL_COUNT,
    build_system_prompt, sub_agent_tool_names,
    set_deep_search_handler, has_deep_search_handler, clear_all_deep_search_handlers,
};

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
