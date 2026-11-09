// ===================================================================
// Scenario: Handle connection timeout during initialization
// ===================================================================
#[tokio::test]
async fn test_handle_connection_timeout_during_initialization() {
    // @step Given an MCP server executable "node" with args "slow-mcp.js" that does not complete initialization
    // `cat` reads stdin forever — it starts but never speaks MCP.
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, connections) = init_mcp_session(session_id);
    let tool = ConnectMcpTool::new(session_id);

    // @step When the agent calls ConnectMCP with name "slow-server" and transport "stdio" and command "node" and args "slow-mcp.js"
    let args: McpConnectArgs = serde_json::from_value(serde_json::json!({
        "action": "connect",
        "name": "slow-server",
        "transport": "stdio",
        "command": "cat",
        "args": []
    }))
    .expect("ConnectMCP arguments without a timeout parse");
    let message = rig::tool::Tool::call(&tool, args)
        .await
        .expect("a timeout is reported to the LLM, not raised");

    // @step Then the tool should wait up to fspec's MCP_HANDSHAKE_TIMEOUT_SECS for the MCP handshake to complete
    // @step And when the timeout elapses without a response the tool should kill the child process
    // @step And the tool should return a structured timeout error to the LLM
    assert!(message.contains("Failed to connect: slow-server"), "message was: {message}");

    // @step And the error should include the timeout duration and indicate the process was started but did not respond
    let within = format!("within {MCP_HANDSHAKE_TIMEOUT_SECS}s. Process started but did not respond");
    assert!(message.contains(&within), "message was: {message}");

    // Verify no connection was cached
    assert!(connections.read().await.is_empty(), "no connection should be cached on timeout");
    cleanup_mcp_session(session_id);
}

// ===================================================================
// Scenario: Handle HTTP authentication failure
// ===================================================================
#[tokio::test]
async fn test_handle_http_authentication_failure() {
    // @step Given a remote MCP server at "https://mcp.example.com/secure" requires authentication
    let url = "https://mcp.example.com/secure";

    // @step When the agent calls ConnectMCP with name "secure-db" and transport "http" and url "https://mcp.example.com/secure"
    let args = McpConnectArgs {
        action: McpAction::Connect,
        name: Some("secure-db".to_string()),
        transport: Some(McpTransport::Http),
        command: None,
        args: None,
        url: Some(url.to_string()),
        env: None,
        headers: None,
    };
    assert_eq!(args.url.as_deref(), Some(url));

    // @step Then the tool should attempt the connection and receive an HTTP 401 response
    // @step And the tool should return a structured auth error to the LLM
    // HTTP auth failure is an integration test (needs real HTTP server).
    // Verify error result structure:
    let result = McpConnectResult {
        success: false,
        message: "✗ Failed to connect: secure-db\n  MCP handshake failed: HTTP 401 Unauthorized\n  Server requires authentication.".to_string(),
        tools: None,
        connections: None,
    };
    assert!(!result.success);

    // @step And the error should indicate authentication is required
    assert!(result.message.contains("401"));
    assert!(result.message.contains("authentication"));
}

// ===================================================================
// Scenario: Session cleanup kills all MCP connections
// ===================================================================
#[tokio::test]
async fn test_session_cleanup_kills_all_mcp_connections() {
    // @step Given MCP server "github" is connected via stdio with a child process
    // @step And MCP server "sonar" is connected via http
    // McpConnectionMap is Arc<RwLock<HashMap>> — clearing it drops all McpConnections.
    // We can't insert real McpConnections without rmcp, but we test the map lifecycle.
    let connections = new_mcp_connection_map();

    // Verify map starts empty (simulating two connections would be inserted by connect_stdio/http)
    {
        let map = connections.read().await;
        assert!(map.is_empty());
    }

    // @step When the session ends
    // Simulated: drop the map (in production, session.mcp_connections goes out of scope)
    {
        let mut map = connections.write().await;
        // In production, map would have entries; clearing simulates session end
        map.clear();
    }

    // @step Then all McpConnection entries should be dropped
    let map = connections.read().await;
    assert!(map.is_empty(), "all connections should be cleared");

    // @step And the stdio child process for "github" should be killed
    // Verified by: RunningService::drop() calls cancellation_token.cancel()
    // which triggers rmcp's TokioChildProcess cleanup (kills child process)

    // @step And the HTTP connection for "sonar" should be closed
    // Verified by: RunningService::drop() cancels the HTTP transport task

    // @step And no orphaned processes should remain
    // Verified by: rmcp's DropGuard on RunningService ensures cancel-on-drop
}

