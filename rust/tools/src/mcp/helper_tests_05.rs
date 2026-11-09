// -----------------------------------------------------------
// Scenario: Reconnect to one server does not affect another server's tools
// -----------------------------------------------------------
#[tokio::test]
async fn test_reconnect_does_not_affect_other_servers() {
    // @step Given an MCP session with two connected servers
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // Server A: webmcp with browser tools
    let webmcp_defs = make_tool_defs(&["browser_navigate", "browser_screenshot"]);
    let webmcp_wrappers: Vec<McpToolWrapper> = webmcp_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, webmcp_wrappers, "webmcp", session_id)
        .await
        .expect("web MCP tools register successfully");

    // Server B: github with repo tools
    let github_defs = make_tool_defs(&["create_issue", "list_repos"]);
    let github_wrappers: Vec<McpToolWrapper> = github_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("github", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, github_wrappers, "github", session_id)
        .await
        .expect("GitHub MCP tools register successfully");

    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 4, "should have 4 total tools");

    // @step When the agent reconnects to "webmcp" with an additional tool
    remove_server_tools_from_handle(&handle, "webmcp", session_id).await;
    let new_webmcp_defs =
        make_tool_defs(&["browser_navigate", "browser_screenshot", "getApiRequests"]);
    let new_webmcp_wrappers: Vec<McpToolWrapper> = new_webmcp_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, new_webmcp_wrappers, "webmcp", session_id)
        .await
        .expect("replacement web MCP tools register successfully");

    // @step Then github tools are unaffected
    let defs_after = handle.get_tool_defs(None).await.unwrap();
    let github_tools: Vec<_> = defs_after
        .iter()
        .filter(|d| d.name.starts_with("mcp__github__"))
        .collect();
    assert_eq!(github_tools.len(), 2, "github tools should be untouched");

    // @step And webmcp has the updated tool set
    let webmcp_tools: Vec<_> = defs_after
        .iter()
        .filter(|d| d.name.starts_with("mcp__webmcp__"))
        .collect();
    assert_eq!(
        webmcp_tools.len(),
        3,
        "webmcp should have 3 tools after reconnect"
    );

    // @step And total tool count is correct (no duplicates)
    assert_eq!(
        defs_after.len(),
        5,
        "total should be 5 (2 github + 3 webmcp)"
    );

    cleanup_mcp_session(session_id);
}
