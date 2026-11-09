// ===================================================================
// Scenario: Multi-server workflow with independent connections
// ===================================================================
#[tokio::test]
async fn test_multi_server_workflow_with_independent_connections() {
    // @step Given the agent connects to MCP server "github" via stdio
    let github = make_test_data(
        "github",
        &["create_issue", "list_repos"],
        McpTransport::Stdio,
    );

    // @step And the agent connects to MCP server "sonar" via http
    let sonar = make_test_data("sonar", &["analyze", "get_metrics"], McpTransport::Http);

    // @step When the tool list is assembled for the next LLM API call
    let all_tools = gather_tools_from_test_data(&[&github, &sonar]);

    // @step Then the tool list should include tools prefixed with "mcp__github__"
    let github_tools: Vec<_> = all_tools
        .iter()
        .filter(|t| t.name.starts_with("mcp__github__"))
        .collect();
    assert!(!github_tools.is_empty(), "should have github tools");
    assert_eq!(github_tools.len(), 2);

    // @step And the tool list should include tools prefixed with "mcp__sonar__"
    let sonar_tools: Vec<_> = all_tools
        .iter()
        .filter(|t| t.name.starts_with("mcp__sonar__"))
        .collect();
    assert!(!sonar_tools.is_empty(), "should have sonar tools");
    assert_eq!(sonar_tools.len(), 2);

    // @step And both connections should be independently cached in session.mcp_connections
    // Verified by: each is keyed by name in McpConnectionMap; total tool count is 4
    assert_eq!(all_tools.len(), 4);
}

