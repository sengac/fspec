//! WT-009 — Cross-transport parity for the new
//! `detach_session_worktree` RPC and the widened
//! `MergeOutcome.worktree_path` field.
//!
//! Feature: spec/features/isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui.feature
//!
//! Drives the new `FspecBackend::detach_session_worktree` against both
//! EmbeddedFspecBackend and WebSocketFspecBackend, constructed against
//! the SAME deterministic StubSessionManagerHandle, and verifies the
//! `MergeOutcome` wire shape round-trips the new `worktree_path` field.
//! Mirrors the RPC-057 cross-transport parity pattern.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{
    MergeOutcome, MergeStatus, SessionId, SessionWorktreeInfo,
};
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_service() -> (
    TempDir,
    Arc<SharedFspecService>,
    Arc<StubSessionManagerHandle>,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    (temp, service, stub)
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> (Arc<dyn FspecBackend>, Arc<dyn FspecBackend>) {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    (embedded, websocket)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket detach_session_worktree both reach the
// stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn detach_session_worktree_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.detach_session_worktree_calls();

    // @step When detach_session_worktree is called via the embedded transport with session_id "s-1"
    embedded
        .detach_session_worktree(SessionId::new("s-1"))
        .await
        .expect("embedded detach_session_worktree");

    // @step And detach_session_worktree is called via the WebSocket transport with session_id "s-1"
    websocket
        .detach_session_worktree(SessionId::new("s-1"))
        .await
        .expect("websocket detach_session_worktree");

    // @step Then the stub's detach_session_worktree_calls counter equals 2
    assert_eq!(
        stub.detach_session_worktree_calls() - initial,
        2,
        "detach_session_worktree_calls should increment by 2"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: merge_session_worktree round-trips the MergeOutcome
// worktree_path field across both transports
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn merge_session_worktree_round_trips_worktree_path_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234"), worktree_path: Some("/tmp/wt/s-1") } behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Success,
        conflicts: Vec::new(),
        merge_commit: Some("abc1234".to_string()),
        worktree_path: Some("/tmp/wt/s-1".to_string()),
    });
    let (embedded, websocket) = dual_backends(service).await;

    // @step When merge_session_worktree is called via the embedded transport with session_id "s-1"
    let em = embedded
        .merge_session_worktree(
            SessionId::new("s-1"),
            codelet_rpc_types::MergeStrategy::FastForward,
        )
        .await
        .expect("embedded merge_session_worktree");

    // @step And merge_session_worktree is called via the WebSocket transport with session_id "s-1"
    let ws = websocket
        .merge_session_worktree(
            SessionId::new("s-1"),
            codelet_rpc_types::MergeStrategy::FastForward,
        )
        .await
        .expect("websocket merge_session_worktree");

    // @step Then both calls return the seeded MergeOutcome including worktree_path Some("/tmp/wt/s-1")
    assert_eq!(em.worktree_path.as_deref(), Some("/tmp/wt/s-1"));
    assert_eq!(ws.worktree_path.as_deref(), Some("/tmp/wt/s-1"));
    assert_eq!(em, ws);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list_session_worktrees round-trips worktree rows (regression
// guard for the widened MergeOutcome wire shape)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_session_worktrees_round_trips_with_widened_merge_outcome() {
    // @step Given a StubSessionManagerHandle seeded with one SessionWorktreeInfo row behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_session_worktrees(vec![SessionWorktreeInfo {
        session_id: SessionId::new("sess-a"),
        worktree_path: "/tmp/wt/a".to_string(),
        base_commit: "aaa1111".to_string(),
        head_commit: "aaa2222".to_string(),
        dirty: true,
    }]);
    let (embedded, websocket) = dual_backends(service).await;

    // @step When list_session_worktrees is called via both transports
    let em = embedded.list_session_worktrees().await.expect("embedded list");
    let ws = websocket.list_session_worktrees().await.expect("websocket list");

    // @step Then both calls return the seeded row
    assert_eq!(em.len(), 1);
    assert_eq!(em[0].worktree_path, "/tmp/wt/a");
    assert_eq!(em, ws);
}
