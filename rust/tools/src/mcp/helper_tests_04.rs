// -----------------------------------------------------------
// Scenario: Previous-turn tools coexist with same-turn tools
// -----------------------------------------------------------
#[tokio::test]
async fn test_previous_turn_tools_coexist_with_same_turn_tools() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step And server "existing_server" was connected in a previous turn with tools already registered
    // Previous-turn tools are added via gather_mcp_tool_wrappers at turn start.
    // Simulate by adding directly to the handle (as run_with_provider! does).
    let existing_defs = make_tool_defs(&["old_tool"]);
    let existing_wrappers: Vec<McpToolWrapper> = existing_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("existing_server", td, session_id))
        .collect();
    for w in existing_wrappers {
        handle.add_tool(w).await.unwrap();
    }

    // @step When ConnectMcpTool connects to a new server "new_server"
    // Mid-turn registration: uses add_wrappers_to_handle (production path)
    let new_defs = make_tool_defs(&["new_tool"]);
    let new_wrappers: Vec<McpToolWrapper> = new_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("new_server", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, new_wrappers, "new_server", session_id)
        .await
        .expect("new server MCP tools register successfully");

    // @step Then tools from both "existing_server" and "new_server" are available
    let defs = handle.get_tool_defs(None).await.unwrap();
    let has_existing = defs
        .iter()
        .any(|d| d.name == "mcp__existing_server__old_tool");
    let has_new = defs.iter().any(|d| d.name == "mcp__new_server__new_tool");
    assert!(
        has_existing,
        "existing server tool should still be available"
    );
    assert!(has_new, "new server tool should be available");

    // @step And only "new_server" tools were added mid-turn via add_tool
    assert_eq!(defs.len(), 2, "both tools should coexist");

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Additional: Verify from_tool_def produces correct qualified names
// -----------------------------------------------------------
#[test]
fn test_from_tool_def_qualified_name() {
    let def = McpToolDef {
        name: "browser_navigate".to_string(),
        description: Some("Navigate to URL".to_string()),
        input_schema: serde_json::json!({"type": "object", "properties": {"url": {"type": "string"}}}),
    };
    let session_id = uuid::Uuid::new_v4();
    let wrapper = McpToolWrapper::from_tool_def("playwright", &def, session_id);

    assert_eq!(wrapper.qualified_name, "mcp__playwright__browser_navigate");
    assert_eq!(wrapper.description, "Navigate to URL");
    assert_eq!(wrapper.session_id, session_id);
}

#[test]
fn test_from_tool_def_empty_description() {
    let def = McpToolDef {
        name: "do_thing".to_string(),
        description: None,
        input_schema: serde_json::json!({"type": "object"}),
    };
    let wrapper = McpToolWrapper::from_tool_def("srv", &def, uuid::Uuid::new_v4());

    assert_eq!(wrapper.qualified_name, "mcp__srv__do_thing");
    assert_eq!(wrapper.description, ""); // None → empty string
}

// -----------------------------------------------------------
// Additional: Verify remove_server_tools_from_handle is prefix-scoped
// -----------------------------------------------------------
#[tokio::test]
async fn test_remove_only_affects_matching_prefix() {
    let session_id = uuid::Uuid::new_v4();
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();

    // Add tools from two servers
    let defs_a = make_tool_defs(&["tool1"]);
    let defs_b = make_tool_defs(&["tool1"]);
    let wrappers_a: Vec<McpToolWrapper> = defs_a
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("serverA", td, session_id))
        .collect();
    let wrappers_b: Vec<McpToolWrapper> = defs_b
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("serverB", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, wrappers_a, "serverA", session_id)
        .await
        .expect("first server MCP tools register successfully");
    add_wrappers_to_handle(&handle, wrappers_b, "serverB", session_id)
        .await
        .expect("second server MCP tools register successfully");

    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 2);

    // Remove only serverA
    remove_server_tools_from_handle(&handle, "serverA", session_id).await;

    let defs_after = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_after.len(), 1, "only serverB tool should remain");
    assert_eq!(defs_after[0].name, "mcp__serverB__tool1");
}

