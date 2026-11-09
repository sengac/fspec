//! BUG-399: a failed session creation is an error, never a placeholder id.
//!
//! Feature: spec/features/session-creation-failures-are-errors.feature

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::mcp_dispatch::{self, InvokeError};
use codelet_rpc::{FspecService, FspecServiceClient, FspecServiceImpl, SharedFspecService};
use codelet_sessions::SessionManager;
use futures::StreamExt;
use tarpc::server::Channel;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn watcher(workspace: &Path) -> Result<Arc<WorkUnitsWatcher>, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(workspace.join("spec"))?;
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    Ok(Arc::new(WorkUnitsWatcher::new(workspace)?))
}

/// An in-process tarpc client for `shared`, served on the current runtime.
fn client(shared: SharedFspecService) -> FspecServiceClient {
    let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
    let service = FspecServiceImpl::new(Arc::new(shared));
    tokio::spawn(
        tarpc::server::BaseChannel::with_defaults(server_transport)
            .execute(service.serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            }),
    );
    FspecServiceClient::new(tarpc::client::Config::default(), client_transport).spawn()
}

/// Invoke create_session through the MCP dispatch and return the daemon's
/// rejection, failing the test if the call succeeded or failed otherwise.
async fn create_session_rejection(client: &FspecServiceClient) -> Result<String, String> {
    creation_rejection(client, "create_session").await
}

/// The daemon's rejection of the creation `method` invoked through MCP.
async fn creation_rejection(client: &FspecServiceClient, method: &str) -> Result<String, String> {
    match mcp_dispatch::invoke(client, method, serde_json::json!({ "role": null })).await {
        Err(InvokeError::Daemon(message)) => Ok(message),
        Ok(value) => Err(format!("{method} succeeded with {value}")),
        Err(other) => Err(format!("{method} failed outside the daemon: {other}")),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_creation_failure_reaches_the_rpc_create_session_caller_as_an_error() -> TestResult {
    // @step Given a daemon over a real SessionManager whose default model is the malformed model string "not-a-model-string"
    let data = tempfile::tempdir()?;
    let workspace = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model("not-a-model-string")?;
    let handle: Arc<dyn SessionManagerHandle> = manager.clone();
    let client = client(SharedFspecService::with_session_manager(
        watcher(workspace.path())?,
        handle,
    ));

    // @step When a client invokes create_session over the RPC
    let rejection = create_session_rejection(&client).await?;

    // @step Then the call fails with the daemon's model parse error
    assert!(
        rejection.contains("Invalid model string 'not-a-model-string'"),
        "unexpected rejection: {rejection}"
    );

    // @step And the session manager holds no live session
    assert!(manager.list_live_sessions()?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_daemon_without_a_session_manager_refuses_create_session() -> TestResult {
    // @step Given a daemon built without a session manager
    let workspace = tempfile::tempdir()?;
    let client = client(SharedFspecService::new(watcher(workspace.path())?));

    // @step When a client invokes create_session over the RPC
    let rejection = create_session_rejection(&client).await?;

    // @step Then the call fails with an error saying session creation requires a session manager
    assert_eq!(rejection, "Session creation requires a session manager");
    Ok(())
}

#[test]
fn the_session_manager_handle_has_no_infallible_session_creator() -> TestResult {
    // @step Given the SessionManagerHandle trait source
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/src/session_manager_handle/handle_trait_1.rs");
    let source = std::fs::read_to_string(path)?;

    // @step When its session creation methods are listed
    let creators: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("fn create_session"))
        .collect();

    // @step Then no create_session method returns a bare SessionId
    assert!(
        creators.iter().all(|line| !line.ends_with("-> SessionId;")),
        "infallible creator still declared: {creators:?}"
    );

    // @step And create_session_checked is a required method returning Result<SessionId, String>
    assert!(
        creators.contains(
            &"fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String>;"
        ),
        "create_session_checked must be required: {creators:?}"
    );
    Ok(())
}

/// The body of `fn name` in `source`, up to the next `async fn`.
fn method_body<'a>(source: &'a str, name: &str) -> Result<&'a str, String> {
    let signature = format!("async fn {name}(");
    let start = source
        .find(&signature)
        .ok_or(format!("{name} is not implemented"))?;
    let rest = &source[start + signature.len()..];
    let end = rest.find("async fn ").unwrap_or(rest.len());
    Ok(&rest[..end])
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn the_checked_creator_is_a_wire_compatibility_alias_of_create_session() -> TestResult {
    // @step Given a daemon built without a session manager
    let workspace = tempfile::tempdir()?;
    let client = client(SharedFspecService::new(watcher(workspace.path())?));

    // @step When a client invokes create_session_checked over the RPC
    let rejection = creation_rejection(&client, "create_session_checked").await?;

    // @step Then the call fails with an error saying session creation requires a session manager
    assert_eq!(rejection, "Session creation requires a session manager");

    // @step And the implementation of create_session_checked delegates to create_session
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let implementation = std::fs::read_to_string(
        root.join("rpc/src/service_implementation/interaction_sessions.rs"),
    )?;
    let alias = method_body(&implementation, "create_session_checked")?;
    assert!(
        alias.contains("create_session(self"),
        "create_session_checked must delegate to create_session: {alias}"
    );
    assert!(
        !alias.contains("session_manager()"),
        "create_session_checked must not repeat the creation logic: {alias}"
    );

    // @step And the TUI transports create sessions with create_session
    for transport in [
        "fspec-tui/src/transport/embedded/sessions.rs",
        "fspec-tui/src/transport/websocket/provider_sessions.rs",
    ] {
        let source = std::fs::read_to_string(root.join(transport))?;
        assert!(
            !source.contains(".create_session_checked("),
            "{transport} must call create_session, not the alias"
        );
        assert!(
            source.contains(".create_session(context::current()"),
            "{transport}"
        );
    }
    Ok(())
}
