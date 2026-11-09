//! MCP-003: duplicate names, unknown sessions, destroy, HTTP and wire order.
//!
//! Feature: spec/features/session-mcp-server-lifecycle.feature

use super::support::*;
use codelet_rpc_types::{SessionId, SessionMcpServer, SessionMcpTransport};
use tarpc::context;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn duplicate_server_names_in_one_request_are_refused() -> TestResult {
    // @step Given a token-armed daemon with an idle session A whose server "notes" is configured
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let client = daemon.client(Some(TOKEN)).await?;
    let rpc = client.client();
    rpc.configure_session_mcp_servers(
        context::current(),
        session.clone(),
        vec![stdio("notes", &[])],
    )
    .await??;

    // @step When a bearer-authenticated client configures "notes" and two servers both named "docs"
    let twice = rpc
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &[]), stdio("docs", &[]), stdio("docs", &[])],
        )
        .await?;

    // @step Then the request fails naming "docs"
    assert!(twice.expect_err("duplicate in request").contains("docs"));

    // @step And session A still has exactly one connection named "notes"
    assert_eq!(
        connection_names(&session).await?,
        Some(vec!["notes".to_string()])
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_unknown_session_is_refused() -> TestResult {
    // @step Given a token-armed daemon
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let missing = SessionId {
        value: uuid::Uuid::new_v4().to_string(),
    };

    // @step When a bearer-authenticated client configures a server for a session that does not exist
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            missing.clone(),
            vec![stdio("notes", &[])],
        )
        .await?;

    // @step Then the request fails naming the session
    assert!(result
        .expect_err("unknown session")
        .contains(&missing.value));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn destroying_a_session_releases_its_configured_servers() -> TestResult {
    // @step Given a token-armed daemon with session A whose server "notes" is configured
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let client = daemon.client(Some(TOKEN)).await?;
    let rpc = client.client();
    rpc.configure_session_mcp_servers(
        context::current(),
        session.clone(),
        vec![stdio("notes", &[])],
    )
    .await??;

    // @step When session A is destroyed
    rpc.destroy_session(context::current(), session.clone())
        .await??;

    // @step Then session A's MCP registry entry is gone
    assert_eq!(connection_names(&session).await?, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_streamable_http_mcp_server_is_configured_for_a_session() -> TestResult {
    // @step Given a token-armed daemon with an idle session A and a local Streamable HTTP MCP server
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let (mut server, url) = super::http_server::http_fixture(None)?;

    // @step When a bearer-authenticated client configures http server "remote" with that url for session A
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![SessionMcpServer {
                name: "remote".to_string(),
                transport: SessionMcpTransport::Http {
                    url,
                    headers: std::collections::BTreeMap::new(),
                },
            }],
        )
        .await?;
    server.kill()?;

    // @step Then the response lists server "remote" with its tool "search"
    let configured = result?;
    assert_eq!(configured[0].name, "remote");
    assert_eq!(configured[0].tools, vec!["mcp__remote__search".to_string()]);
    Ok(())
}

#[test]
fn configure_session_mcp_servers_is_appended_after_every_existing_method() -> TestResult {
    // @step Given the FspecService methods in declaration order
    let rpc = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rpc");
    let mut methods = Vec::new();
    for fragment in [
        "workspace.rs",
        "providers_sessions.rs",
        "supervision_credentials.rs",
        "session_mcp.rs",
    ] {
        let path = rpc.join("src/service_declaration").join(fragment);
        for line in std::fs::read_to_string(path)?.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("async fn ") {
                let name = rest.split(['(', '<']).next().ok_or("empty method line")?;
                methods.push(name.to_string());
            }
        }
    }
    let frozen = std::fs::read_to_string(rpc.join("tests/fixtures/wire_order_frozen.txt"))?;

    // @step When the index of configure_session_mcp_servers is read
    let index = methods
        .iter()
        .position(|name| name == "configure_session_mcp_servers");

    // @step Then configure_session_mcp_servers keeps index 129, after every method declared before it
    assert_eq!(index, Some(129));
    let listed: Vec<&str> = frozen
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(
        listed.get(129).copied(),
        Some("configure_session_mcp_servers")
    );
    Ok(())
}
