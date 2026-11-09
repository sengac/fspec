//! MCP-008: configure_session_mcp_servers sets a session's whole MCP server
//! set, under fspec's own MCP handshake timeout.
//!
//! Feature: spec/features/declarative-session-mcp-server-sets.feature

#[path = "mcp003/support.rs"]
mod support;

use codelet_rpc_server::FspecWsClient;
use codelet_rpc_types::{SessionId, SessionMcpServer, SessionMcpTransport};
use codelet_tools::mcp::MCP_HANDSHAKE_TIMEOUT_SECS;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use support::*;
use tarpc::context;

/// MCP-010: a context whose deadline outlasts one handshake timeout, as the
/// feature documents for daemon clients.
fn outlasting_context() -> context::Context {
    let mut ctx = context::current();
    ctx.deadline =
        std::time::SystemTime::now() + Duration::from_secs(MCP_HANDSHAKE_TIMEOUT_SECS + 5);
    ctx
}

/// Stdio server `name` that starts but never answers the handshake.
fn silent(name: &str) -> SessionMcpServer {
    SessionMcpServer {
        transport: SessionMcpTransport::Stdio {
            command: "cat".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
        },
        ..stdio(name, &[])
    }
}

/// When each connection of a session's MCP registry was established, by name.
async fn connections(session: &SessionId) -> Boxed<BTreeMap<String, Instant>> {
    let uuid = uuid::Uuid::parse_str(&session.value)?;
    let map = codelet_tools::mcp::get_mcp_connections(uuid).ok_or("no MCP registry")?;
    let map = map.read().await;
    Ok(map
        .iter()
        .map(|(name, connection)| (name.clone(), connection.connected_at))
        .collect())
}

