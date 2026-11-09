/// MCP-008: spawn the child process, perform the MCP initialize handshake
/// and discover tools, without caching the connection (see `connect_stdio`).
pub async fn establish_stdio(
    name: &str,
    command: &stdio_command::McpStdioCommand,
    env: Option<&HashMap<String, String>>,
    timeout_secs: u64,
    injection_tx: McpInjectionTx,
    connections: McpConnectionMap,
) -> Result<McpEstablished, McpConnectResult> {
    use rmcp::transport::ConfigureCommandExt;
    use tokio::process::Command;

    if command.program.is_empty() {
        return Err(McpConnectResult {
            success: false,
            message: format!("✗ Failed to connect: {name}\n  Empty command"),
            tools: None,
            connections: None,
        });
    }

    // MCP-005: the program and every argument go to the process literally.
    let program = command.program.as_str();
    let args = &command.args;

    // Build the transport
    let transport_result =
        rmcp::transport::TokioChildProcess::new(Command::new(program).configure(|cmd| {
            cmd.args(args);
            if let Some(env_vars) = env {
                cmd.envs(env_vars.iter());
            }
        }));

    let transport = match transport_result {
        Ok(t) => t,
        Err(e) => {
            return Err(McpConnectResult {
                success: false,
                message: format!(
                    "✗ Failed to connect: {name}\n  {e}\n\n  Hint: Check that '{program}' is installed and in PATH."
                ),
                tools: None,
                connections: None,
            });
        }
    };

    // Create handler and serve with timeout
    let handler = DynMcpHandler::new(name.to_string(), injection_tx, connections.clone());

    let service_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        handler.serve(transport),
    )
    .await;

    let service = match service_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Err(McpConnectResult {
                success: false,
                message: format!("✗ Failed to connect: {name}\n  MCP handshake failed: {e}"),
                tools: None,
                connections: None,
            });
        }
        Err(_) => {
            return Err(McpConnectResult {
                success: false,
                message: format!(
                    "✗ Failed to connect: {name} — Timeout: MCP handshake not completed within {timeout_secs}s. Process started but did not respond."
                ),
                tools: None,
                connections: None,
            });
        }
    };

    // Discover tools
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

    // Get server info
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

    // Build success message
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

    Ok(McpEstablished {
        connection: McpConnection {
            name: name.to_string(),
            service,
            tools: tool_defs,
            raw_tools: tools,
            server_info,
            connected_at: Instant::now(),
            call_count: 0,
            transport: McpTransport::Stdio,
            configured_transport: None,
        },
        message,
    })
}

/// Route an MCP tool call through the cached connection.
///
/// Parses the qualified name (mcp__<server>__<tool>), looks up the connection,
/// and forwards the call via rmcp's peer().call_tool().
pub async fn route_mcp_tool_call(
    qualified_name: &str,
    arguments: serde_json::Value,
    connections: &McpConnectionMap,
) -> Result<String, String> {
    let (server, tool) = parse_mcp_tool_name(qualified_name)
        .ok_or_else(|| format!("Invalid MCP tool name: {qualified_name}"))?;

    let mut map = connections.write().await;
    let conn = map
        .get_mut(server)
        .ok_or_else(|| format!("MCP server '{server}' is not connected"))?;

    // Increment call count
    conn.call_count += 1;

    let arguments_map: Option<serde_json::Map<String, serde_json::Value>> =
        if let serde_json::Value::Object(m) = arguments {
            Some(m)
        } else {
            None
        };

    let tool_name = tool.to_string();
    let call_params = if let Some(args) = arguments_map {
        CallToolRequestParams::new(tool_name).with_arguments(args)
    } else {
        CallToolRequestParams::new(tool_name)
    };

    let result = conn
        .service
        .peer()
        .call_tool(call_params)
        .await
        .map_err(|e| format!("MCP tool call failed: {e}"))?;

    // Check for error response
    if let Some(true) = result.is_error {
        let error_text: String = result
            .content
            .iter()
            .filter_map(|c| c.raw.as_text().map(|t| t.text.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("MCP tool error: {error_text}"));
    }

    // Combine content into output string
    let output: String = result
        .content
        .iter()
        .filter_map(|c| c.raw.as_text().map(|t| t.text.to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(output)
}

/// Disconnect a named MCP connection.
pub async fn disconnect_mcp(name: &str, connections: &McpConnectionMap) -> McpConnectResult {
    let mut map = connections.write().await;
    match map.remove(name) {
        Some(conn) => {
            let uptime = format_uptime(conn.connected_at);
            conn.service.cancellation_token().cancel();
            McpConnectResult {
                success: true,
                message: format!(
                    "✓ Disconnected: {name} (was connected {uptime}, {} tool calls made)",
                    conn.call_count
                ),
                tools: None,
                connections: None,
            }
        }
        None => McpConnectResult {
            success: false,
            message: format!("✗ No connection named '{name}' found"),
            tools: None,
            connections: None,
        },
    }
}

/// Parse a qualified MCP tool name (mcp__<server>__<tool>) into (server, tool).
///
/// Returns None if the name doesn't match the expected format.
pub fn parse_mcp_tool_name(qualified_name: &str) -> Option<(&str, &str)> {
    let stripped = qualified_name.strip_prefix("mcp__")?;
    let (server, tool) = stripped.split_once("__")?;
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server, tool))
}
