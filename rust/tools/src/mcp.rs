//! Dynamic MCP: Tool-Driven MCP Integration via ConnectMCP
//!
//! Feature: spec/features/dynamic-mcp-connect.feature
//!
//! Implements ConnectMCP tool that establishes MCP server connections at runtime.
//! The agent decides when to connect, receives structured feedback, and gains
//! typed MCP tools dynamically mid-session via the rmcp crate.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use rig::tool::server::ToolServerHandle;
use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, ClientInfo, CreateMessageRequestParams,
    CreateMessageResult, Implementation, LoggingMessageNotificationParam,
    ResourceUpdatedNotificationParam,
};
use rmcp::service::{NotificationContext, RequestContext, RoleClient, RunningService};
use rmcp::ServiceExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, warn};

include!("mcp/types.rs");
include!("mcp/client_handler.rs");
include!("mcp/stdio_and_routing.rs");
include!("mcp/http_and_session_registry.rs");
include!("mcp/remote_connect.rs");
include!("mcp/sse_connect.rs");
include!("mcp/establish.rs");
include!("mcp/tool_wrappers.rs");
include!("mcp/handle_registration.rs");
include!("mcp/connect_tool.rs");

pub mod host_config;
pub mod http_headers;
pub mod sse_transport;
pub mod stdio_command;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod tests {
    use super::*;
    include!("mcp/tests_01.rs");
    include!("mcp/tests_02.rs");
    include!("mcp/tests_03.rs");
    include!("mcp/tests_04.rs");
    // ===================================================================
    // Additional unit tests for helper functions
    // ===================================================================
    mod helper_tests {
        use super::*;
        use rig::tool::Tool;
        include!("mcp/helper_tests_01.rs");
        include!("mcp/helper_tests_02.rs");
        include!("mcp/helper_tests_03.rs");
        include!("mcp/helper_tests_04.rs");
        include!("mcp/helper_tests_05.rs");
    }
}
