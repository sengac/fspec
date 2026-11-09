#[tokio::test]
async fn test_connect_mcp_tool_connect_spawn_failure() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Connect,
        name: Some("broken".to_string()),
        transport: Some(McpTransport::Stdio),
        command: Some("__fspec_nonexistent_binary__".to_string()),
        args: Some(Vec::new()),
        url: None,
        env: None,
        headers: None,
    };
    // Spawn failure returns Ok(error_message) so LLM can reason about it
    let result = tool.call(args).await.unwrap();
    assert!(result.contains("Failed to connect"));

    cleanup_mcp_session(session_id);
}

#[test]
fn test_connect_mcp_tool_definition_name() {
    let tool = ConnectMcpTool::new(uuid::Uuid::new_v4());
    assert_eq!(<ConnectMcpTool as rig::tool::Tool>::NAME, "ConnectMCP");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let def = rig::tool::Tool::definition(&tool, "test".to_string()).await;
        assert_eq!(def.name, "ConnectMCP");
        assert!(def.description.contains("MCP"));
        assert!(def.parameters.is_object());
    });
}

// =================================================================
// MCP-002: Same-turn tool availability after ConnectMCP
// Feature: spec/features/mcp-same-turn-tool-availability.feature
// =================================================================

/// Helper: Insert synthetic tool definitions into the session's connection map.
///
/// Cannot construct McpConnection (requires RunningService), but we can
/// bypass that by testing the extracted pure functions directly:
/// - `McpToolWrapper::from_tool_def`
/// - `add_wrappers_to_handle`
/// - `remove_server_tools_from_handle`
fn make_tool_defs(names: &[&str]) -> Vec<McpToolDef> {
    names
        .iter()
        .map(|n| McpToolDef {
            name: n.to_string(),
            description: Some(format!("{n} tool")),
            input_schema: serde_json::json!({"type": "object"}),
        })
        .collect()
}

// -----------------------------------------------------------
// Scenario: ToolServerHandle is stored after agent build
// -----------------------------------------------------------
#[tokio::test]
async fn test_set_and_get_tool_server_handle() {
    // @step Given an MCP session is initialized
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    // @step When the run_with_provider macro builds an agent
    // Simulate: create a ToolServer, run it, and store the handle
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();

    // @step Then the agent's ToolServerHandle is stored in McpSessionState via set_mcp_tool_server_handle
    set_mcp_tool_server_handle(session_id, handle);

    // @step And subsequent ConnectMcpTool calls can access it for mid-turn registration
    let retrieved = get_mcp_tool_server_handle(session_id);
    assert!(
        retrieved.is_some(),
        "ToolServerHandle should be retrievable after set"
    );

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Scenario: Graceful degradation when ToolServerHandle is not set
// -----------------------------------------------------------
#[tokio::test]
async fn test_no_tool_server_handle_graceful_degradation() {
    // @step Given an MCP session is initialized without a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    // NOTE: We do NOT call set_mcp_tool_server_handle

    // @step When ConnectMcpTool successfully connects to an MCP server "playwright"
    // register_new_tools_with_handle reads handle from global state — should be None
    let tool = ConnectMcpTool::new(session_id);
    // This should return gracefully without error (debug log only)
    tool.register_new_tools_with_handle("playwright")
        .await
        .expect("missing handle defers registration successfully");

    // @step Then the connection succeeds and tools are stored in the connection map
    // @step And no error occurs during the connect call
    // @step But the tools are not registered with any ToolServerHandle
    let handle = get_mcp_tool_server_handle(session_id);
    assert!(
        handle.is_none(),
        "ToolServerHandle should be None when not set — graceful degradation"
    );

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Scenario: Same-turn tool invocation after connect
// -----------------------------------------------------------
#[tokio::test]
async fn test_same_turn_tool_registration_after_connect() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step When ConnectMcpTool successfully connects to an MCP server "playwright"
    // Build wrappers from tool defs (exercises from_tool_def) and register
    // via add_wrappers_to_handle (the actual production code path)
    let tool_defs = make_tool_defs(&["browser_navigate", "browser_click"]);
    let wrappers: Vec<McpToolWrapper> = tool_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("playwright", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, wrappers, "playwright", session_id)
        .await
        .expect("MCP tools register successfully");

    // @step Then the newly discovered tools are registered with the running agent's ToolServerHandle
    let defs = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs.len(), 2, "Should have 2 tools registered");

    // @step And calling "mcp__playwright__browser_navigate" succeeds in the same turn
    let has_navigate = defs
        .iter()
        .any(|d| d.name == "mcp__playwright__browser_navigate");
    let has_click = defs
        .iter()
        .any(|d| d.name == "mcp__playwright__browser_click");
    assert!(has_navigate, "browser_navigate should be in handle");
    assert!(has_click, "browser_click should be in handle");

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Scenario: Multiple servers connected in same turn
// -----------------------------------------------------------
#[tokio::test]
async fn test_multiple_servers_same_turn_registration() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step When ConnectMcpTool connects to server "serverA" with 2 tools
    let server_a_defs = make_tool_defs(&["tool1", "tool2"]);
    let wrappers_a: Vec<McpToolWrapper> = server_a_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("serverA", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, wrappers_a, "serverA", session_id)
        .await
        .expect("first server MCP tools register successfully");

    // @step And ConnectMcpTool connects to server "serverB" with 3 tools
    let server_b_defs = make_tool_defs(&["toolX", "toolY", "toolZ"]);
    let wrappers_b: Vec<McpToolWrapper> = server_b_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("serverB", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, wrappers_b, "serverB", session_id)
        .await
        .expect("second server MCP tools register successfully");

    // @step Then the ToolServerHandle contains tools from both "serverA" and "serverB"
    let defs = handle.get_tool_defs(None).await.unwrap();
    let server_a_tools: Vec<_> = defs
        .iter()
        .filter(|d| d.name.starts_with("mcp__serverA__"))
        .collect();
    let server_b_tools: Vec<_> = defs
        .iter()
        .filter(|d| d.name.starts_with("mcp__serverB__"))
        .collect();
    assert_eq!(server_a_tools.len(), 2, "serverA should have 2 tools");
    assert_eq!(server_b_tools.len(), 3, "serverB should have 3 tools");

    // @step And calling tools from either server succeeds
    assert_eq!(defs.len(), 5, "total tools should be 5");

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Scenario: Disconnect removes tools from running agent
// -----------------------------------------------------------
#[tokio::test]
async fn test_disconnect_removes_tools_from_handle() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step And ConnectMcpTool has connected to server "playwright" with tools registered
    let tool_defs = make_tool_defs(&["browser_navigate", "browser_click"]);
    let wrappers: Vec<McpToolWrapper> = tool_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("playwright", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, wrappers, "playwright", session_id)
        .await
        .expect("disconnect fixture MCP tools register successfully");

    // Verify tools are registered
    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 2);

    // @step When ConnectMcpTool disconnects from server "playwright"
    // Uses the actual production code: remove_server_tools_from_handle
    remove_server_tools_from_handle(&handle, "playwright", session_id).await;

    // @step Then the tools from "playwright" are removed from the ToolServerHandle
    let defs_after = handle.get_tool_defs(None).await.unwrap();
    let playwright_tools: Vec<_> = defs_after
        .iter()
        .filter(|d| d.name.starts_with("mcp__playwright__"))
        .collect();
    assert!(
        playwright_tools.is_empty(),
        "playwright tools should be removed after disconnect"
    );

    cleanup_mcp_session(session_id);
}
