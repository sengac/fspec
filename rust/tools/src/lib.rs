//! Tool Execution bounded context
//!
//! File operations, code search, bash execution.
//! All tools implement rig::tool::Tool trait for use with RigAgent.
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS

pub mod agent_manager;
pub mod apply_patch;
pub mod astgrep;
pub mod astgrep_refactor;
pub mod bash;
pub mod bash_abort;
pub mod bash_binary_guard;
pub mod bash_output;
pub mod bash_process;
#[cfg(windows)]
pub mod bash_process_windows;
pub mod bash_streams;
pub mod blocklist;
pub mod bridge;
pub mod bridge_handler;
pub mod bridge_multiplexed;
pub mod bridge_pty;
pub mod bridge_relay;
pub mod chrome_browser;
pub mod dart_lang;
pub mod deep_search;
pub mod done; // CONT-002: auto-continue done() tool + armed/acceptance registries
pub mod edit;
pub mod error;
pub mod facade;
pub mod file_type;
pub mod fspec;
pub mod fspec_handler;
pub mod fspec_workflow_guidance;
pub mod inject_summary;
pub mod stage_permissions;

pub mod footer_cwd;
pub mod glob;
pub mod graph_search;
pub mod grep;
pub mod image_dimensions;
pub mod limits;
pub mod ls;
pub mod mcp;
pub mod page_fetcher;
pub mod pdf;
pub mod pre_tool_hook;
pub mod profile;
pub mod read;
pub mod request_user_input;
pub mod schedule;
pub mod search_engine;
pub mod serde_coerce;
pub mod session_registry;
pub mod session_search;
pub mod tool_pause;
pub mod tool_progress;
pub mod truncation;
pub mod unicode_path;
pub mod unified_exec;
pub mod validation;
pub mod web_search;
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
    clear_all_agent_manager_handlers, has_agent_manager_handler, set_agent_manager_async_handler,
    set_agent_manager_handler, AgentManagerAction, AgentManagerAsyncHandler, AgentManagerHandler,
    AgentManagerResult, AgentManagerTool, AwaitOutcome, AwaitSessionResult, SessionEntry,
    SessionIdParam, SessionStatus,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use apply_patch::ApplyPatchTool;
