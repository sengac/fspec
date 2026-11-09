//! MCP-003: daemon clients configure a session's MCP servers.
//!
//! Feature: spec/features/session-mcp-server-configuration.feature
//!
//! Run with `--features mcp-fixture` (the fixture MCP server binary).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "mcp003/http_server.rs"]
mod http_server;
#[path = "mcp003/lifecycle.rs"]
mod lifecycle;
#[path = "mcp003/support.rs"]
mod support;

use codelet_rpc::{FspecService, FspecServiceImpl};
use support::*;
use tarpc::context;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_bearer_authenticated_client_configures_a_stdio_mcp_server_for_a_session() -> TestResult {
    // @step Given a daemon started with FSPEC_DAEMON_TOKEN and an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;

    // @step When a client connected with the daemon bearer configures stdio server "notes" for session A
    let client = daemon.client(Some(TOKEN)).await?;
    let configured = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &[])],
        )
        .await??;

    // @step Then the response lists server "notes" with its tool "search"
    assert_eq!(configured.len(), 1);
    assert_eq!(configured[0].name, "notes");
    assert_eq!(configured[0].tools, vec!["mcp__notes__search".to_string()]);

    // @step And session A's MCP tools include "mcp__notes__search"
    assert!(tool_names(&session)
        .await?
        .contains(&"mcp__notes__search".to_string()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_daemon_without_fspec_daemon_token_refuses_mcp_configuration() -> TestResult {
    // @step Given a daemon started without FSPEC_DAEMON_TOKEN and an idle session A
    let daemon = Daemon::start(None).await?;
    let session = daemon.session().await?;

    // @step When a client configures stdio server "notes" for session A
    let client = daemon.client(None).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &[])],
        )
        .await?;

    // @step Then the request fails with an error naming FSPEC_DAEMON_TOKEN
    let error = result.expect_err("a token-less daemon must refuse");
    assert!(error.contains("FSPEC_DAEMON_TOKEN"), "{error}");

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn an_in_process_client_is_not_bearer_authenticated() -> TestResult {
    // @step Given a token-armed daemon service reached in-process without a bearer handshake
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;
    let in_process = FspecServiceImpl::new(daemon._service.clone());

    // @step When the in-process client configures stdio server "notes" for session A
    let result = in_process
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("notes", &[])],
        )
        .await;

    // @step Then the request fails because the connection did not authenticate with the daemon bearer
    let error = result.expect_err("an in-process connection must be refused");
    assert!(error.contains("bearer"), "{error}");

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_configured_server_is_visible_only_to_its_own_session() -> TestResult {
    // @step Given a token-armed daemon with idle sessions A and B
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session_a = daemon.session().await?;
    let session_b = daemon.session().await?;

    // @step When a bearer-authenticated client configures stdio server "notes" for session A
    let client = daemon.client(Some(TOKEN)).await?;
    client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session_a.clone(),
            vec![stdio("notes", &[])],
        )
        .await??;

    // @step Then session A has an MCP connection named "notes"
    assert_eq!(
        connection_names(&session_a).await?,
        Some(vec!["notes".to_string()])
    );

    // @step And session B has no MCP connections
    assert_eq!(connection_names(&session_b).await?, Some(Vec::new()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_failing_server_rolls_back_the_whole_request() -> TestResult {
    // @step Given a token-armed daemon with an idle session A
    let daemon = Daemon::start(Some(TOKEN)).await?;
    let session = daemon.session().await?;

    // @step When a bearer-authenticated client configures servers "good" and "broken" where "broken" exits before its handshake
    let client = daemon.client(Some(TOKEN)).await?;
    let result = client
        .client()
        .configure_session_mcp_servers(
            context::current(),
            session.clone(),
            vec![stdio("good", &[]), stdio("broken", &["--exit"])],
        )
        .await?;

    // @step Then the request fails with an error naming "broken"
    let error = result.expect_err("a failing server must fail the request");
    assert!(error.contains("broken"), "{error}");
    assert!(
        !error.contains(fixture()),
        "the command must not leak: {error}"
    );

    // @step And session A has no MCP connections
    assert_eq!(connection_names(&session).await?, Some(Vec::new()));
    Ok(())
}
