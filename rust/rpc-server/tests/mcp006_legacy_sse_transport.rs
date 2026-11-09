//! MCP-006: the legacy SSE MCP client transport.
//!
//! Feature: spec/features/mcp-legacy-sse-transport.feature
//!
//! Run with `--features mcp-fixture` (the fixture MCP server binary).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "mcp003/http_server.rs"]
mod http_server;
#[path = "mcp003/support.rs"]
mod support;

use codelet_rpc_types::{SessionMcpServer, SessionMcpTransport};
use codelet_tools::mcp::{
    cleanup_mcp_session, connect_sse, disconnect_mcp, get_mcp_connections, init_mcp_session,
    new_mcp_connection_map, route_mcp_tool_call, ConnectMcpTool, McpConnectArgs, McpConnectResult,
    McpConnectionMap,
};
use http_server::{fixture_server, http_fixture};
use rig::tool::Tool;
use std::collections::{BTreeMap, HashMap};
use support::*;
use tarpc::context;

/// Start the fixture's legacy SSE server with `args` and return it with its SSE url.
fn sse_fixture(args: &[&str]) -> Boxed<(std::process::Child, String)> {
    let (child, port) = fixture_server(args)?;
    Ok((child, format!("http://127.0.0.1:{port}/sse")))
}

async fn connect(
    name: &str,
    url: &str,
    headers: Option<HashMap<String, String>>,
    timeout_secs: u64,
) -> (McpConnectResult, McpConnectionMap) {
    let (injection_tx, _injection_rx) = tokio::sync::mpsc::channel(8);
    let connections = new_mcp_connection_map();
    let result = connect_sse(
        name,
        url,
        headers.as_ref(),
        timeout_secs,
        injection_tx,
        connections.clone(),
    )
    .await;
    (result, connections)
}

fn bearer(token: &str) -> HashMap<String, String> {
    HashMap::from([("Authorization".to_string(), format!("Bearer {token}"))])
}

