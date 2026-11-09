//! MCP-012 — MCP tool discovery failures fail the connect.
//!
//! Feature: spec/features/mcp-tool-discovery-failures-fail-the-connect.feature

use codelet_tools::{cleanup_mcp_session, init_mcp_session, ConnectMcpTool, McpConnectArgs};
use rig::tool::Tool;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A stdio MCP server in `sh`: it answers `initialize` (echoing the
/// client's id and protocol version), reads `notifications/initialized`,
/// then handles `tools/list` with `on_list` and reads stdin forever.
fn fixture_script(on_list: &str) -> String {
    format!(
        r#"read line
id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
version=$(printf '%s' "$line" | sed -n 's/.*"protocolVersion":"\([^"]*\)".*/\1/p')
printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"%s","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"fixture","version":"1"}}}}}}\n' "$id" "$version"
read initialized
read list
id=$(printf '%s' "$list" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
{on_list}
cat >/dev/null
"#
    )
}

async fn connect(script: String) -> Result<(String, usize), Box<dyn std::error::Error>> {
    let session_id = uuid::Uuid::new_v4();
    let (_injection, connections) = init_mcp_session(session_id);
    let args: McpConnectArgs = serde_json::from_value(json!({
        "action": "connect",
        "name": "fixture",
        "transport": "stdio",
        "command": "sh",
        "args": ["-c", script],
    }))?;
    let message = ConnectMcpTool::new(session_id).call(args).await?;
    let cached = connections.read().await.len();
    cleanup_mcp_session(session_id);
    Ok((message, cached))
}

#[tokio::test]
async fn a_tools_list_error_fails_the_connect() -> TestResult {
    codelet_tools::facade::declare_unrestricted_tool_access("MCP fixture tests");
    // @step Given a stdio MCP server that completes the handshake and answers tools/list with the error "tools are broken"
    let script = fixture_script(
        r#"printf '{"jsonrpc":"2.0","id":%s,"error":{"code":-32603,"message":"tools are broken"}}\n' "$id""#,
    );
    // @step When ConnectMCP connects to it
    let (message, cached) = connect(script).await?;
    // @step Then the connect fails naming "tools are broken" and no connection is cached
    assert!(message.contains("Failed to connect: fixture"), "{message}");
    assert!(message.contains("tools are broken"), "{message}");
    assert_eq!(cached, 0);
    Ok(())
}

#[tokio::test]
async fn an_unanswered_tools_list_fails_the_connect_within_the_timeout() -> TestResult {
    codelet_tools::facade::declare_unrestricted_tool_access("MCP fixture tests");
    // @step Given a stdio MCP server that completes the handshake and never answers tools/list
    let script = fixture_script(":");
    // @step When ConnectMCP connects to it
    let connected = codelet_test_helpers::deadlock_guard("connect(script)", connect(script))
        .await
        .map_err(|_| "ConnectMCP did not return: tool discovery is unbounded")?;
    let (message, cached) = connected?;
    // @step Then the connect fails saying tool discovery did not complete within the MCP timeout and no connection is cached
    assert!(message.contains("Failed to connect: fixture"), "{message}");
    assert!(message.contains("tool discovery"), "{message}");
    assert_eq!(cached, 0);
    Ok(())
}