// -----------------------------------------------------------
// Scenario: Reconnect to same server removes old tools before adding new ones
// -----------------------------------------------------------
// Verifies fix: ConnectMCP Connect action now calls remove_tools_from_handle
// before register_new_tools_with_handle. Without this fix, reconnecting to
// the same server name would add duplicate tools to the ToolServerHandle,
// triggering "Tool names must be unique" errors from the LLM API.
#[tokio::test]
async fn test_reconnect_same_server_removes_old_tools_first() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step And server "webmcp" was previously connected with tools registered
    let original_defs = make_tool_defs(&[
        "browser_navigate",
        "browser_screenshot",
        "browser_list_tabs",
    ]);
    let original_wrappers: Vec<McpToolWrapper> = original_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, original_wrappers, "webmcp", session_id)
        .await
        .expect("original MCP tools register successfully");

    // Verify original tools are registered
    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 3, "should have 3 original tools");

    // @step When the agent reconnects to "webmcp" (remove old, then add new)
    // This mirrors the fixed production code path in ConnectMcpTool::call():
    //   tool.remove_tools_from_handle(&name).await;
    //   tool.register_new_tools_with_handle(&name).await;
    remove_server_tools_from_handle(&handle, "webmcp", session_id).await;

    // Verify old tools are gone
    let defs_mid = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(
        defs_mid.len(),
        0,
        "old tools should be removed before re-add"
    );

    // Now register the new tools (server may expose different tools after reconnect)
    let new_defs = make_tool_defs(&[
        "browser_navigate",
        "browser_screenshot",
        "browser_list_tabs",
        "getApiRequests",
    ]);
    let new_wrappers: Vec<McpToolWrapper> = new_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, new_wrappers, "webmcp", session_id)
        .await
        .expect("replacement MCP tools register successfully");

    // @step Then the ToolServerHandle contains only the new tools (no duplicates)
    let defs_after = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(
        defs_after.len(),
        4,
        "should have 4 new tools (3 original + 1 new)"
    );

    // @step And no duplicate tool names exist
    let mut names: Vec<String> = defs_after.iter().map(|d| d.name.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(
        names.len(),
        4,
        "all tool names must be unique — no duplicates from reconnect"
    );

    // @step And the new tool (getApiRequests) is present
    let has_new_tool = defs_after
        .iter()
        .any(|d| d.name == "mcp__webmcp__getApiRequests");
    assert!(has_new_tool, "new tool from reconnect should be registered");

    cleanup_mcp_session(session_id);
}

// -----------------------------------------------------------
// Scenario: Reconnect with identical tools produces no duplicates
// -----------------------------------------------------------
// Edge case: reconnecting to the same server with the exact same tool set.
// Without the remove-first fix, each reconnect would double the tool count.
#[tokio::test]
async fn test_reconnect_identical_tools_no_duplicates() {
    // @step Given an MCP session is initialized with a ToolServerHandle
    let session_id = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session_id);
    let server = rig::tool::server::ToolServer::new();
    let handle = server.run();
    set_mcp_tool_server_handle(session_id, handle.clone());

    // @step And server "webmcp" is connected with 11 native browser tools
    let tool_names: Vec<&str> = vec![
        "browser_navigate",
        "browser_screenshot",
        "browser_list_tabs",
        "browser_execute_script",
        "browser_switch_tab",
        "browser_close_tab",
        "browser_get_page_content",
        "browser_click_element",
        "browser_fill_form",
        "browser_go_back",
        "browser_go_forward",
    ];
    let original_defs = make_tool_defs(&tool_names);
    let original_wrappers: Vec<McpToolWrapper> = original_defs
        .iter()
        .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
        .collect();
    add_wrappers_to_handle(&handle, original_wrappers, "webmcp", session_id)
        .await
        .expect("reconnect fixture MCP tools register successfully");

    let defs_before = handle.get_tool_defs(None).await.unwrap();
    assert_eq!(defs_before.len(), 11, "should start with 11 tools");

    // @step When the agent reconnects to "webmcp" 3 times in a row
    for reconnect_num in 1..=3 {
        // Remove old tools first (the fix)
        remove_server_tools_from_handle(&handle, "webmcp", session_id).await;
        // Re-add same tools
        let reconnect_defs = make_tool_defs(&tool_names);
        let reconnect_wrappers: Vec<McpToolWrapper> = reconnect_defs
            .iter()
            .map(|td| McpToolWrapper::from_tool_def("webmcp", td, session_id))
            .collect();
        add_wrappers_to_handle(&handle, reconnect_wrappers, "webmcp", session_id)
            .await
            .expect("reconnected MCP tools register successfully");

        // @step Then the tool count stays at 11 after each reconnect
        let defs = handle.get_tool_defs(None).await.unwrap();
        assert_eq!(
            defs.len(),
            11,
            "after reconnect #{reconnect_num}, should still have exactly 11 tools"
        );
    }

    cleanup_mcp_session(session_id);
}