fn tool_list(result: &McpConnectResult) -> Vec<String> {
    result
        .tools
        .iter()
        .flatten()
        .map(|tool| tool.name.clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connectmcp_connects_a_bearer_guarded_legacy_sse_server_and_calls_its_tool() -> TestResult {
    // @step Given a legacy SSE MCP server that requires the bearer "token-a" on every request
    let (mut server, url) = sse_fixture(&["--sse", "--bearer", "token-a"])?;

    // @step When the agent calls ConnectMCP for sse server "legacy" with header Authorization "Bearer token-a"
    let session = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session);
    let args: McpConnectArgs = serde_json::from_value(serde_json::json!({
        "action": "connect",
        "name": "legacy",
        "transport": "sse",
        "url": url,
        "headers": { "Authorization": "Bearer token-a" },
    }))?;
    let output = ConnectMcpTool::new(session).call(args).await?;

    // @step Then the connection lists the tool "search"
    assert!(output.contains("search"), "{output}");

    // @step And calling "mcp__legacy__search" returns the fixture result
    let connections = get_mcp_connections(session).ok_or("no MCP registry")?;
    let result =
        route_mcp_tool_call("mcp__legacy__search", serde_json::json!({}), &connections).await;
    cleanup_mcp_session(session);
    server.kill()?;
    assert!(result?.contains("fixture result"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_legacy_sse_server_that_requires_credentials_refuses_a_connection_without_headers(
) -> TestResult {
    // @step Given a legacy SSE MCP server that requires the bearer "token-a" on every request
    let (mut server, url) = sse_fixture(&["--sse", "--bearer", "token-a"])?;

    // @step When connect_sse connects server "legacy" without headers
    let (result, _connections) = connect("legacy", &url, None, 10).await;
    server.kill()?;

    // @step Then the connect fails saying the server requires authentication
    assert!(!result.success);
    assert!(
        result.message.contains("Server requires authentication"),
        "{}",
        result.message
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unreachable_sse_url_fails_the_connect() -> TestResult {
    // @step Given no server listening on a local port
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();

    // @step When connect_sse connects server "gone" to that port
    let (result, _connections) =
        connect("gone", &format!("http://127.0.0.1:{port}/sse"), None, 10).await;

    // @step Then the connect fails naming server "gone"
    assert!(!result.success);
    assert!(result.message.contains("gone"), "{}", result.message);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_server_that_never_sends_the_endpoint_event_times_out() -> TestResult {
    // @step Given a legacy SSE MCP server that never sends its endpoint event
    let (mut server, url) = sse_fixture(&["--sse-silent"])?;

    // @step When connect_sse connects server "silent" with a timeout of 1 second
    let (result, _connections) = connect("silent", &url, None, 1).await;
    server.kill()?;

    // @step Then the connect fails with a timeout naming server "silent"
    assert!(!result.success);
    assert!(
        result.message.contains("Timeout") && result.message.contains("silent"),
        "{}",
        result.message
    );
    // TEST-042: no wall-clock bound; the "Timeout" outcome is the 1 s
    // connect timeout's own result.
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_endpoint_on_another_origin_is_refused() -> TestResult {
    // @step Given a legacy SSE MCP server whose endpoint event names "http://other.invalid/messages"
    let (mut server, url) = sse_fixture(&["--sse-foreign"])?;

    // @step When connect_sse connects server "foreign" with header Authorization "Bearer token-a"
    let (result, _connections) = connect("foreign", &url, Some(bearer("token-a")), 10).await;
    server.kill()?;

    // @step Then the connect fails saying the endpoint is on another origin
    assert!(!result.success);
    assert!(
        result.message.contains("another origin"),
        "{}",
        result.message
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_streamable_http_server_is_not_mistaken_for_a_legacy_sse_server() -> TestResult {
    // @step Given a Streamable HTTP MCP server that refuses GET requests
    let (mut server, url) = http_fixture(None)?;

    // @step When connect_sse connects server "streamable" to its url
    let (result, connections) = connect("streamable", &url, None, 10).await;
    server.kill()?;

    // @step Then the connect fails naming server "streamable"
    assert!(!result.success);
    assert!(result.message.contains("streamable"), "{}", result.message);

    // @step And no connection named "streamable" exists
    assert!(!connections.read().await.contains_key("streamable"));
    Ok(())
}

fn sse(name: &str, url: &str, headers: &HashMap<String, String>) -> SessionMcpServer {
    SessionMcpServer {
        name: name.to_string(),
        transport: SessionMcpTransport::Sse {
            url: url.to_string(),
            headers: headers.clone().into_iter().collect(),
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_daemon_client_configures_a_legacy_sse_server_for_one_session_only() -> TestResult {
    // @step Given a token-armed daemon with idle sessions A and B and a legacy SSE MCP server requiring the bearer "token-a"
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session_a = daemon.session().await?;
    let session_b = daemon.session().await?;
    let (mut server, url) = sse_fixture(&["--sse", "--bearer", "token-a"])?;

    // @step When a bearer-authenticated client configures sse server "legacy" with header Authorization "Bearer token-a" for session A and stdio server "notes" for session B
    let client = daemon.client(Some(TOKEN)).await?;
    let rpc = client.client();
    let configured_a = rpc
        .configure_session_mcp_servers(
            context::current(),
            session_a.clone(),
            vec![sse("legacy", &url, &bearer("token-a"))],
        )
        .await?;
    let configured_b = rpc
        .configure_session_mcp_servers(
            context::current(),
            session_b.clone(),
            vec![stdio("notes", &[])],
        )
        .await?;
    configured_a?;
    configured_b?;

    // @step Then session A's MCP tools are exactly "mcp__legacy__search"
    let tools_a = tool_names(&session_a).await?;
    server.kill()?;
    assert_eq!(tools_a, vec!["mcp__legacy__search".to_string()]);

    // @step And session B's MCP tools are exactly "mcp__notes__search"
    assert_eq!(
        tool_names(&session_b).await?,
        vec!["mcp__notes__search".to_string()]
    );
    assert_eq!(
        connection_names(&session_b).await?,
        Some(vec!["notes".to_string()])
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_disconnected_sse_server_can_be_connected_again() -> TestResult {
    // @step Given a legacy SSE MCP server and a connection "legacy" to it
    let (mut server, url) = sse_fixture(&["--sse"])?;
    let (first, connections) = connect("legacy", &url, None, 10).await;
    assert!(first.success, "{}", first.message);

    // @step When the connection is disconnected and connect_sse connects "legacy" again
    let disconnected = disconnect_mcp("legacy", &connections).await;
    assert!(disconnected.success, "{}", disconnected.message);
    let (second, _connections) = connect("legacy", &url, None, 10).await;
    server.kill()?;

    // @step Then the new connection lists the tool "search"
    assert_eq!(
        tool_list(&second),
        vec!["search".to_string()],
        "{}",
        second.message
    );
    Ok(())
}

#[test]
fn debug_output_of_an_sse_transport_hides_the_url_and_header_values() {
    // @step Given an SSE session MCP transport with url "https://mcp.example/sse-secret" and header Authorization "Bearer secret-token"
    let transport = SessionMcpTransport::Sse {
        url: "https://mcp.example/sse-secret".to_string(),
        headers: BTreeMap::from([(
            "Authorization".to_string(),
            "Bearer secret-token".to_string(),
        )]),
    };

    // @step When it is formatted with Debug
    let output = format!("{transport:?}");

    // @step Then the output contains "Authorization"
    assert!(output.contains("Authorization"), "{output}");

    // @step And the output contains neither "sse-secret" nor "secret-token"
    assert!(
        !output.contains("sse-secret") && !output.contains("secret-token"),
        "{output}"
    );
}
