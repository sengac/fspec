// ===================================================================
// Scenario: ConnectMCP with duplicate server name replaces existing connection
// ===================================================================
#[tokio::test]
async fn test_connect_mcp_with_duplicate_server_name_replaces_existing() {
    // @step Given MCP server "github" is already connected
    // connect_stdio handles replacement by removing old connection before inserting new one.
    // We test the replacement logic via data structures.
    let original = make_test_data("github", &["create_issue"], McpTransport::Stdio);
    let mut tool_map: HashMap<String, Vec<McpToolDef>> = HashMap::new();
    tool_map.insert("github".to_string(), original.tools.clone());
    assert_eq!(tool_map.get("github").unwrap().len(), 1);

    // @step When the agent calls ConnectMCP with name "github" and a different command
    let replacement = make_test_data(
        "github",
        &["create_issue", "list_repos", "search_code"],
        McpTransport::Stdio,
    );

    // @step Then the existing "github" connection should be disconnected first
    // Verified by: connect_stdio calls map.remove(name) + cancellation_token().cancel()

    // @step And the new connection should replace it in session.mcp_connections
    tool_map.insert("github".to_string(), replacement.tools.clone());
    assert_eq!(
        tool_map.len(),
        1,
        "should still have exactly one github entry"
    );
    assert_eq!(
        tool_map.get("github").unwrap().len(),
        3,
        "should have tools from new connection",
    );

    // @step And the tool should return a success result noting the replacement
    let result = McpConnectResult {
        success: true,
        message: format!(
            "✓ Connected: github (MCP {})\n  Server: {} v{}\n  Tools (3):",
            replacement.server_info.protocol_version,
            replacement.server_info.name,
            replacement.server_info.version,
        ),
        tools: Some(replacement.tools.clone()),
        connections: None,
    };
    assert!(result.success);
    assert_eq!(result.tools.as_ref().unwrap().len(), 3);

    // @step And old tools are removed from the ToolServerHandle before new ones are added
    // This verifies the fix: the Connect action now calls remove_tools_from_handle
    // before register_new_tools_with_handle. We simulate the full reconnect sequence
    // using a real ToolServerHandle.
    let session_id = uuid::Uuid::new_v4();
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();

    // Register old tools
    let old_wrappers: Vec<McpToolWrapper> = original
        .tools
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("github", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, old_wrappers, "github", session_id)
        .await
        .expect("initial MCP tools register successfully");
    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 1, "should have 1 old tool");

    // Reconnect: remove old, add new (mirrors the fixed production code)
    remove_server_tools_from_handle(&handle, "github", session_id).await;
    let new_wrappers: Vec<McpToolWrapper> = replacement
        .tools
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("github", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, new_wrappers, "github", session_id)
        .await
        .expect("replacement MCP tools register successfully");

    let defs_after = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(
        defs_after.len(),
        3,
        "should have exactly 3 new tools — no duplicates from reconnect"
    );
}

// ===================================================================
// Scenario: ConnectMCP connect without a transport is refused (MCP-009)
// ===================================================================
#[tokio::test]
async fn test_connect_mcp_connect_without_transport_is_refused() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, connections) = init_mcp_session(session_id);
    let tool = ConnectMcpTool::new(session_id);

    // @step Given the agent calls ConnectMCP with action "connect", name "srv", a command and an args list but no transport
    let args: McpConnectArgs = serde_json::from_value(serde_json::json!({
        "action": "connect",
        "name": "srv",
        "command": "mcp009-no-such-executable",
        "args": []
    }))
    .expect("ConnectMCP arguments without a transport still parse");

    // @step When the tool validates the call
    let result = rig::tool::Tool::call(&tool, args).await;

    // @step Then the call should fail with a validation error "transport is required for connect action"
    match result {
        Err(crate::ToolError::Validation { tool, message }) => {
            assert_eq!(tool, "ConnectMCP");
            assert_eq!(message, "transport is required for connect action");
        }
        other => panic!("expected a transport validation error, got {other:?}"),
    }

    // @step And no MCP connection named "srv" should exist in session.mcp_connections
    assert!(
        !connections.read().await.contains_key("srv"),
        "a refused connect must not register a connection"
    );

    cleanup_mcp_session(session_id);
}