/// A started daemon, a bearer client and an idle session with `names` configured.
async fn configured(names: &[&str]) -> Boxed<(Daemon, FspecWsClient, SessionId)> {
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let client = daemon.client(Some(TOKEN)).await?;
    let servers = names.iter().map(|name| stdio(name, &[])).collect();
    client
        .client()
        .configure_session_mcp_servers(context::current(), session.clone(), servers)
        .await??;
    Ok((daemon, client, session))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_unlisted_server_is_disconnected_and_an_identical_one_is_kept() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose servers "notes" and "docs" are configured
    let (_daemon, client, session) = configured(&["notes", "docs"]).await?;
    let before = connections(&session).await?;

    // @step When a bearer-authenticated client configures only server "notes" with the same transport
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &[])],
        )
        .await?;

    // @step Then the response lists only server "notes" with its tool "search"
    let configured = result?;
    assert_eq!(configured.len(), 1);
    assert_eq!(configured[0].name, "notes");
    assert_eq!(configured[0].tools, vec!["mcp__notes__search".to_string()]);

    // @step And session A's only MCP connection is the original "notes" connection
    let after = connections(&session).await?;
    assert_eq!(after.keys().collect::<Vec<_>>(), vec!["notes"]);
    assert_eq!(
        after.get("notes"),
        before.get("notes"),
        "notes was reconnected"
    );
    assert_eq!(
        tool_names(&session).await?,
        vec!["mcp__notes__search".to_string()]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_server_whose_transport_changed_is_reconnected() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose server "notes" is configured
    let (_daemon, client, session) = configured(&["notes"]).await?;
    let before = connections(&session).await?;

    // @step When a bearer-authenticated client configures server "notes" with different arguments
    client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &["--argv", "changed"])],
        )
        .await??;

    // @step Then session A's only MCP connection is a new "notes" connection
    let after = connections(&session).await?;
    assert_eq!(after.keys().collect::<Vec<_>>(), vec!["notes"]);
    assert_ne!(
        after.get("notes"),
        before.get("notes"),
        "notes was not reconnected"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_empty_list_disconnects_every_server() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose server "notes" is configured
    let (_daemon, client, session) = configured(&["notes"]).await?;

    // @step When a bearer-authenticated client configures an empty server list for session A
    let result = client
        .client()
        .configure_session_mcp_servers(context::current(), session.clone(), Vec::new())
        .await?;

    // @step Then the response is empty
    assert_eq!(result?, Vec::new());

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_failed_connect_leaves_the_previous_set_intact() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose servers "notes" and "docs" are configured
    let (_daemon, client, session) = configured(&["notes", "docs"]).await?;
    let before = connections(&session).await?;

    // @step When a bearer-authenticated client configures "notes" with different arguments and "broken" where "broken" exits before its handshake
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![
                stdio("notes", &["--argv", "changed"]),
                stdio("broken", &["--exit"]),
            ],
        )
        .await?;

    // @step Then the request fails with an error naming "broken"
    let error = result.err().ok_or("broken was accepted")?;
    assert!(error.contains("broken"), "{error}");

    // @step And session A still has the original "notes" and "docs" connections
    assert_eq!(connections(&session).await?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_daemon_configured_server_is_bounded_by_the_fspec_handshake_timeout() -> TestResult {
    // @step Given a token-armed daemon with an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let client = daemon.client(Some(TOKEN)).await?;

    // @step When a bearer-authenticated client configures stdio server "silent" that never answers the handshake
    let result = client
        .client()
        .configure_session_mcp_servers(
            outlasting_context(),
            session.clone(),
            vec![silent("silent")],
        )
        .await?;

    // @step Then the request fails naming "silent" and the MCP_HANDSHAKE_TIMEOUT_SECS value
    let error = result.err().ok_or("silent was accepted")?;
    assert!(error.contains("silent"), "{error}");
    let within = format!("within {MCP_HANDSHAKE_TIMEOUT_SECS}s");
    assert!(error.contains(&within), "{error}");

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}

#[tokio::test]
async fn neither_front_door_takes_a_handshake_timeout() -> TestResult {
    // @step Given the ConnectMCP parameter schema and a SessionMcpServer JSON object carrying timeout_secs
    let tool = codelet_tools::ConnectMcpTool::new(uuid::Uuid::new_v4());
    let carrying = serde_json::json!({
        "name": "notes",
        "transport": { "Stdio": { "command": "fixture", "args": [], "env": {} } },
        "timeout_secs": 10
    });

    // @step When the schema is inspected and the object is deserialized
    let definition = rig::tool::Tool::definition(&tool, String::new()).await;
    let parsed = serde_json::from_value::<SessionMcpServer>(carrying);

    // @step Then the schema has no timeout property
    let properties = definition.parameters["properties"]
        .as_object()
        .ok_or("ConnectMCP schema has no properties")?;
    assert!(properties.contains_key("name"), "{properties:?}");
    assert!(!properties.contains_key("timeout"), "{properties:?}");

    // @step And the deserialization fails naming timeout_secs
    let error = parsed.err().ok_or("timeout_secs was accepted")?;
    assert!(error.to_string().contains("timeout_secs"), "{error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn several_silent_servers_fail_together_within_one_handshake_timeout() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose server "notes" is configured
    let (_daemon, client, session) = configured(&["notes"]).await?;
    let before = connections(&session).await?;

    // @step When a bearer-authenticated client configures "notes" and stdio servers "quiet-a", "quiet-b" and "quiet-c" that never answer the handshake
    let quiet = ["quiet-a", "quiet-b", "quiet-c"];
    let mut servers = vec![stdio("notes", &[])];
    servers.extend(quiet.iter().map(|name| silent(name)));
    let result = client
        .client()
        .configure_session_mcp_servers(outlasting_context(), session.clone(), servers)
        .await?;

    // @step Then the request fails within twice MCP_HANDSHAKE_TIMEOUT_SECS with an error naming "quiet-a", "quiet-b" and "quiet-c"
    // TEST-042: no wall-clock bound; every silent server is named, each by
    // its own handshake timeout.
    let error = result.err().ok_or("the silent servers were accepted")?;
    for name in quiet {
        assert!(error.contains(&format!("'{name}'")), "{error}");
    }

    // @step And session A still has only the original "notes" connection
    assert_eq!(connections(&session).await?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_server_that_never_answers_tools_list_is_bounded_by_the_fspec_handshake_timeout(
) -> TestResult {
    // @step Given a token-armed daemon with an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let client = daemon.client(Some(TOKEN)).await?;

    // @step When a bearer-authenticated client configures stdio server "mute" that completes its handshake but never answers tools/list
    let result = client
        .client()
        .configure_session_mcp_servers(
            outlasting_context(),
            session.clone(),
            vec![stdio("mute", &["--no-tools"])],
        )
        .await?;

    // @step Then the request fails within twice MCP_HANDSHAKE_TIMEOUT_SECS naming "mute" and the MCP_HANDSHAKE_TIMEOUT_SECS value
    // TEST-042: no wall-clock bound; the error names the handshake timeout.
    let error = result.err().ok_or("mute was accepted")?;
    assert!(error.contains("'mute'"), "{error}");
    assert!(
        error.contains(&format!("within {MCP_HANDSHAKE_TIMEOUT_SECS}s")),
        "{error}"
    );

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}