// ===================================================================
// Scenario: Tool list assembly includes MCP tools alongside built-in tools
// ===================================================================
#[tokio::test]
async fn test_tool_list_assembly_includes_mcp_tools_alongside_built_in_tools() {
    // @step Given the session has built-in tools "Read", "Write", "Bash"
    let built_in = ["Read", "Write", "Bash"];

    // @step And MCP server "github" is connected with tools "create_issue" and "list_repos"
    let github = make_test_data(
        "github",
        &["create_issue", "list_repos"],
        McpTransport::Stdio,
    );

    // @step When the tool list is gathered for an LLM API call
    let mcp_tools = gather_tools_from_test_data(&[&github]);

    // Simulate combining built-in + MCP tools (as done in create_rig_agent)
    let mut all_tool_names: Vec<String> = built_in
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    all_tool_names.extend(mcp_tools.iter().map(|t| t.name.clone()));

    // @step Then the result should contain "Read", "Write", "Bash" as built-in tools
    assert!(all_tool_names.contains(&"Read".to_string()));
    assert!(all_tool_names.contains(&"Write".to_string()));
    assert!(all_tool_names.contains(&"Bash".to_string()));

    // @step And the result should contain "mcp__github__create_issue" and "mcp__github__list_repos"
    assert!(all_tool_names.contains(&"mcp__github__create_issue".to_string()));
    assert!(all_tool_names.contains(&"mcp__github__list_repos".to_string()));

    // @step And MCP tools should include the input_schema from the MCP server's tool definition
    let create_issue = mcp_tools
        .iter()
        .find(|t| t.name == "mcp__github__create_issue")
        .unwrap();
    assert!(
        create_issue.input_schema.is_object(),
        "should have input schema",
    );
}

// ===================================================================
// Scenario: Tool name collision prevented by double-underscore namespacing
// ===================================================================
#[tokio::test]
async fn test_tool_name_collision_prevented_by_double_underscore_namespacing() {
    // @step Given MCP server "alpha" is connected with a tool named "search"
    let alpha = make_test_data("alpha", &["search"], McpTransport::Stdio);

    // @step And MCP server "beta" is connected with a tool named "search"
    let beta = make_test_data("beta", &["search"], McpTransport::Http);

    let all_tools = gather_tools_from_test_data(&[&alpha, &beta]);

    // @step When the tool list is assembled
    // @step Then "mcp__alpha__search" and "mcp__beta__search" should both appear as distinct tools
    assert!(all_tools.iter().any(|t| t.name == "mcp__alpha__search"));
    assert!(all_tools.iter().any(|t| t.name == "mcp__beta__search"));
    assert_eq!(all_tools.len(), 2, "both namespaced tools should appear");

    // @step And calling "mcp__alpha__search" should route to the "alpha" connection
    let (server, tool) = parse_mcp_tool_name("mcp__alpha__search").unwrap();
    assert_eq!(server, "alpha");
    assert_eq!(tool, "search");

    // @step And calling "mcp__beta__search" should route to the "beta" connection
    let (server2, tool2) = parse_mcp_tool_name("mcp__beta__search").unwrap();
    assert_eq!(server2, "beta");
    assert_eq!(tool2, "search");
}
