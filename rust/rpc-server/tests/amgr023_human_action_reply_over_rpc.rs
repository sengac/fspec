//! Feature: spec/features/a-human-action-has-its-own-done-or-cancel-reply.feature
//!
//! AMGR-023: pause_human_action is the one reply to a human action. It is
//! appended to FspecService (RPC-449); pause_resume keeps its frozen variant
//! index but answers nothing. The client MCP lists pause_human_action and no
//! pause_resume, and a call naming the request id answers the human action
//! over the RPC.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::mcp_dispatch;
use codelet_rpc::{FspecService, FspecServiceClient, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::SessionId;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use futures::StreamExt;
use tarpc::server::Channel;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn watcher(workspace: &Path) -> TestResult<Arc<WorkUnitsWatcher>> {
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

/// The data directory is process-wide, so the tests (which run in parallel)
/// share one, set once and alive until the process exits (BUG-406).
fn data_directory() -> Result<(), String> {
    static DATA: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let data = codelet_test_helpers::shared_temp_subdirectory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        // The session selects a model at creation; no request is ever sent.
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        Ok(())
    })
    .clone()
}

/// A daemon with one session and an in-process client.
struct Daemon {
    client: FspecServiceClient,
    id: SessionId,
    session: Arc<BackgroundSession>,
    _release: Release,
    _workspace: tempfile::TempDir,
}

impl Daemon {
    fn start() -> TestResult<Self> {
        data_directory()?;
        let workspace = tempfile::tempdir()?;
        let manager = SessionManager::new();
        manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
        manager.set_default_model("anthropic/claude-opus-4-5")?;
        let handle: Arc<dyn SessionManagerHandle> = manager.clone();
        let id = handle.create_session_checked(None)?;
        let session = manager.get_session(&id.value)?;
        let client = client(SharedFspecService::with_session_manager(
            watcher(workspace.path())?,
            handle,
        ));
        Ok(Self {
            client,
            id,
            _release: Release(session.clone()),
            session,
            _workspace: workspace,
        })
    }

    /// Raise a WebSearch human action and wait for it on its own thread.
    fn hold_human_action(
        &self,
    ) -> TestResult<(String, tokio::sync::oneshot::Receiver<PauseResponse>)> {
        self.session.set_pause_state(Some(PauseState {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".into(),
            message: "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.".into(),
            details: None,
            tool_call_id: None,
        }));
        let (answered, answer) = tokio::sync::oneshot::channel();
        let waiting = self.session.clone();
        std::thread::spawn(move || {
            let _ = answered.send(waiting.wait_for_pause_response());
        });
        let route = self
            .session
            .pending_pause_route()
            .ok_or("the session has no pending human action")?;
        Ok((route.request_id, answer))
    }
}

fn rpc_crate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../rpc")
}

/// FspecService method names in declaration (variant-index) order, read as
/// RPC-449's wire-order test reads them.
fn declared_methods() -> TestResult<Vec<String>> {
    let dir = rpc_crate().join("src/service_declaration");
    let mut names = Vec::new();
    for fragment in [
        "workspace.rs",
        "providers_sessions.rs",
        "supervision_credentials.rs",
        "session_mcp.rs",
    ] {
        for line in std::fs::read_to_string(dir.join(fragment))?.lines() {
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("async fn ") {
                names.push(
                    rest.split(['(', '<'])
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                );
            }
        }
    }
    Ok(names)
}

fn frozen_methods() -> TestResult<Vec<String>> {
    let path = rpc_crate().join("tests/fixtures/wire_order_frozen.txt");
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn index_of(methods: &[String], name: &str) -> TestResult<usize> {
    Ok(methods
        .iter()
        .position(|method| method == name)
        .ok_or(format!("{name} is not declared"))?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_pause_resume_keeps_its_frozen_index_and_answers_nothing() -> TestResult {
    // @step Given the FspecService methods in declaration order and the frozen wire-order fixture
    let declared = declared_methods()?;
    let frozen = frozen_methods()?;

    // @step When pause_resume and pause_human_action are looked up
    let resume = index_of(&declared, "pause_resume")?;
    let human_action = index_of(&declared, "pause_human_action")?;

    // @step Then pause_resume keeps its frozen index and pause_human_action is the last method
    assert_eq!(resume, index_of(&frozen, "pause_resume")?);
    assert_eq!(human_action, declared.len() - 1);
    assert_eq!(human_action, index_of(&frozen, "pause_human_action")?);

    // @step And a pause_resume call naming a pending human action's request id is rejected naming pause_human_action, and the human action is still pending
    let daemon = Daemon::start()?;
    let (request_id, answer) = daemon.hold_human_action()?;
    let reply = daemon
        .client
        .pause_resume(
            tarpc::context::current(),
            daemon.id.clone(),
            request_id.clone(),
        )
        .await?;
    let error = reply.err().ok_or("pause_resume answered a human action")?;
    assert!(error.contains("pause_human_action"), "{error}");
    assert_eq!(
        daemon
            .session
            .pending_pause_route()
            .map(|route| route.request_id),
        Some(request_id)
    );
    // TEST-042 causality: pause_resume sent nothing, so the wait's first
    // response is the cancel's.
    daemon.session.cancel_pending_interactions();
    let response =
        codelet_test_helpers::deadlock_guard("the human action is cancelled", answer).await??;
    assert_eq!(response, PauseResponse::Interrupted);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_client_mcp_answers_a_human_action_by_request_id() -> TestResult {
    // @step Given the client MCP catalog
    let methods = mcp_dispatch::methods()?;

    // @step Then it lists pause_human_action with session_id, request_id and reply, and no pause_resume
    assert!(
        !methods.iter().any(|method| method.name == "pause_resume"),
        "the client MCP still lists pause_resume"
    );
    let method = methods
        .iter()
        .find(|method| method.name == "pause_human_action")
        .ok_or("the client MCP does not list pause_human_action")?;
    let schema = method.schema.to_string();
    for field in ["session_id", "request_id", "reply", "Done", "Cancel"] {
        assert!(schema.contains(field), "{field} missing from {schema}");
    }

    // @step When pause_human_action is called naming a pending human action's request id with Done
    let daemon = Daemon::start()?;
    let (request_id, answer) = daemon.hold_human_action()?;
    let arguments =
        serde_json::json!({"session_id": daemon.id, "request_id": request_id, "reply": "Done"});
    mcp_dispatch::validate("pause_human_action", arguments.clone())?;
    mcp_dispatch::invoke(&daemon.client, "pause_human_action", arguments).await?;

    // @step Then the human action is answered
    let response =
        codelet_test_helpers::deadlock_guard("the human action is answered", answer).await??;
    assert_eq!(response, PauseResponse::Resumed);
    assert!(daemon.session.pending_pause_route().is_none());
    Ok(())
}
