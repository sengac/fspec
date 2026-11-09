/// MCP-002: Add a batch of McpToolWrapper instances to a ToolServerHandle.
///
/// Extracted from `ConnectMcpTool::register_new_tools_with_handle` so the core
/// registration logic is testable without requiring a live MCP server connection.
async fn add_wrappers_to_handle(
    handle: &ToolServerHandle,
    wrappers: Vec<McpToolWrapper>,
    server_name: &str,
    session_id: uuid::Uuid,
) -> Result<(), rig::tool::server::ToolServerError> {
    let count = wrappers.len();
    for wrapper in wrappers {
        if let Err(e) = handle.add_tool(wrapper).await {
            tracing::warn!("[MCP-002] Failed to add tool mid-turn: {}", e);
            return Err(e);
        }
    }
    tracing::info!(
        "[MCP-002] Registered {} tools from '{}' mid-turn for session {}",
        count,
        server_name,
        session_id,
    );
    Ok(())
}

/// MCP-002: Remove all tools from a server by name prefix from a ToolServerHandle.
///
/// Extracted from `ConnectMcpTool::remove_tools_from_handle` so the core
/// removal logic is testable without requiring a live MCP server connection.
async fn remove_server_tools_from_handle(
    handle: &ToolServerHandle,
    server_name: &str,
    session_id: uuid::Uuid,
) {
    let prefix = format!("mcp__{server_name}__");
    let defs = match handle.get_tool_defs(None).await {
        Ok(d) => d,
        Err(_) => return,
    };

    let tools_to_remove: Vec<String> = defs
        .iter()
        .filter(|d| d.name.starts_with(&prefix))
        .map(|d| d.name.clone())
        .collect();

    for name in &tools_to_remove {
        if let Err(e) = handle.remove_tool(name).await {
            tracing::warn!("[MCP-002] Failed to remove tool '{}': {}", name, e);
        }
    }

    if !tools_to_remove.is_empty() {
        tracing::info!(
            "[MCP-002] Removed {} tools from '{}' mid-turn for session {}",
            tools_to_remove.len(),
            server_name,
            session_id,
        );
    }
}
