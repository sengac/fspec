//! RPC-057 — Cross-transport parity for the /merge-worktree RPC surface.
//!
//! Feature: spec/features/rpc057-merge-worktree-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-054 / RPC-055 / RPC-056
//! cross-transport parity patterns.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock,
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
    MergeOutcome, MergeStatus, MergeStrategy, SessionChangesSummary, SessionId, SessionWorktreeInfo,
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
// Scenario: Embedded and WebSocket merge_session_worktree both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn merge_session_worktree_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    stub.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Success,
        conflicts: vec![],
        merge_commit: Some("abc1234".to_string()),
    });
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.merge_session_worktree_calls();

    // @step When merge_session_worktree is called via the embedded transport with session_id "s-1" and MergeStrategy::FastForward
    let em = embedded
        .merge_session_worktree(SessionId::new("s-1"), MergeStrategy::FastForward)
        .await
        .expect("embedded merge_session_worktree");

    // @step And merge_session_worktree is called via the WebSocket transport with session_id "s-1" and MergeStrategy::FastForward
    let ws = websocket
        .merge_session_worktree(SessionId::new("s-1"), MergeStrategy::FastForward)
        .await
        .expect("websocket merge_session_worktree");

    // @step Then the stub's merge_session_worktree_calls counter equals 2
    assert_eq!(
        stub.merge_session_worktree_calls() - initial,
        2,
        "merge_session_worktree_calls should increment by 2"
    );

    // @step And both calls return MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") }
    assert_eq!(em.status, MergeStatus::Success);
    assert_eq!(ws.status, MergeStatus::Success);
    assert_eq!(em.conflicts, Vec::<String>::new());
    assert_eq!(ws.conflicts, Vec::<String>::new());
    assert_eq!(em.merge_commit.as_deref(), Some("abc1234"));
    assert_eq!(ws.merge_commit.as_deref(), Some("abc1234"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket discard_session_worktree both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn discard_session_worktree_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded to return Ok(()) for discard_session_worktree behind both transports
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.discard_session_worktree_calls();

    // @step When discard_session_worktree is called via the embedded transport with session_id "s-1"
    embedded
        .discard_session_worktree(SessionId::new("s-1"))
        .await
        .expect("embedded discard_session_worktree");

    // @step And discard_session_worktree is called via the WebSocket transport with session_id "s-1"
    websocket
        .discard_session_worktree(SessionId::new("s-1"))
        .await
        .expect("websocket discard_session_worktree");

    // @step Then the stub's discard_session_worktree_calls counter equals 2
    assert_eq!(
        stub.discard_session_worktree_calls() - initial,
        2,
        "discard_session_worktree_calls should increment by 2"
    );

    // @step And both calls return Ok(())
    // (already asserted via expect("..."))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket prune_orphaned_worktrees both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prune_orphaned_worktrees_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with pruned session ids ["sess-a", "sess-b"] behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_pruned_sessions(vec!["sess-a".to_string(), "sess-b".to_string()]);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.prune_orphaned_worktrees_calls();

    // @step When prune_orphaned_worktrees is called via the embedded transport
    let em = embedded
        .prune_orphaned_worktrees()
        .await
        .expect("embedded prune_orphaned_worktrees");

    // @step And prune_orphaned_worktrees is called via the WebSocket transport
    let ws = websocket
        .prune_orphaned_worktrees()
        .await
        .expect("websocket prune_orphaned_worktrees");

    // @step Then the stub's prune_orphaned_worktrees_calls counter equals 2
    assert_eq!(
        stub.prune_orphaned_worktrees_calls() - initial,
        2,
        "prune_orphaned_worktrees_calls should increment by 2"
    );

    // @step And both calls return ["sess-a", "sess-b"]
    assert_eq!(em, vec!["sess-a".to_string(), "sess-b".to_string()]);
    assert_eq!(ws, vec!["sess-a".to_string(), "sess-b".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket list_session_worktrees both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_session_worktrees_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with two SessionWorktreeInfo rows behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_session_worktrees(vec![
        SessionWorktreeInfo {
            session_id: SessionId::new("sess-a"),
            worktree_path: "/tmp/wt/a".to_string(),
            base_commit: "aaa1111".to_string(),
            head_commit: "aaa2222".to_string(),
            dirty: false,
        },
        SessionWorktreeInfo {
            session_id: SessionId::new("sess-b"),
            worktree_path: "/tmp/wt/b".to_string(),
            base_commit: "bbb1111".to_string(),
            head_commit: "bbb2222".to_string(),
            dirty: true,
        },
    ]);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.list_session_worktrees_calls();

    // @step When list_session_worktrees is called via the embedded transport
    let em = embedded
        .list_session_worktrees()
        .await
        .expect("embedded list_session_worktrees");

    // @step And list_session_worktrees is called via the WebSocket transport
    let ws = websocket
        .list_session_worktrees()
        .await
        .expect("websocket list_session_worktrees");

    // @step Then the stub's list_session_worktrees_calls counter equals 2
    assert_eq!(
        stub.list_session_worktrees_calls() - initial,
        2,
        "list_session_worktrees_calls should increment by 2"
    );

    // @step And both calls return a Vec of length 2
    assert_eq!(em.len(), 2);
    assert_eq!(ws.len(), 2);

    // @step And each entry has identical session_id, worktree_path, base_commit, head_commit, dirty fields across the two transports
    for (e, w) in em.iter().zip(ws.iter()) {
        assert_eq!(e.session_id, w.session_id);
        assert_eq!(e.worktree_path, w.worktree_path);
        assert_eq!(e.base_commit, w.base_commit);
        assert_eq!(e.head_commit, w.head_commit);
        assert_eq!(e.dirty, w.dirty);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket inspect_session_changes both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inspect_session_changes_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with SessionChangesSummary { files_changed: 3, insertions: 12, deletions: 5, commits: ["abc1234"] } behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 3,
        insertions: 12,
        deletions: 5,
        commits: vec!["abc1234".to_string()],
    });
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.inspect_session_changes_calls();

    // @step When inspect_session_changes is called via the embedded transport with session_id "s-1"
    let em = embedded
        .inspect_session_changes(SessionId::new("s-1"))
        .await
        .expect("embedded inspect_session_changes");

    // @step And inspect_session_changes is called via the WebSocket transport with session_id "s-1"
    let ws = websocket
        .inspect_session_changes(SessionId::new("s-1"))
        .await
        .expect("websocket inspect_session_changes");

    // @step Then the stub's inspect_session_changes_calls counter equals 2
    assert_eq!(
        stub.inspect_session_changes_calls() - initial,
        2,
        "inspect_session_changes_calls should increment by 2"
    );

    // @step And both calls return SessionChangesSummary { files_changed: 3, insertions: 12, deletions: 5, commits: ["abc1234"] }
    assert_eq!(em.files_changed, 3);
    assert_eq!(em.insertions, 12);
    assert_eq!(em.deletions, 5);
    assert_eq!(em.commits, vec!["abc1234".to_string()]);
    assert_eq!(ws.files_changed, 3);
    assert_eq!(ws.insertions, 12);
    assert_eq!(ws.deletions, 5);
    assert_eq!(ws.commits, vec!["abc1234".to_string()]);
}
