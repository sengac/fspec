// MCP-006: the tail shared by the remote (Streamable HTTP and legacy SSE)
// transports — serve within the timeout and discover tools (MCP-008: the
// caller caches the connection).

/// Serve `transport` within `remaining`, then discover its tools for a
/// connection of `kind`. `timeout_secs` is the handshake timeout, for messages.
async fn serve_and_discover<T, E, A>(
    name: &str,
    transport: T,
    kind: McpTransport,
    timeout_secs: u64,
    remaining: std::time::Duration,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> Result<McpEstablished, McpConnectResult>
where
    T: rmcp::transport::IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let handler = DynMcpHandler::new(name.to_string(), injection_tx, connections.clone());

    let service_result = tokio::time::timeout(remaining, handler.serve(transport)).await;

    let service = match service_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let err_str = format!("{e}");
            let hint = if err_str.contains("401") || err_str.contains("Unauthorized") {
                "\n  Server requires authentication."
            } else {
                ""
            };
            return Err(McpConnectResult {
                success: false,
                message: format!("✗ Failed to connect: {name}\n  MCP handshake failed: {e}{hint}"),
                tools: None,
                connections: None,
            });
        }
        Err(_) => {
            return Err(McpConnectResult {
                success: false,
                message: format!(
                    "✗ Failed to connect: {name} — Timeout: MCP handshake not completed within {timeout_secs}s."
                ),
                tools: None,
                connections: None,
            });
        }
    };

    // Discover tools and cache connection (same logic as stdio)
    // MCP-012: tool discovery is bounded like the handshake, and a failed or
    // unanswered tools/list fails the connect (dropping the service).
    let tools = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        service.peer().list_all_tools(),
    )
    .await
    {
        Ok(Ok(tools)) => tools,
        Ok(Err(e)) => {
            return Err(McpConnectResult {
                success: false,
                message: format!(
                    "✗ Failed to connect: {name}\n  MCP tool discovery (tools/list) failed: {e}"
                ),
                tools: None,
                connections: None,
            });
        }
        Err(_) => {
            return Err(McpConnectResult {
                success: false,
                message: format!(
                    "✗ Failed to connect: {name} — Timeout: MCP tool discovery (tools/list) not completed within {timeout_secs}s."
                ),
                tools: None,
                connections: None,
            });
        }
    };

    let tool_defs: Vec<McpToolDef> = tools
        .iter()
        .map(|t| McpToolDef {
            name: t.name.to_string(),
            description: t.description.as_ref().map(std::string::ToString::to_string),
            input_schema: t.schema_as_json_value(),
        })
        .collect();

    let server_info = if let Some(info) = service.peer().peer_info() {
        McpServerInfo {
            name: info.server_info.name.clone(),
            version: info.server_info.version.clone(),
            protocol_version: info.protocol_version.as_str().to_string(),
        }
    } else {
        McpServerInfo {
            name: name.to_string(),
            version: "unknown".to_string(),
            protocol_version: "2025-11-25".to_string(),
        }
    };

    let tool_list = tool_defs
        .iter()
        .map(|t| {
            let desc = t.description.as_deref().unwrap_or("No description");
            format!("    - {} — {desc}", t.name)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let message = format!(
        "✓ Connected: {name} (MCP {})\n  Server: {} v{}\n  Tools ({}):\n{tool_list}",
        server_info.protocol_version,
        server_info.name,
        server_info.version,
        tool_defs.len(),
    );

    if let Err(error) = validate_mcp_tool_definitions(name, &tool_defs) {
        service.cancellation_token().cancel();
        return Err(McpConnectResult {
            success: false,
            message: format!(
                "✗ Failed to connect: {name}\n  Tool schema registration rejected: {error}"
            ),
            tools: None,
            connections: None,
        });
    }

    Ok(McpEstablished {
        connection: McpConnection {
            name: name.to_string(),
            service,
            tools: tool_defs,
            raw_tools: tools,
            server_info,
            connected_at: Instant::now(),
            call_count: 0,
            transport: kind,
            configured_transport: None,
        },
        message,
    })
}
