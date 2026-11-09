// MCP-008: connecting is two steps — establish (handshake and discovery, no
// cache) and cache — so a daemon reconfiguration can establish every new and
// changed server before it commits any of them.

/// A handshaken connection with its tools discovered, not yet cached.
pub struct McpEstablished {
    /// The connection to cache.
    pub connection: McpConnection,
    /// The success message for the LLM.
    pub message: String,
}

/// Cache an established connection, replacing (and cancelling) any connection
/// with the same name.
pub async fn cache_connection(
    connections: &McpConnectionMap,
    established: McpEstablished,
) -> McpConnectResult {
    let tools = established.connection.tools.clone();
    let name = established.connection.name.clone();
    let replaced = connections
        .write()
        .await
        .insert(name, established.connection);
    if let Some(old) = replaced {
        old.service.cancellation_token().cancel();
    }
    McpConnectResult {
        success: true,
        message: established.message,
        tools: Some(tools),
        connections: None,
    }
}

async fn establish_and_cache(
    established: Result<McpEstablished, McpConnectResult>,
    connections: &McpConnectionMap,
) -> McpConnectResult {
    match established {
        Ok(established) => cache_connection(connections, established).await,
        Err(failed) => failed,
    }
}

/// Connect to an MCP server via stdio transport: spawn the child process,
/// perform the MCP initialize handshake, discover tools, and cache the
/// connection.
pub async fn connect_stdio(
    name: &str,
    command: &stdio_command::McpStdioCommand,
    env: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> McpConnectResult {
    let established =
        establish_stdio(name, command, env, timeout_secs, injection_tx, connections.clone()).await;
    establish_and_cache(established, &connections).await
}

/// Connect to an MCP server via Streamable HTTP and cache the connection.
pub async fn connect_http(
    name: &str,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> McpConnectResult {
    let established =
        establish_http(name, url, headers, timeout_secs, injection_tx, connections.clone()).await;
    establish_and_cache(established, &connections).await
}

/// MCP-006: connect to a legacy HTTP+SSE MCP server and cache the connection.
pub async fn connect_sse(
    name: &str,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> McpConnectResult {
    let established =
        establish_sse(name, url, headers, timeout_secs, injection_tx, connections.clone()).await;
    establish_and_cache(established, &connections).await
}
