//! MCP-005: structured stdio MCP command arguments.
//!
//! Feature: spec/features/mcp-stdio-structured-arguments.feature
//!
//! Run with `--features mcp-fixture` (the fixture MCP server binary).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "mcp003/support.rs"]
mod support;

use codelet_rpc_types::{SessionId, SessionMcpServer, SessionMcpTransport};
use codelet_tools::mcp::{
    cleanup_mcp_session, get_mcp_connections, init_mcp_session, route_mcp_tool_call,
    ConnectMcpTool, McpConnectArgs,
};
use codelet_tools::ToolError;
use rig::tool::Tool;
use std::collections::BTreeMap;
use support::*;
use tarpc::context;

/// Call `mcp__<server>__search` in `session` and decode the arguments the fixture received.
async fn received_args(session: uuid::Uuid, server: &str) -> Boxed<Vec<String>> {
    let connections = get_mcp_connections(session).ok_or("no MCP registry")?;
    let text = route_mcp_tool_call(
        &format!("mcp__{server}__search"),
        serde_json::json!({}),
        &connections,
    )
    .await?;
    let start = text.find('[').ok_or("no JSON array in the tool result")?;
    let end = text.rfind(']').ok_or("no JSON array in the tool result")?;
    Ok(serde_json::from_str(&text[start..=end])?)
}

fn session_uuid(session: &SessionId) -> Boxed<uuid::Uuid> {
    Ok(uuid::Uuid::parse_str(&session.value)?)
}

