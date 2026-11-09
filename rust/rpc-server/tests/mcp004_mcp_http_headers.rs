//! MCP-004: the MCP HTTP transport sends configured headers.
//!
//! Feature: spec/features/mcp-http-transport-headers.feature
//!
//! Run with `--features mcp-fixture` (the fixture MCP server binary).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "mcp003/http_server.rs"]
mod http_server;
#[path = "mcp003/support.rs"]
mod support;

use codelet_rpc_types::{SessionMcpServer, SessionMcpTransport};
use codelet_tools::mcp::{
    connect_http, new_mcp_connection_map, route_mcp_tool_call, McpConnectResult, McpConnectionMap,
};
use http_server::http_fixture;
use std::collections::{BTreeMap, HashMap};
use support::*;
use tarpc::context;

/// Connect `name` to `url` through the shared connect_http path with `headers`.
async fn connect(
    name: &str,
    url: &str,
    headers: Option<HashMap<String, String>>,
) -> (McpConnectResult, McpConnectionMap) {
    let (injection_tx, _injection_rx) = tokio::sync::mpsc::channel(8);
    let connections = new_mcp_connection_map();
    let result = connect_http(
        name,
        url,
        headers.as_ref(),
        10,
        injection_tx,
        connections.clone(),
    )
    .await;
    (result, connections)
}

fn header(name: &str, value: &str) -> HashMap<String, String> {
    HashMap::from([(name.to_string(), value.to_string())])
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
async fn a_configured_bearer_header_authenticates_the_handshake_discovery_and_a_tool_call(
) -> TestResult {
    // @step Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    let (mut server, url) = http_fixture(Some("token-a"))?;

    // @step When connect_http connects server "remote" with header Authorization "Bearer token-a"
    let (result, connections) = connect(
        "remote",
        &url,
        Some(header("Authorization", "Bearer token-a")),
    )
    .await;

    // @step Then the connection lists the tool "search"
    assert!(result.success, "{}", result.message);
    assert_eq!(tool_list(&result), vec!["search".to_string()]);

    // @step And calling "mcp__remote__search" returns the fixture result
    let output =
        route_mcp_tool_call("mcp__remote__search", serde_json::json!({}), &connections).await;
    server.kill()?;
    assert!(output?.contains("fixture result"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_server_that_requires_credentials_refuses_a_connection_without_headers() -> TestResult {
    // @step Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    let (mut server, url) = http_fixture(Some("token-a"))?;

    // @step When connect_http connects server "remote" without headers
    let (result, _connections) = connect("remote", &url, None).await;
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
async fn a_reserved_mcp_header_is_refused() -> TestResult {
    // @step Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    let (mut server, url) = http_fixture(Some("token-a"))?;

    // @step When connect_http connects server "remote" with header Mcp-Session-Id "forged"
    let (result, _connections) =
        connect("remote", &url, Some(header("Mcp-Session-Id", "forged"))).await;
    server.kill()?;

    // @step Then the connect fails naming the reserved header "mcp-session-id"
    assert!(!result.success);
    assert!(
        result.message.contains("mcp-session-id"),
        "{}",
        result.message
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_connections_each_send_only_their_own_token() -> TestResult {
    // @step Given two Streamable HTTP MCP servers requiring the bearers "token-a" and "token-b"
    let (mut server_a, url_a) = http_fixture(Some("token-a"))?;
    let (mut server_b, url_b) = http_fixture(Some("token-b"))?;

    // @step When server "alpha" with token "token-a" and server "beta" with token "token-b" connect concurrently
    let ((alpha, _), (beta, _)) = tokio::join!(
        connect(
            "alpha",
            &url_a,
            Some(header("Authorization", "Bearer token-a"))
        ),
        connect(
            "beta",
            &url_b,
            Some(header("Authorization", "Bearer token-b"))
        ),
    );
    server_a.kill()?;
    server_b.kill()?;

    // @step Then both connections list the tool "search"
    assert_eq!(
        tool_list(&alpha),
        vec!["search".to_string()],
        "{}",
        alpha.message
    );
    assert_eq!(
        tool_list(&beta),
        vec!["search".to_string()],
        "{}",
        beta.message
    );
    Ok(())
}

fn http(name: &str, url: &str, headers: &[(&str, &str)]) -> SessionMcpServer {
    SessionMcpServer {
        name: name.to_string(),
        transport: SessionMcpTransport::Http {
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_daemon_client_configures_an_http_mcp_server_with_headers_for_a_session() -> TestResult {
    // @step Given a token-armed daemon with an idle session A and an HTTP MCP server requiring the bearer "token-a"
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let (mut server, url) = http_fixture(Some("token-a"))?;

    // @step When a bearer-authenticated client configures http server "remote" with header Authorization "Bearer token-a"
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![http("remote", &url, &[("Authorization", "Bearer token-a")])],
        )
        .await?;
    server.kill()?;

    // @step Then the response lists server "remote" with its tool "search"
    let configured = result?;
    assert_eq!(configured[0].name, "remote");
    assert_eq!(configured[0].tools, vec!["mcp__remote__search".to_string()]);
    assert!(tool_names(&session)
        .await?
        .contains(&"mcp__remote__search".to_string()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_invalid_header_value_is_refused_without_revealing_it() -> TestResult {
    // @step Given a token-armed daemon with an idle session A and an HTTP MCP server requiring the bearer "token-a"
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let (mut server, url) = http_fixture(Some("token-a"))?;

    // @step When a bearer-authenticated client configures stdio server "good" and http server "remote" with header X-Token "secret-value\nsplit"
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![
                stdio("good", &[]),
                http("remote", &url, &[("X-Token", "secret-value\nsplit")]),
            ],
        )
        .await?;
    server.kill()?;

    // @step Then the request fails with an error naming the header "X-Token"
    let error = result.err().ok_or("the configuration was accepted")?;
    assert!(error.contains("X-Token"), "{error}");

    // @step And the error does not contain "secret-value"
    assert!(!error.contains("secret-value"), "{error}");

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}

#[test]
fn debug_output_of_an_http_transport_shows_header_names_but_never_values() {
    // @step Given an HTTP session MCP transport with header Authorization "Bearer secret-token"
    let transport = SessionMcpTransport::Http {
        url: "https://mcp.example/mcp".to_string(),
        headers: BTreeMap::from([(
            "Authorization".to_string(),
            "Bearer secret-token".to_string(),
        )]),
    };

    // @step When it is formatted with Debug
    let output = format!("{transport:?}");

    // @step Then the output contains "Authorization"
    assert!(output.contains("Authorization"), "{output}");

    // @step And the output does not contain "secret-token"
    assert!(!output.contains("secret-token"), "{output}");
}
