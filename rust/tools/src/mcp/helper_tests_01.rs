#[test]
fn test_parse_mcp_tool_name_valid() {
    let result = parse_mcp_tool_name("mcp__github__create_issue");
    assert_eq!(result, Some(("github", "create_issue")));
}

#[test]
fn test_parse_mcp_tool_name_no_prefix() {
    assert!(parse_mcp_tool_name("github__create_issue").is_none());
}

#[test]
fn test_parse_mcp_tool_name_single_underscore() {
    assert!(parse_mcp_tool_name("mcp_github_create_issue").is_none());
}

#[test]
fn test_parse_mcp_tool_name_empty_server() {
    assert!(parse_mcp_tool_name("mcp____tool").is_none());
}

#[test]
fn test_parse_mcp_tool_name_empty_tool() {
    assert!(parse_mcp_tool_name("mcp__server__").is_none());
}

#[test]
fn test_parse_mcp_tool_name_builtin() {
    assert!(parse_mcp_tool_name("Read").is_none());
    assert!(parse_mcp_tool_name("Bash").is_none());
}

#[test]
fn test_parse_mcp_tool_name_with_nested_double_underscore() {
    // mcp__server__tool__subtool should parse as server="server", tool="tool__subtool"
    let result = parse_mcp_tool_name("mcp__server__tool__subtool");
    assert_eq!(result, Some(("server", "tool__subtool")));
}

#[test]
fn test_qualified_mcp_tool_name() {
    assert_eq!(
        qualified_mcp_tool_name("github", "create_issue"),
        "mcp__github__create_issue",
    );
}

#[test]
fn test_qualified_roundtrip() {
    let qualified = qualified_mcp_tool_name("myserver", "my_tool");
    let (server, tool) = parse_mcp_tool_name(&qualified).unwrap();
    assert_eq!(server, "myserver");
    assert_eq!(tool, "my_tool");
}

#[test]
fn test_gather_mcp_tools_empty() {
    let connections: HashMap<String, McpConnection> = HashMap::new();
    let tools = gather_mcp_tools(&connections);
    assert!(tools.is_empty());
}

#[test]
fn test_new_mcp_connection_map_is_empty() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let map = new_mcp_connection_map();
        let inner = map.read().await;
        assert!(inner.is_empty());
    });
}

#[test]
fn test_mcp_connect_args_deserialization() {
    let json = r#"{
        "action": "connect",
        "name": "test",
        "transport": "stdio",
        "command": "echo",
        "args": ["hello"]
    }"#;
    let args: McpConnectArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.action, McpAction::Connect);
    assert_eq!(args.name.as_deref(), Some("test"));
    assert_eq!(args.transport, Some(McpTransport::Stdio));
    assert_eq!(args.command.as_deref(), Some("echo"));
    assert_eq!(args.args, Some(vec!["hello".to_string()]));
    assert!(args.env.is_none());
}

#[test]
fn test_mcp_connect_args_with_env() {
    let json = r#"{
        "action": "connect",
        "name": "github",
        "transport": "stdio",
        "command": "npx",
        "args": ["server"],
        "env": {"GITHUB_TOKEN": "ghp_abc123"}
    }"#;
    let args: McpConnectArgs = serde_json::from_str(json).unwrap();
    let env = args.env.unwrap();
    assert_eq!(env.get("GITHUB_TOKEN").unwrap(), "ghp_abc123");
}

#[test]
fn test_mcp_connect_args_with_headers() {
    let json = r#"{
        "action": "connect",
        "name": "remote",
        "transport": "http",
        "url": "https://mcp.example.com",
        "headers": {"Authorization": "Bearer token123"}
    }"#;
    let args: McpConnectArgs = serde_json::from_str(json).unwrap();
    let headers = args.headers.unwrap();
    assert_eq!(headers.get("Authorization").unwrap(), "Bearer token123");
}

/// MCP-011: there is no default action.
#[test]
fn test_mcp_connect_args_require_an_action() {
    let json = r#"{
        "name": "test",
        "transport": "stdio",
        "command": "echo",
        "args": []
    }"#;
    let error = serde_json::from_str::<McpConnectArgs>(json).unwrap_err();
    assert!(error.to_string().contains("action"), "{error}");
}

#[test]
fn test_mcp_connect_args_disconnect_action() {
    let json = r#"{
        "action": "disconnect",
        "name": "github"
    }"#;
    let args: McpConnectArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.action, McpAction::Disconnect);
    assert_eq!(args.name.as_deref(), Some("github"));
    assert!(args.transport.is_none());
    assert!(args.command.is_none());
}

#[test]
fn test_mcp_connect_args_list_action() {
    let json = r#"{"action": "list"}"#;
    let args: McpConnectArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.action, McpAction::List);
    assert!(args.name.is_none());
}

#[test]
fn test_format_uptime_seconds() {
    let instant = Instant::now();
    let result = format_uptime(instant);
    assert!(result.contains("ago"), "result was: {result}");
    assert!(result.contains("0s"), "just-created should be 0s ago");
}

#[test]
fn test_mcp_connect_result_serialization() {
    let result = McpConnectResult {
        success: true,
        message: "✓ Connected".to_string(),
        tools: Some(vec![McpToolDef {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        }]),
        connections: None,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["success"], true);
    assert!(json["tools"].is_array());
    assert_eq!(json["tools"][0]["name"], "test_tool");
}

#[test]
fn test_mcp_connection_summary_serialization() {
    let summary = McpConnectionSummary {
        name: "test".to_string(),
        uptime: "5m ago".to_string(),
        tool_count: 3,
        call_count: 42,
        transport: McpTransport::Stdio,
    };
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["name"], "test");
    assert_eq!(json["tool_count"], 3);
    assert_eq!(json["call_count"], 42);
    assert_eq!(json["transport"], "stdio");
}

#[test]
fn test_mcp_server_info_fields() {
    let info = McpServerInfo {
        name: "Test Server".to_string(),
        version: "2.0.0".to_string(),
        protocol_version: "2025-11-25".to_string(),
    };
    assert_eq!(info.name, "Test Server");
    assert_eq!(info.version, "2.0.0");
    assert_eq!(info.protocol_version, "2025-11-25");
}

#[test]
fn test_mcp_tool_def_without_description() {
    let tool = McpToolDef {
        name: "bare_tool".to_string(),
        description: None,
        input_schema: serde_json::json!({"type": "object"}),
    };
    assert!(tool.description.is_none());
    let json = serde_json::to_value(&tool).unwrap();
    assert_eq!(json["name"], "bare_tool");
    assert!(json["description"].is_null());
}

#[test]
fn test_gather_tools_namespaces_correctly() {
    let data = make_test_data("srv", &["a", "b"], McpTransport::Stdio);
    let tools = gather_tools_from_test_data(&[&data]);
    assert_eq!(tools[0].name, "mcp__srv__a");
    assert_eq!(tools[1].name, "mcp__srv__b");
}

#[test]
fn test_summaries_from_test_data_uptime() {
    let data = make_test_data("x", &["t"], McpTransport::Http);
    let summaries = summaries_from_test_data(&[&data]);
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].uptime.contains("ago"));
    assert_eq!(summaries[0].transport, McpTransport::Http);
}

#[tokio::test]
async fn test_injection_channel_notification() {
    let (tx, mut rx) = mpsc::channel::<McpInjection>(4);
    tx.send(McpInjection::Notification("hello".to_string()))
        .await
        .unwrap();
    match rx.recv().await.unwrap() {
        McpInjection::Notification(msg) => assert_eq!(msg, "hello"),
        _ => panic!("expected Notification"),
    }
}