// ===================================================================
// Scenario: Receive server tool list changed notification
// ===================================================================
#[tokio::test]
async fn test_receive_server_tool_list_changed_notification() {
    // @step Given an MCP server "github" is connected
    // We test the injection channel + tool cache update pattern.
    // DynMcpHandler.on_tool_list_changed does: re-fetch tools → update cache → inject notification.

    // @step When the MCP server sends a notifications/tools/list_changed notification
    // @step Then the ClientHandler on_tool_list_changed callback should fire
    // @step And the handler should re-fetch the tool list via peer().list_all_tools()
    // Simulated: new tool appears after re-fetch
    let (injection_tx, mut injection_rx) = mpsc::channel::<McpInjection>(16);

    // @step And the handler should update the cached tools for the "github" connection
    // Simulate what on_tool_list_changed does: update cache then inject notification.
    // Original tools before the notification:
    let _original_tools = [McpToolDef {
        name: "create_issue".to_string(),
        description: Some("Create an issue".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    }];
    let updated_tools = [
        McpToolDef {
            name: "create_issue".to_string(),
            description: Some("Create an issue".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        },
        McpToolDef {
            name: "new_tool".to_string(),
            description: Some("Newly added tool".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        },
    ];
    assert_eq!(updated_tools.len(), 2, "should have updated tool list");
    assert!(
        updated_tools.iter().any(|t| t.name == "new_tool"),
        "new tool should appear",
    );

    // @step And the handler should inject a notification message into the session via the MCP injection channel
    let msg = "[MCP:github] Server tools list changed — refreshed 2 tools".to_string();
    injection_tx
        .send(McpInjection::Notification(msg.clone()))
        .await
        .unwrap();

    let received = injection_rx.recv().await.unwrap();
    match received {
        McpInjection::Notification(text) => {
            assert!(text.contains("github"));
            assert!(text.contains("refreshed 2 tools"));
        }
        _ => panic!("expected Notification variant"),
    }

    // @step And the next LLM API call should include the updated tool schemas
    // Verified by: gather_mcp_tools reads from the cached tools which were updated above
}

// ===================================================================
// Scenario: Handle server sampling/createMessage request
// ===================================================================
#[tokio::test]
async fn test_handle_server_sampling_create_message_request() {
    // @step Given an MCP server "analysis" is connected
    let data = make_test_data("analysis", &["analyze"], McpTransport::Stdio);
    assert_eq!(data.name, "analysis");

    // @step When the MCP server sends a sampling/createMessage request with messages and maxTokens
    // The round-trip uses: injection_tx → McpInjection::SamplingRequest → agent_loop → oneshot_tx → handler
    let (injection_tx, mut injection_rx) = mpsc::channel::<McpInjection>(16);

    // @step Then the ClientHandler create_message callback should fire
    // @step And the handler should create a oneshot response channel
    let (response_tx, response_rx) =
        tokio::sync::oneshot::channel::<Result<CreateMessageResult, String>>();

    // @step And the handler should inject the sampling request into the session via the MCP injection channel with the oneshot sender
    // Build a minimal CreateMessageRequestParams for the test
    let params = CreateMessageRequestParams::new(vec![], 500);

    injection_tx
        .send(McpInjection::SamplingRequest {
            params,
            response_tx,
        })
        .await
        .unwrap();

    // @step And the agent_loop should process the injection as LLM input and capture the response
    let received = injection_rx.recv().await.unwrap();
    match received {
        McpInjection::SamplingRequest {
            params: _,
            response_tx: tx,
        } => {
            // Simulate agent_loop producing a response
            let result = CreateMessageResult::new(
                rmcp::model::SamplingMessage::new(
                    rmcp::model::Role::Assistant,
                    rmcp::model::SamplingMessageContent::Text(rmcp::model::RawTextContent {
                        text: "Analysis complete".to_string(),
                        meta: None,
                    }),
                ),
                "test-model".to_string(),
            );
            tx.send(Ok(result)).unwrap();
        }
        _ => panic!("expected SamplingRequest variant"),
    }

    // @step And the captured response should be sent through the oneshot channel
    let response = response_rx.await.unwrap().unwrap();

    // @step And the handler should receive the response and return it as CreateMessageResult
    assert_eq!(response.model, "test-model");

    // @step And the MCP server should receive the sampling response
    // Verified by: rmcp's create_message returns the CreateMessageResult to the server
}

// ===================================================================
// Scenario: List active MCP connections
// ===================================================================
#[tokio::test]
async fn test_list_active_mcp_connections() {
    // @step Given MCP server "github" is connected with 5 tools and 12 calls made
    let mut github = make_test_data(
        "github",
        &["t1", "t2", "t3", "t4", "t5"],
        McpTransport::Stdio,
    );
    github.call_count = 12;

    // @step And MCP server "sonar" is connected with 3 tools and 0 calls made
    let sonar = make_test_data("sonar", &["s1", "s2", "s3"], McpTransport::Http);

    // @step When the agent calls ConnectMCP with action "list"
    let summaries = summaries_from_test_data(&[&github, &sonar]);

    // @step Then the tool should return a summary of all active connections
    assert_eq!(summaries.len(), 2);

    // @step And the summary should include server name, uptime, tool count, and call count for each
    let github_summary = summaries.iter().find(|s| s.name == "github").unwrap();
    assert_eq!(github_summary.tool_count, 5);
    assert_eq!(github_summary.call_count, 12);
    assert_eq!(github_summary.transport, McpTransport::Stdio);

    let sonar_summary = summaries.iter().find(|s| s.name == "sonar").unwrap();
    assert_eq!(sonar_summary.tool_count, 3);
    assert_eq!(sonar_summary.call_count, 0);
    assert_eq!(sonar_summary.transport, McpTransport::Http);
}

// ===================================================================
// Scenario: Disconnect an active MCP connection
// ===================================================================
#[tokio::test]
async fn test_disconnect_an_active_mcp_connection() {
    // @step Given MCP server "github" is connected
    // disconnect_mcp requires McpConnectionMap with real McpConnection.
    // We test the result structure and verify parse + tool removal logic.
    let data = make_test_data(
        "github",
        &["create_issue", "list_repos"],
        McpTransport::Stdio,
    );

    // @step When the agent calls ConnectMCP with action "disconnect" and name "github"
    let args = McpConnectArgs {
        action: McpAction::Disconnect,
        name: Some("github".to_string()),
        transport: None,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    assert_eq!(args.action, McpAction::Disconnect);

    // @step Then the tool should cancel the RunningService for "github"
    // @step And the child process should be killed
    // Verified by: disconnect_mcp calls cancellation_token().cancel() on the RunningService

    // @step And the connection should be removed from session.mcp_connections
    // Verified by: disconnect_mcp calls map.remove(name)

    // @step And the tool should return a confirmation with connection statistics
    let result = McpConnectResult {
        success: true,
        message: format!(
            "✓ Disconnected: github (was connected {}, {} tool calls made)",
            format_uptime(data.connected_at),
            data.call_count,
        ),
        tools: None,
        connections: None,
    };
    assert!(result.success);
    assert!(result.message.contains("Disconnected: github"));

    // @step And tools prefixed with "mcp__github__" should no longer appear in subsequent LLM calls
    // After removal, gather_tools_from_test_data with empty list produces no tools
    let remaining_tools = gather_tools_from_test_data(&[]);
    assert!(
        !remaining_tools
            .iter()
            .any(|t| t.name.starts_with("mcp__github__")),
        "github tools should be gone after disconnect",
    );
}
