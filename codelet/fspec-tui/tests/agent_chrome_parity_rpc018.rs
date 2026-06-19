//! RPC-018 — Cross-transport parity for `FspecBackend::get_model_info`,
//! `get_thinking_level`, `get_workspace_info`.
//!
//! Feature: spec/features/rpc018-cross-transport-parity.feature
//!
//! Mirrors the RPC-015 cross-transport-parity pattern: drives the SAME
//! scripted scenario against BOTH transports and asserts identical
//! results.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::session_manager_handle::StubSessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{ModelInfo, SessionId, ThinkingLevel, WorkspaceInfo};
use tempfile::TempDir;

fn init_git_repo_on_branch(branch: &str) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let repo_path = tmp.path();
    Command::new("git")
        .args(["init", "-b", branch])
        .current_dir(repo_path)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("config email");
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path)
        .output()
        .expect("config name");
    fs::write(repo_path.join("README.md"), "# x\n").expect("write README");
    fs::create_dir_all(repo_path.join("spec")).expect("mkdir spec");
    fs::write(
        repo_path.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo_path)
        .output()
        .expect("git commit");
    tmp
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

fn service_without_cwd() -> Arc<SharedFspecService> {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    // Leak the tempdir so the watcher path stays valid for the lifetime of
    // the test — we only need the service surface, not the tempdir handle.
    Box::leak(Box::new(tmp));
    Arc::new(SharedFspecService::new(watcher))
}

/// Scenario: EmbeddedFspecBackend::get_workspace_info delegates through the shared service
#[tokio::test]
async fn embedded_backend_get_workspace_info_delegates_through_the_shared_service() {
    // @step Given a SharedFspecService constructed via with_cwd against a temp git repo on branch "main"
    let tmp = init_git_repo_on_branch("main");
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_workspace_info().await is invoked
    let info = backend
        .get_workspace_info()
        .await
        .expect("get_workspace_info");
    // @step Then the awaited result is Ok(WorkspaceInfo { cwd: <tmp_path>, git_branch: Some("main") })
    assert_eq!(
        info.cwd,
        tmp.path().to_string_lossy().to_string(),
        "cwd should equal tmp path"
    );
    assert_eq!(info.git_branch.as_deref(), Some("main"));
}

/// Scenario: WebSocketFspecBackend::get_workspace_info crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_get_workspace_info_crosses_tarpc_cleanly() {
    // @step Given an rpc-server bound to the SAME shared service (cwd is a temp git repo on branch "main")
    let tmp = init_git_repo_on_branch("main");
    let service = service_for(tmp.path());
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.get_workspace_info().await is invoked
    let info = backend
        .get_workspace_info()
        .await
        .expect("get_workspace_info");
    // @step Then the awaited result is Ok(WorkspaceInfo { cwd: <tmp_path>, git_branch: Some("main") })
    assert_eq!(info.cwd, tmp.path().to_string_lossy().to_string());
    assert_eq!(info.git_branch.as_deref(), Some("main"));
}

/// Scenario: Both transports return identical WorkspaceInfo for the same SharedFspecService
#[tokio::test]
async fn both_transports_return_identical_workspace_info() {
    // @step Given a SharedFspecService constructed via with_cwd against a temp git repo on branch "feature/test-branch"
    let tmp = init_git_repo_on_branch("feature/test-branch");
    let service = service_for(tmp.path());
    // @step And an rpc-server bound to that shared service
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service.clone())
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And an EmbeddedFspecBackend wrapping the same shared service
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step And a WebSocketFspecBackend connected to the rpc-server
    let ws: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.get_workspace_info().await is invoked on BOTH backends
    let a = embedded.get_workspace_info().await.expect("embedded");
    let b = ws.get_workspace_info().await.expect("ws");
    // @step Then both awaited results are equal
    assert_eq!(a.cwd, b.cwd);
    assert_eq!(a.git_branch, b.git_branch);
}

/// Scenario: get_workspace_info returns the process cwd with no branch when no cwd was attached
#[tokio::test]
async fn get_workspace_info_returns_process_cwd_with_no_branch_when_no_cwd_attached() {
    // @step Given a SharedFspecService constructed WITHOUT with_cwd (no cwd attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_workspace_info().await is invoked
    let info = backend
        .get_workspace_info()
        .await
        .expect("get_workspace_info");
    // @step Then the awaited result is Ok with git_branch = None
    assert!(
        info.git_branch.is_none(),
        "git_branch should be None, got {:?}",
        info.git_branch
    );
    // @step And the cwd field is non-empty (defaults to std::env::current_dir())
    assert!(!info.cwd.is_empty(), "cwd should default to process cwd");
}

