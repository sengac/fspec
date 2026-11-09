/// Build the qualified tool name from server and tool names.
pub fn qualified_mcp_tool_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// Gather all MCP tool definitions from active connections, with namespaced names.
pub fn gather_mcp_tools(connections: &HashMap<String, McpConnection>) -> Vec<McpToolDef> {
    let mut tools = Vec::new();
    for conn in connections.values() {
        for tool in &conn.tools {
            tools.push(McpToolDef {
                name: qualified_mcp_tool_name(&conn.name, &tool.name),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            });
        }
    }
    tools
}

/// Format connection uptime as human-readable string.
fn format_uptime(connected_at: Instant) -> String {
    let elapsed = connected_at.elapsed();
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

/// Get a summary of all active connections.
pub fn list_connections(connections: &HashMap<String, McpConnection>) -> Vec<McpConnectionSummary> {
    connections
        .values()
        .map(|conn| McpConnectionSummary {
            name: conn.name.clone(),
            uptime: format_uptime(conn.connected_at),
            tool_count: conn.tools.len(),
            call_count: conn.call_count,
            transport: conn.transport.clone(),
        })
        .collect()
}

// =============================================================================
// HTTP Transport Connection
// =============================================================================

/// MCP-008: create a StreamableHttpClientTransport, perform the MCP initialize
/// handshake and discover tools, without caching (see `connect_http`).
pub async fn establish_http(
    name: &str,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> Result<McpEstablished, McpConnectResult> {
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
    use rmcp::transport::StreamableHttpClientTransport;

    // MCP-004: every configured header goes on every request of this connection;
    // rmcp itself refuses reserved MCP headers (ReservedHeaderConflict).
    let custom_headers = match http_headers::parse_mcp_http_headers(headers.into_iter().flatten()) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(McpConnectResult {
                success: false,
                message: format!("✗ Failed to connect: {name}\n  {e}"),
                tools: None,
                connections: None,
            });
        }
    };
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url).custom_headers(custom_headers),
    );

    serve_and_discover(
        name,
        transport,
        McpTransport::Http,
        timeout_secs,
        std::time::Duration::from_secs(timeout_secs),
        injection_tx,
        connections,
    )
    .await
}

// =============================================================================
// Global Per-Session MCP State
// =============================================================================

/// Per-session MCP state: connection map + injection sender + optional tool server handle.
struct McpSessionState {
    connections: McpConnectionMap,
    injection_tx: McpInjectionTx,
    /// MCP-002: Handle to the running agent's tool server for mid-turn tool registration.
    /// Set after agent build in run_with_provider! macro. None before agent starts.
    tool_server_handle: Option<ToolServerHandle>,
}

/// Global registry of MCP state keyed by session UUID.
/// Uses std::sync::Mutex (not tokio) because:
/// 1. We never hold the lock across .await points
/// 2. tokio::sync::Mutex doesn't work across different tokio runtimes (test isolation)
static MCP_SESSIONS: once_cell::sync::Lazy<std::sync::Mutex<HashMap<uuid::Uuid, McpSessionState>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

/// Initialize MCP state for a session. Returns the injection receiver
/// for the agent_loop to consume, and the McpConnectionMap for gathering tools.
pub fn init_mcp_session(
    session_id: uuid::Uuid,
) -> (mpsc::Receiver<McpInjection>, McpConnectionMap) {
    let (injection_tx, injection_rx) = mpsc::channel::<McpInjection>(32);
    let connections = new_mcp_connection_map();
    let state = McpSessionState {
        connections: connections.clone(),
        injection_tx,
        tool_server_handle: None,
    };
    let mut sessions = MCP_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    sessions.insert(session_id, state);
    (injection_rx, connections)
}

/// Remove MCP state for a session, cancelling all connections.
///
/// Removing from the global map drops our reference. The `McpConnectionMap` is
/// `Arc<RwLock<HashMap>>` — other holders (like active tool calls) may still
/// have references. We explicitly cancel each `RunningService` so child
/// processes are killed immediately rather than waiting for all `Arc` refs
/// to drop.
pub fn cleanup_mcp_session(session_id: uuid::Uuid) {
    let state = {
        let mut sessions = MCP_SESSIONS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions.remove(&session_id)
    };
    // Cancel each connection's service synchronously via the cancellation token.
    // This is non-async — cancellation_token().cancel() is a sync operation that
    // signals the RunningService to shut down.
    if let Some(state) = state {
        // try_write to avoid blocking if someone else holds the lock.
        // If we can't get the write lock, the connections will be cancelled
        // when the last Arc reference is dropped anyway.
        if let Ok(mut map) = state.connections.try_write() {
            for (_, conn) in map.drain() {
                conn.service.cancellation_token().cancel();
            }
        }
    }
}

/// Get the McpConnectionMap for a session (for gathering tools at agent build time).
pub fn get_mcp_connections(session_id: uuid::Uuid) -> Option<McpConnectionMap> {
    let sessions = MCP_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    sessions.get(&session_id).map(|s| s.connections.clone())
}
