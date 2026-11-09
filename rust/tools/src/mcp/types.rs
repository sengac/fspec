/// MCP-008: fspec's one MCP handshake timeout, in seconds. ConnectMCP and the
/// daemon path (`host_config`) both connect under it; nobody else sets one.
pub const MCP_HANDSHAKE_TIMEOUT_SECS: u64 = 10;

/// An active MCP server connection cached on the session.
///
/// Holds the rmcp RunningService, cached tool list, server metadata,
/// and connection statistics. Ephemeral — lives only as long as the session.
pub struct McpConnection {
    /// Server name (user-provided identifier)
    pub name: String,
    /// The running rmcp service for this connection
    pub service: RunningService<RoleClient, DynMcpHandler>,
    /// Cached tool definitions from tools/list (simplified for serialization)
    pub tools: Vec<McpToolDef>,
    /// Original rmcp tool definitions from tools/list (used for agent building)
    pub raw_tools: Vec<rmcp::model::Tool>,
    /// Server info from initialize handshake
    pub server_info: McpServerInfo,
    /// When this connection was established
    pub connected_at: Instant,
    /// Number of tool calls routed through this connection
    pub call_count: u32,
    /// Transport type used
    pub transport: McpTransport,
    /// MCP-008: the daemon-configured transport this connection was made
    /// with, so an identical reconfiguration keeps it; `None` when ConnectMCP
    /// made it.
    pub configured_transport: Option<host_config::HostMcpTransport>,
}

/// Simplified tool definition from MCP server's tools/list response.
/// Maps to rmcp::model::Tool fields we need for tool registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    /// Tool name as reported by the MCP server
    pub name: String,
    /// Optional human-readable description
    pub description: Option<String>,
    /// JSON Schema for tool input parameters
    pub input_schema: serde_json::Value,
}

fn validate_mcp_tool_definitions(
    server_name: &str,
    tools: &[McpToolDef],
) -> Result<(), crate::tool_attribution::AttributionSchemaError> {
    for tool in tools {
        crate::tool_attribution::decorate_definition(rig::completion::ToolDefinition {
            name: qualified_mcp_tool_name(server_name, &tool.name),
            description: tool.description.clone().unwrap_or_default(),
            parameters: tool.input_schema.clone(),
        })?;
    }
    Ok(())
}

/// Server information from the MCP initialize handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Negotiated protocol version
    pub protocol_version: String,
}

/// Transport type for MCP connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    /// Local stdio transport — spawns child process
    Stdio,
    /// Remote HTTP transport — connects to URL
    Http,
    /// MCP-006: legacy HTTP+SSE transport — GET event stream, POST messages
    Sse,
}

/// Actions supported by the ConnectMCP tool.
/// MCP-011: there is no default action — every call states one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpAction {
    /// Establish a new MCP server connection
    Connect,
    /// Tear down a named connection
    Disconnect,
    /// List all active connections
    List,
}

/// Arguments for the ConnectMCP tool call.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpConnectArgs {
    /// Action to perform (MCP-011: required)
    pub action: McpAction,
    /// Server name identifier
    pub name: Option<String>,
    /// Transport type (required for connect: stdio, http or sse; MCP-009)
    pub transport: Option<McpTransport>,
    /// Executable to spawn (stdio transport, required and non-empty): a name
    /// on PATH or a path, used literally and never split on whitespace.
    pub command: Option<String>,
    /// Literal argument list for `command` (stdio transport, required; `[]`
    /// for none): each entry is one argument, never split or shell-interpreted.
    pub args: Option<Vec<String>>,
    /// Server URL (http and sse transports)
    pub url: Option<String>,
    /// Environment variables for the subprocess
    pub env: Option<HashMap<String, String>>,
    /// HTTP headers (http and sse transports)
    pub headers: Option<HashMap<String, String>>,
}

/// Result from ConnectMCP tool execution.
#[derive(Debug, Serialize)]
pub struct McpConnectResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message for the LLM
    pub message: String,
    /// Discovered tools (on successful connect)
    pub tools: Option<Vec<McpToolDef>>,
    /// Connection listing (on list action)
    pub connections: Option<Vec<McpConnectionSummary>>,
}

/// Summary of an active connection (for list action).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionSummary {
    /// Server name
    pub name: String,
    /// How long the connection has been active (human-readable)
    pub uptime: String,
    /// Number of tools available
    pub tool_count: usize,
    /// Number of tool calls made
    pub call_count: u32,
    /// Transport type
    pub transport: McpTransport,
}

/// Thread-safe container for MCP connections on a session.
pub type McpConnectionMap = Arc<RwLock<HashMap<String, McpConnection>>>;

/// Create a new empty MCP connection map for a session.
pub fn new_mcp_connection_map() -> McpConnectionMap {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Message sent to inject MCP events into the session's agent loop.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum McpInjection {
    /// Notification text to display to the LLM
    Notification(String),
    /// Sampling request requiring LLM response (with oneshot return channel)
    SamplingRequest {
        params: CreateMessageRequestParams,
        response_tx: tokio::sync::oneshot::Sender<Result<CreateMessageResult, String>>,
    },
}

/// Sender for injecting MCP events into the session.
pub type McpInjectionTx = mpsc::Sender<McpInjection>;

/// ClientHandler implementation for Dynamic MCP.
///
/// Handles server-initiated messages by injecting them into the session
/// via the injection channel. Each MCP connection gets its own handler instance.
pub struct DynMcpHandler {
    /// Server name for log/notification prefixing
    name: String,
    /// Channel to inject messages into the session
    injection_tx: McpInjectionTx,
    /// Shared mutable reference to the connection map for tool cache updates
    connections: McpConnectionMap,
}

impl DynMcpHandler {
    /// Create a new handler for a named MCP connection.
    pub fn new(name: String, injection_tx: McpInjectionTx, connections: McpConnectionMap) -> Self {
        Self {
            name,
            injection_tx,
            connections,
        }
    }
}