fn literal_stdio(name: &str, command: &str, args: &[&str]) -> SessionMcpServer {
    SessionMcpServer {
        name: name.to_string(),
        transport: SessionMcpTransport::Stdio {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: BTreeMap::new(),
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_daemon_clients_structured_stdio_arguments_reach_the_server_literally() -> TestResult {
    // @step Given a token-armed daemon with an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;

    // @step When a bearer-authenticated client configures stdio server "argv" with arguments "two words", "", "say \"hi\"" and "naïve ☃"
    let expected = ["two words", "", "say \"hi\"", "naïve ☃"];
    let client = daemon.client(Some(TOKEN)).await?;
    let mut args = vec!["--argv"];
    args.extend(expected);
    client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("argv", &args)],
        )
        .await??;

    // @step Then calling "mcp__argv__search" in session A returns exactly those four arguments
    assert_eq!(
        received_args(session_uuid(&session)?, "argv").await?,
        expected
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_executable_whose_path_contains_a_space_is_spawned_literally() -> TestResult {
    // @step Given a token-armed daemon with an idle session A and the fixture MCP server copied into a directory named "dir with space"
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let root = tempfile::tempdir()?;
    let dir = root.path().join("dir with space");
    std::fs::create_dir(&dir)?;
    let executable = dir.join("fspec mcp fixture");
    std::fs::copy(fixture(), &executable)?;
    let executable = executable.to_str().ok_or("non-UTF-8 temp path")?;

    // @step When a bearer-authenticated client configures stdio server "spaced" with that executable and no arguments
    let client = daemon.client(Some(TOKEN)).await?;
    let configured = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![literal_stdio("spaced", executable, &[])],
        )
        .await??;

    // @step Then the response lists server "spaced" with its tool "search"
    assert_eq!(configured[0].name, "spaced");
    assert_eq!(configured[0].tools, vec!["mcp__spaced__search".to_string()]);
    Ok(())
}

/// Run the ConnectMCP tool for a fresh live MCP session.
async fn connect_mcp(args: serde_json::Value) -> Boxed<(uuid::Uuid, String)> {
    let session = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session);
    let tool = ConnectMcpTool::new(session);
    let args: McpConnectArgs = serde_json::from_value(args)?;
    let output = tool.call(args).await?;
    Ok((session, output))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connectmcp_with_args_passes_the_command_and_every_argument_literally() -> TestResult {
    // @step Given a live MCP session and the ConnectMCP tool
    let args = serde_json::json!({
        "action": "connect",
        "name": "argv",
        "transport": "stdio",
        "command": fixture(),
        "args": ["--argv", "a b", ""],
    });

    // @step When the agent calls ConnectMCP for stdio server "argv" with the fixture as command and args "--argv", "a b" and ""
    let (session, output) = connect_mcp(args).await?;
    assert!(output.contains("argv"), "{output}");

    // @step Then calling "mcp__argv__search" returns exactly the arguments "a b" and ""
    let received = received_args(session, "argv").await;
    cleanup_mcp_session(session);
    assert_eq!(received?, ["a b", ""]);
    Ok(())
}

/// MCP-007: run ConnectMCP and return its validation message and the connection count left behind.
async fn rejected_connect(args: serde_json::Value) -> Boxed<(String, usize)> {
    let session = uuid::Uuid::new_v4();
    let (_injection_rx, _connections) = init_mcp_session(session);
    let tool = ConnectMcpTool::new(session);
    let args: McpConnectArgs = serde_json::from_value(args)?;
    let result = tool.call(args).await;
    let connections = get_mcp_connections(session).ok_or("no MCP registry")?;
    let count = connections.read().await.len();
    cleanup_mcp_session(session);
    match result {
        Err(ToolError::Validation { message, .. }) => Ok((message, count)),
        other => Err(format!("expected a validation error, got {other:?}").into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connectmcp_stdio_without_args_is_rejected_naming_args() -> TestResult {
    // @step Given a live MCP session and the ConnectMCP tool
    let args = serde_json::json!({
        "action": "connect",
        "name": "split",
        "transport": "stdio",
        "command": format!("{} --argv one two", fixture()),
    });

    // @step When the agent calls ConnectMCP for stdio server "split" with command "<fixture> --argv one two" and no args
    let (message, connections) = rejected_connect(args).await?;

    // @step Then the call fails with a validation error naming "args"
    assert!(message.contains("args"), "{message}");

    // @step And the session has no MCP connections
    assert_eq!(connections, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connectmcp_stdio_with_an_empty_command_is_rejected_naming_command() -> TestResult {
    // @step Given a live MCP session and the ConnectMCP tool
    let args = serde_json::json!({
        "action": "connect",
        "name": "blank",
        "transport": "stdio",
        "command": "   ",
        "args": [],
    });

    // @step When the agent calls ConnectMCP for stdio server "blank" with command "   " and an empty args list
    let (message, connections) = rejected_connect(args).await?;

    // @step Then the call fails with a validation error naming "command"
    assert!(message.contains("command"), "{message}");

    // @step And the session has no MCP connections
    assert_eq!(connections, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_missing_stdio_executable_fails_without_revealing_the_command_or_its_arguments(
) -> TestResult {
    // @step Given a token-armed daemon with an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;

    // @step When a bearer-authenticated client configures stdio server "missing" with executable "/nonexistent dir/mcp-secret-bin" and argument "--token=arg-secret"
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![literal_stdio(
                "missing",
                "/nonexistent dir/mcp-secret-bin",
                &["--token=arg-secret"],
            )],
        )
        .await?;

    // @step Then the request fails with an error naming "missing"
    let error = result.err().ok_or("the configuration was accepted")?;
    assert!(error.contains("'missing'"), "{error}");

    // @step And the error contains neither "mcp-secret-bin" nor "arg-secret"
    assert!(
        !error.contains("mcp-secret-bin") && !error.contains("arg-secret"),
        "{error}"
    );

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    assert!(tool_names(&session).await?.is_empty());
    Ok(())
}

#[test]
fn debug_output_of_a_stdio_transport_hides_the_command_and_its_arguments() {
    // @step Given a stdio session MCP transport with command "/opt/secret-bin" and argument "--token=arg-secret"
    let transport = literal_stdio("debug", "/opt/secret-bin", &["--token=arg-secret"]).transport;

    // @step When it is formatted with Debug
    let output = format!("{transport:?}");

    // @step Then the output contains neither "secret-bin" nor "arg-secret"
    assert!(
        !output.contains("secret-bin") && !output.contains("arg-secret"),
        "{output}"
    );
}
