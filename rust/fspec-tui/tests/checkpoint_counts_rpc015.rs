//! RPC-015 — cross-transport parity for `FspecBackend::checkpoint_counts`.
//!
//! Feature: spec/features/rpc015-cross-transport-parity.feature
//!
//! Mirrors the RPC-009 cross-transport-parity pattern: drives the SAME
//! scripted scenario (build a temp git repo with 1 manual + 1 auto
//! checkpoint ref, then call `backend.checkpoint_counts()`) against
//! BOTH transports and asserts identical results.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::CheckpointCounts;
use tempfile::TempDir;

/// Helper: initialize a fresh git repo with a seed commit + spec/work-units.json
/// then drop in 1 manual + 1 auto ghost-checkpoint ref.
fn seed_repo_with_one_manual_one_auto() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let repo_path = tmp.path();
    Command::new("git")
        .args(["init"])
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
    add_ghost_checkpoint(repo_path, "AUTH-001", "baseline");
    add_ghost_checkpoint(repo_path, "AUTH-001", "AUTH-001-auto-testing");
    tmp
}

fn add_ghost_checkpoint(repo_path: &Path, work_unit_id: &str, name: &str) {
    let f = repo_path.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&f, format!("{work_unit_id}/{name}")).expect("write");
    codelet_git::ghost_commit::create_ghost_commit(repo_path, work_unit_id, name)
        .expect("create_ghost_commit");
}

/// Build a `SharedFspecService` bound to `repo_path` so `checkpoint_counts`
/// has a cwd to read refs from.
fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

/// Scenario: EmbeddedFspecBackend::checkpoint_counts delegates through the shared service
#[tokio::test]
async fn embedded_backend_checkpoint_counts_delegates_through_the_shared_service() {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo containing 1 manual + 1 auto checkpoint ref
    let tmp = seed_repo_with_one_manual_one_auto();
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.checkpoint_counts().await is invoked
    let counts = backend
        .checkpoint_counts()
        .await
        .expect("checkpoint_counts");
    // @step Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 1 });
}

/// Scenario: WebSocketFspecBackend::checkpoint_counts crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_checkpoint_counts_crosses_tarpc_cleanly() {
    // @step Given an rpc-server bound to the SAME shared service (cwd repo with 1 manual + 1 auto ref)
    let tmp = seed_repo_with_one_manual_one_auto();
    let service = service_for(tmp.path());
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.checkpoint_counts().await is invoked
    let counts = backend
        .checkpoint_counts()
        .await
        .expect("checkpoint_counts");
    // @step Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 1 });
}

/// Scenario: napi::count_checkpoints is wired through the same git helper
#[test]
fn napi_count_checkpoints_is_wired_through_the_same_git_helper() {
    // @step Given rust/napi/src/git.rs after RPC-015 lands
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("napi")
        .join("src")
        .join("git.rs");
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "pub fn count_checkpoints"
    assert!(
        body.contains("pub fn count_checkpoints"),
        "rust/napi/src/git.rs must export `pub fn count_checkpoints`"
    );
    // @step And the function body contains the substring "codelet_git::ghost_commit::count_checkpoints"
    assert!(
        body.contains("codelet_git::ghost_commit::count_checkpoints"),
        "napi count_checkpoints must delegate to codelet_git::ghost_commit::count_checkpoints"
    );
}
