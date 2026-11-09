//! Feature: spec/features/questions-and-approvals-are-requests-answered-by-id.feature
//!
//! AMGR-020: a client MCP reply to an approval must name the pending
//! request's id, read from get_pause_state. The MCP method schema rejects a
//! pause_triple call without request_id before anything is sent; with the
//! id, the reply reaches the daemon over the RPC and answers the approval.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::mcp_dispatch::{self, InvokeError};
use codelet_rpc::{FspecService, FspecServiceClient, FspecServiceImpl, SharedFspecService};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
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

/// Cancels whatever the session still waits on so the waiting thread never
/// outlives the test.
struct Release(Arc<BackgroundSession>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.cancel_pending_interactions();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_client_mcp_replies_must_name_the_request_id() -> TestResult {
    let data = tempfile::tempdir()?;
    let workspace = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    // The session selects a model at creation; no request is ever sent.
    std::env::set_var("ANTHROPIC_API_KEY", "offline");
    let manager = SessionManager::new();
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: Arc<dyn SessionManagerHandle> = manager.clone();
    let id = handle.create_session_checked(None)?;
    let session = manager.get_session(&id.value)?;
    let _release = Release(session.clone());
    let client = client(SharedFspecService::with_session_manager(
        watcher(workspace.path())?,
        handle,
    ));

    // @step Given a session waits on a blocklist approval
    session.set_pause_state(Some(PauseState {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Command requires approval".into(),
        details: Some("echo amgr020 git reset --hard".into()),
        tool_call_id: None,
    }));
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_pause_response());
    });

    // @step When a client MCP pause_triple call omits request_id and then names the id from get_pause_state
    let without_id = serde_json::json!({"session_id": id, "choice": "Approve"});
    let rejected = mcp_dispatch::validate("pause_triple", without_id);
    let pending = mcp_dispatch::invoke(
        &client,
        "get_pause_state",
        serde_json::json!({"session_id": id}),
    )
    .await?;
    let request_id = pending["route"]["request_id"]
        .as_str()
        .ok_or("get_pause_state returned no request id")?
        .to_string();
    let with_id =
        serde_json::json!({"session_id": id, "request_id": request_id, "choice": "Approve"});
    mcp_dispatch::validate("pause_triple", with_id.clone())?;
    mcp_dispatch::invoke(&client, "pause_triple", with_id).await?;

    // @step Then the first fails schema validation and the second answers the approval
    match rejected {
        Err(InvokeError::Arguments(message)) => {
            assert!(
                message.contains("request_id"),
                "unexpected error: {message}"
            )
        }
        other => return Err(format!("pause_triple without request_id: {other:?}").into()),
    }
    let response =
        codelet_test_helpers::deadlock_guard("the approval is answered", answer).await??;
    assert_eq!(response, PauseResponse::AllowOnce);
    assert!(session.pending_pause_route().is_none());
    Ok(())
}
