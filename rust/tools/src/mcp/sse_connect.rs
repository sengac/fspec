/// MCP-006: connect to an MCP server over the legacy HTTP+SSE transport.
///
/// Opens the event stream with the configured headers, waits for its endpoint
/// and performs the MCP handshake, all within `timeout_secs`; then discovers
/// tools like `establish_http`, without caching (see `connect_sse`). SSE is
/// never retried as Streamable HTTP.
pub async fn establish_sse(
    name: &str,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> Result<McpEstablished, McpConnectResult> {
    let failed = |reason: String| {
        Err(McpConnectResult {
            success: false,
            message: format!("✗ Failed to connect: {name}\n  {reason}"),
            tools: None,
            connections: None,
        })
    };
    let custom_headers = match http_headers::parse_mcp_http_headers(headers.into_iter().flatten()) {
        Ok(parsed) => parsed,
        Err(e) => return failed(e.to_string()),
    };
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let started = Instant::now();
    let transport = match tokio::time::timeout(
        timeout,
        sse_transport::LegacySseTransport::connect(url, custom_headers),
    )
    .await
    {
        Ok(Ok(transport)) => transport,
        Ok(Err(e)) => return failed(format!("Legacy SSE connection failed: {e}")),
        Err(_) => {
            return failed(format!(
                "Timeout: the SSE endpoint event did not arrive within {timeout_secs}s."
            ))
        }
    };
    serve_and_discover(
        name,
        transport,
        McpTransport::Sse,
        timeout_secs,
        timeout.saturating_sub(started.elapsed()),
        injection_tx,
        connections,
    )
    .await
}