pub use astgrep::AstGrepTool;
pub use astgrep_refactor::AstGrepRefactorTool;
pub use bash::BashTool;
pub use bash_abort::{
    clear_bash_abort, is_bash_abort_requested, request_bash_abort, unregister_bash_abort_flag,
};
pub use blocklist::{
    check_bash_command, check_command_raw, check_file_path, init_blocklist, load_blocklist_config,
    project_config_path, reload_blocklist, system_config_path, BlockedError, BlocklistAction,
    BlocklistConfig, BlocklistMatcher, BlocklistRule, CheckResult,
};
pub use bridge::{
    get_or_create_bridge_manager, remove_bridge_manager, BridgeAction, BridgeConnectionInfo,
    BridgeConnectionState, BridgeManager, BridgeResult, BridgeTool, BridgeToolArgs,
};
pub use bridge_handler::{
    execute_bridge_command, get_bridge_session_context, handle_bridge_action,
    has_bridge_handler_for_session, remove_bridge_session_context, set_bridge_handler,
    set_bridge_session_context, BridgeHandler, BridgeRequest, BridgeSessionContext,
    BroadcastReceiverFactory,
};
pub use bridge_multiplexed::{
    is_multiplexed_endpoint, route_inbound, Envelope, InboundAction, InstanceMetadata,
    Service as MultiplexedService,
};
pub use bridge_pty::{
    create_terminal, destroy_terminal, resize_terminal, write_terminal_input, CreateTerminalOpts,
    PtyEntry, PtyRegistry,
};
pub use bridge_relay::{
    broadcast_metadata_update, get_instance_metadata, get_subordinate_chunk_senders,
    handle_multiplexed_inbound, process_outbound_envelope, set_model_info_provider,
    set_pty_registry, set_session_creator, set_session_list_provider, spawn_relay_task,
    CommandEmitter, ControlHandler, ImageData, InjectedInput, InputInjector, ModelInfoProvider,
    OutboundControlTx, OutboundEnvelopeAction, SessionCreator, SessionListProvider,
    SubordinateChunkTx,
};
pub use chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
pub use deep_search::{
    build_system_prompt, clear_all_deep_search_handlers, has_deep_search_handler,
    set_deep_search_handler, split_scope, sub_agent_tool_names, DeepSearchArgs, DeepSearchHandler,
    DeepSearchTool, DEFAULT_DEEP_SEARCH_MAX_DEPTH, DEFAULT_MAX_RECURSION_DEPTH,
    SUB_AGENT_TOOL_COUNT, SUB_AGENT_TOOL_NAMES,
};
pub use done::{
    clear_done_acceptance, done_rejection_count, get_session_goal, is_continue_armed,
    set_continue_armed, set_session_goal, set_verify_timeout_for_tests, take_done_acceptance,
    DoneArgs, DoneTool, GoalSpec, DONE_TOOL_NAME,
};
pub use edit::EditTool;
pub use footer_cwd::{get_footer_cwd, unregister_footer_cwd, update_footer_cwd};
pub use fspec::FspecTool;
pub use fspec_handler::{
    clear_all_fspec_handlers, execute_fspec_command_for_session, has_fspec_handler_for_session,
    set_fspec_handler_for_session, FspecHandler, FspecRequest as FspecHandlerRequest,
    FspecResult as FspecHandlerResult,
};
pub use fspec_workflow_guidance::{get_fspec_workflow_guidance, FSPEC_WORKFLOW_GUIDANCE};
pub use glob::GlobTool;
pub use graph_search::{
    clear_all_graph_search_handlers, execute_graph_search, has_graph_search_handler,
    set_graph_search_handler, GraphSearchAction, GraphSearchArgs, GraphSearchHandler,
    GraphSearchTool,
};
pub use grep::GrepTool;
pub use inject_summary::{
    clear_all_inject_summary_handlers, execute_inject_summary, has_inject_summary_handler,
    set_inject_summary_handler, InjectSummaryHandler, InjectSummaryResult, InjectSummaryTool,
};
pub use ls::LsTool;
pub use mcp::{
    cleanup_mcp_session, connect_http, connect_stdio, disconnect_mcp,
    gather_mcp_tool_registrations, gather_mcp_tool_wrappers, get_mcp_connections, init_mcp_session,
    new_mcp_connection_map, parse_mcp_tool_name, qualified_mcp_tool_name, route_mcp_tool_call,
    set_mcp_tool_server_handle, ConnectMcpTool, DynMcpHandler, McpAction, McpConnectArgs,
    McpConnectResult, McpConnection, McpConnectionMap, McpConnectionSummary, McpInjection,
    McpInjectionTx, McpServerInfo, McpToolDef, McpToolRegistration, McpToolWrapper, McpTransport,
};
pub use page_fetcher::{Heading, Link, PageContent, PageFetcher};
pub use pre_tool_hook::{
    pre_tool_hook_check, register_pre_tool_hook, unregister_pre_tool_hook, PreToolHookDecision,
    PreToolHookHandler,
};
pub use read::{ReadOutput, ReadTool};
pub use request_user_input::{
    clear_all_hitl_handlers, execute_hitl, has_hitl_handler, set_hitl_handler, HitlAnswer,
    HitlHandler, HitlOption, HitlQuestion, HitlRequest, HitlResponse, RequestUserInputArgs,
    RequestUserInputTool,
};
pub use schedule::types::ScheduleRequest;
pub use schedule::{
    clear_all_schedule_handlers, has_schedule_handler, set_schedule_handler, ScheduleArgs,
    ScheduleHandler, ScheduleResult, ScheduleTool,
};
pub use search_engine::{SearchEngine, SearchResult};
pub use session_search::{
    clear_all_session_search_handlers, has_session_search_handler, set_session_search_handler,
    SessionSearchHandler, SessionSearchTool,
};
pub use stage_permissions::{
    check_write_permission, check_write_raw, init_stage_permissions, load_stage_permissions_config,
    project_config_path as stage_permissions_project_config_path, reload_stage_permissions,
    system_config_path as stage_permissions_system_config_path, FileCategory, StageBlockedError,
    StageCheckResult, StagePermission, StagePermissionsConfig, StagePermissionsMatcher,
};
pub use tool_pause::{
    has_pause_handler, pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse, PauseState,
};
pub use tool_progress::{emit_tool_progress, set_tool_progress_callback, ToolProgressCallback};
pub use unified_exec::{
    clamp_poll_yield_time, clamp_yield_time, session_id_to_evict, ExecCommand, ProcessStore,
    UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool, DEFAULT_YIELD_TIME_MS, LRU_PROTECT_COUNT,
    MAX_UNIFIED_EXEC_PROCESSES, MAX_YIELD_TIME_MS, MIN_EMPTY_YIELD_TIME_MS, MIN_YIELD_TIME_MS,
    UNIFIED_EXEC_OUTPUT_MAX_BYTES,
};
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
