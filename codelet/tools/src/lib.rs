//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.
//! All tools implement rig::tool::Tool trait for use with RigAgent.
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS

pub mod agent_manager;
pub mod astgrep;
pub mod astgrep_refactor;
pub mod apply_patch;
pub mod bash;
pub mod bash_abort;
pub mod bash_output;
pub mod bash_process;
pub mod bash_streams;
pub mod blocklist;
pub mod bridge;
pub mod bridge_multiplexed;
pub mod bridge_pty;
pub mod dart_lang;
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
pub mod graph_search;
pub mod grep;
pub mod image_dimensions;
pub mod limits;
pub mod ls;
pub mod mcp;
pub mod page_fetcher;
pub mod pdf;
pub mod profile;
pub mod read;
pub mod request_user_input;
pub mod schedule;
pub mod serde_coerce;
pub mod search_engine;
pub mod tool_pause;
pub mod tool_progress;
pub mod pre_tool_hook;
pub mod truncation;
pub mod validation;
pub mod unicode_path;
pub mod unified_exec;
pub mod web_search;
pub mod session_search;
pub mod session_registry;
pub mod write;

// Test fixtures for integration tests
#[cfg(test)]
pub mod bridge_test_fixtures;

// Integration tests using real WebSocket fixtures
#[cfg(test)]
mod bridge_integration_tests;

// Multiplexed wiring tests (ARCH-004)
#[cfg(test)]
mod bridge_multiplexed_wiring_tests;

// Subordinate session relay tests (SESS-015)
#[cfg(test)]
mod subordinate_relay_tests;

pub use error::ToolError;

pub use agent_manager::{
    AgentManagerTool, AgentManagerHandler, AgentManagerAsyncHandler,
    AgentManagerResult, AgentManagerAction,
    AwaitOutcome, AwaitSessionResult, SessionIdParam,
    SessionEntry, SessionStatus,
    set_agent_manager_handler, set_agent_manager_async_handler,
    has_agent_manager_handler, clear_all_agent_manager_handlers,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use astgrep::AstGrepTool;
pub use astgrep_refactor::AstGrepRefactorTool;
pub use apply_patch::ApplyPatchTool;
pub use bash::BashTool;
pub use bash_abort::{clear_bash_abort, is_bash_abort_requested, request_bash_abort, unregister_bash_abort_flag};
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
    get_bridge_session_context,
    BridgeHandler, BridgeRequest, BridgeSessionContext, BroadcastReceiverFactory,
};
pub use bridge_relay::{spawn_relay_task, InputInjector, InjectedInput, ImageData, ControlHandler, CommandEmitter,
    handle_multiplexed_inbound, process_outbound_envelope, get_instance_metadata, OutboundEnvelopeAction,
    set_session_list_provider, set_model_info_provider, broadcast_metadata_update,
    SessionListProvider, ModelInfoProvider, OutboundControlTx,
    SubordinateChunkTx, get_subordinate_chunk_senders,
    SessionCreator, set_session_creator, set_pty_registry};
pub use bridge_multiplexed::{
    Envelope, Service as MultiplexedService, InstanceMetadata,
    InboundAction, route_inbound, is_multiplexed_endpoint,
};
pub use bridge_pty::{
    PtyRegistry, PtyEntry, CreateTerminalOpts,
    create_terminal, resize_terminal, write_terminal_input, destroy_terminal,
};
pub use chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
pub use edit::EditTool;
pub use fspec::FspecTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use unified_exec::{
    UnifiedExecTool, UnifiedExecArgs, UnifiedExecResult,
    ProcessStore, ExecCommand, session_id_to_evict,
    MIN_YIELD_TIME_MS, MAX_YIELD_TIME_MS, DEFAULT_YIELD_TIME_MS,
    MIN_EMPTY_YIELD_TIME_MS, MAX_UNIFIED_EXEC_PROCESSES,
    UNIFIED_EXEC_OUTPUT_MAX_BYTES, LRU_PROTECT_COUNT,
    clamp_yield_time, clamp_poll_yield_time,
};
pub use page_fetcher::{Heading, Link, PageContent, PageFetcher};
pub use read::{ReadOutput, ReadTool};
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
pub use pre_tool_hook::{
    pre_tool_hook_check, register_pre_tool_hook, unregister_pre_tool_hook,
    PreToolHookDecision, PreToolHookHandler,
};
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
    DEFAULT_DEEP_SEARCH_MAX_DEPTH, DEFAULT_MAX_RECURSION_DEPTH,
    SUB_AGENT_TOOL_NAMES, SUB_AGENT_TOOL_COUNT,
    build_system_prompt, sub_agent_tool_names,
    set_deep_search_handler, has_deep_search_handler, clear_all_deep_search_handlers,
};
pub use request_user_input::{
    RequestUserInputTool, RequestUserInputArgs,
    HitlHandler, HitlRequest, HitlResponse, HitlQuestion, HitlOption, HitlAnswer,
    set_hitl_handler, has_hitl_handler, execute_hitl, clear_all_hitl_handlers,
};
pub use schedule::{
    ScheduleTool, ScheduleArgs, ScheduleHandler, ScheduleResult,
    set_schedule_handler, has_schedule_handler, clear_all_schedule_handlers,
};
pub use schedule::types::ScheduleRequest;
pub use graph_search::{
    GraphSearchTool, GraphSearchHandler, GraphSearchAction, GraphSearchArgs,
    set_graph_search_handler, has_graph_search_handler,
    execute_graph_search, clear_all_graph_search_handlers,
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
