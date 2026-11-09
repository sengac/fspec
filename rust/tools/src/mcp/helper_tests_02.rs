#[tokio::test]
async fn test_injection_channel_sampling_roundtrip() {
    let (tx, mut rx) = mpsc::channel::<McpInjection>(4);
    let (resp_tx, resp_rx) =
        tokio::sync::oneshot::channel::<Result<CreateMessageResult, String>>();

    let params = CreateMessageRequestParams::new(vec![], 100);

    tx.send(McpInjection::SamplingRequest {
        params,
        response_tx: resp_tx,
    })
    .await
    .unwrap();

    // Consumer side
    match rx.recv().await.unwrap() {
        McpInjection::SamplingRequest {
            params: p,
            response_tx: rtx,
        } => {
            assert_eq!(p.max_tokens, 100);
            rtx.send(Err("test error".to_string())).unwrap();
        }
        _ => panic!("expected SamplingRequest"),
    }

    // Verify response
    let resp = resp_rx.await.unwrap();
    assert!(resp.is_err());
    assert_eq!(resp.unwrap_err(), "test error");
}

#[test]
fn test_connect_stdio_empty_command() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (tx, _rx) = mpsc::channel::<McpInjection>(4);
        let conns = new_mcp_connection_map();
        let result = connect_stdio("test", &crate::mcp::stdio_command::McpStdioCommand { program: String::new(), args: Vec::new() }, None, 10, tx, conns).await;
        assert!(!result.success);
        assert!(result.message.contains("Empty command"));
    });
}

#[tokio::test]
async fn test_init_mcp_session_creates_state() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, connections) = init_mcp_session(session_id);

    // Verify the map is empty initially
    {
        let map = connections.read().await;
        assert!(map.is_empty());
    }

    // Verify we can retrieve the connections via global state
    let retrieved = get_mcp_connections(session_id);
    assert!(retrieved.is_some());

    // Clean up
    cleanup_mcp_session(session_id);

    // Verify cleaned up
    let after_cleanup = get_mcp_connections(session_id);
    assert!(after_cleanup.is_none());
}

#[tokio::test]
async fn test_cleanup_mcp_session_removes_state() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    cleanup_mcp_session(session_id);

    let result = get_mcp_session_state(session_id);
    assert!(result.is_none());
}

#[tokio::test]
async fn test_gather_mcp_tool_registrations_empty_session() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let registrations = gather_mcp_tool_registrations(session_id).await;
    assert!(registrations.is_empty());

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_gather_mcp_tool_registrations_nonexistent_session() {
    let session_id = uuid::Uuid::new_v4();
    let registrations = gather_mcp_tool_registrations(session_id).await;
    assert!(registrations.is_empty());
}

#[tokio::test]
async fn test_connect_mcp_tool_call_without_session_returns_validation_result() {
    // ConnectMcpTool should return a message (not panic) when session not initialized
    let tool = ConnectMcpTool::new(uuid::Uuid::new_v4());
    let args = McpConnectArgs {
        action: McpAction::List,
        name: None,
        transport: None,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("MCP session not initialized"));
}

#[tokio::test]
async fn test_connect_mcp_tool_list_action_empty() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::List,
        name: None,
        transport: None,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await.unwrap();
    assert!(result.contains("No active MCP connections"));

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_connect_mcp_tool_connect_missing_name() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Connect,
        name: None,
        transport: Some(McpTransport::Stdio),
        command: Some("echo".to_string()),
        args: Some(vec!["test".to_string()]),
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("name is required"));

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_connect_mcp_tool_connect_missing_command() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Connect,
        name: Some("test".to_string()),
        transport: Some(McpTransport::Stdio),
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("command is required"));

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_connect_mcp_tool_connect_missing_url() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Connect,
        name: Some("test".to_string()),
        transport: Some(McpTransport::Http),
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("url is required"));

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_connect_mcp_tool_disconnect_missing_name() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Disconnect,
        name: None,
        transport: None,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("name is required"));

    cleanup_mcp_session(session_id);
}

#[tokio::test]
async fn test_connect_mcp_tool_disconnect_nonexistent() {
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);

    let tool = ConnectMcpTool::new(session_id);
    let args = McpConnectArgs {
        action: McpAction::Disconnect,
        name: Some("nonexistent".to_string()),
        transport: None,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    };
    // Returns success=false but as Ok(message) so LLM can reason about it
    let result = tool.call(args).await.unwrap();
    assert!(result.contains("No connection named"));

    cleanup_mcp_session(session_id);
}