/// Scenario: get_workspace_info returns git_branch = None when cwd is not a git repository
#[tokio::test]
async fn get_workspace_info_returns_no_branch_when_cwd_is_not_a_git_repo() {
    // @step Given a SharedFspecService constructed via with_cwd against a tempdir that is NOT a git repository
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    let service = Arc::new(SharedFspecService::new(watcher).with_cwd(tmp.path().to_path_buf()));
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_workspace_info().await is invoked
    let info = backend
        .get_workspace_info()
        .await
        .expect("get_workspace_info");
    // @step Then the awaited result is Ok with git_branch = None
    assert!(info.git_branch.is_none());
    // @step And the cwd field equals the supplied tempdir path
    assert_eq!(info.cwd, tmp.path().to_string_lossy().to_string());
}

/// Scenario: get_model_info returns safe defaults when no session manager is attached
#[tokio::test]
async fn get_model_info_returns_safe_defaults_when_no_session_manager_attached() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_model_info(SessionId::new("anything")).await is invoked
    let info = backend
        .get_model_info(SessionId::new("anything"))
        .await
        .expect("get_model_info");
    // @step Then the awaited result is Ok(ModelInfo::default()) with display_name = "" and supports_reasoning = false and supports_vision = false and context_window = 0
    assert_eq!(info, ModelInfo::default());
    assert_eq!(info.display_name, "");
    assert!(!info.supports_reasoning);
    assert!(!info.supports_vision);
    assert_eq!(info.context_window, 0);
}

/// Scenario: get_thinking_level returns ThinkingLevel::Off when no session manager is attached
#[tokio::test]
async fn get_thinking_level_returns_off_when_no_session_manager_attached() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_thinking_level(SessionId::new("anything")).await is invoked
    let level = backend
        .get_thinking_level(SessionId::new("anything"))
        .await
        .expect("get_thinking_level");
    // @step Then the awaited result is Ok(ThinkingLevel::Off)
    assert_eq!(level, ThinkingLevel::Off);
}

/// Scenario: get_model_info / get_thinking_level cross tarpc cleanly with safe defaults
#[tokio::test]
async fn get_model_info_and_get_thinking_level_cross_tarpc_cleanly_with_safe_defaults() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.get_model_info(SessionId::new("anything")).await is invoked
    let info = backend
        .get_model_info(SessionId::new("anything"))
        .await
        .expect("get_model_info");
    // @step Then the awaited result is Ok(ModelInfo::default())
    assert_eq!(info, ModelInfo::default());
    // @step When backend.get_thinking_level(SessionId::new("anything")).await is invoked
    let level = backend
        .get_thinking_level(SessionId::new("anything"))
        .await
        .expect("get_thinking_level");
    // @step Then the awaited result is Ok(ThinkingLevel::Off)
    assert_eq!(level, ThinkingLevel::Off);
}

/// Scenario: StubSessionManagerHandle inherits the default SessionManagerHandle implementations
#[tokio::test]
async fn stub_session_manager_handle_inherits_default_implementations() {
    // @step Given a SharedFspecService constructed via with_session_manager(stub_handle, watcher)
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, stub));
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_model_info(SessionId::new("stub-1")).await is invoked
    let info = backend
        .get_model_info(SessionId::new("stub-1"))
        .await
        .expect("get_model_info");
    // @step Then the awaited result is Ok(ModelInfo::default())
    assert_eq!(info, ModelInfo::default());
    // @step When backend.get_thinking_level(SessionId::new("stub-1")).await is invoked
    let level = backend
        .get_thinking_level(SessionId::new("stub-1"))
        .await
        .expect("get_thinking_level");
    // @step Then the awaited result is Ok(ThinkingLevel::Off)
    assert_eq!(level, ThinkingLevel::Off);
    // Tempdir stays alive via the closure capture in the watcher
    drop(tmp);
}

/// Silence the `WorkspaceInfo` unused import — this is intentional;
/// the type is referenced via the backend trait surface and used in
/// equality assertions above.
#[allow(dead_code)]
fn _workspace_info_referenced(_w: WorkspaceInfo) {}
