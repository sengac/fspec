//! RPC-015 — cross-transport parity for `FspecBackend::checkpoint_counts`.
//!
//! Feature: spec/features/rpc015-cross-transport-parity.feature
//!
//! Mirrors the RPC-009 cross-transport-parity pattern: drives the SAME
//! scripted scenario (build a temp git repo with 1 manual + 1 auto
//! checkpoint ref, then call `backend.checkpoint_counts()`) against
//! BOTH transports and asserts identical results.

use codelet_test_helpers::{OrFail, TestResult};
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
fn seed_repo_with_one_manual_one_auto() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    let repo_path = tmp.path();
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .or_fail("git init")?;
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .or_fail("config email")?;
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path)
        .output()
        .or_fail("config name")?;
    fs::write(repo_path.join("README.md"), "# x\n").or_fail("write README")?;
    fs::create_dir_all(repo_path.join("spec")).or_fail("mkdir spec")?;
    fs::write(
        repo_path.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .or_fail("git add")?;
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo_path)
        .output()
        .or_fail("git commit")?;
    add_ghost_checkpoint(repo_path, "AUTH-001", "baseline")?;
    add_ghost_checkpoint(repo_path, "AUTH-001", "AUTH-001-auto-testing")?;
    Ok(tmp)
}

fn add_ghost_checkpoint(
    repo_path: &Path,
    work_unit_id: &str,
    name: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    let f = repo_path.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&f, format!("{work_unit_id}/{name}")).or_fail("write")?;
    codelet_git::ghost_commit::create_ghost_commit(repo_path, work_unit_id, name)
        .or_fail("create_ghost_commit")?;
    Ok(())
}

/// Build a `SharedFspecService` bound to `repo_path` so `checkpoint_counts`
/// has a cwd to read refs from.
fn service_for(
    repo_path: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).or_fail("WorkUnitsWatcher::new")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()),
    ))
}

/// Scenario: EmbeddedFspecBackend::checkpoint_counts delegates through the shared service
#[tokio::test]
async fn embedded_backend_checkpoint_counts_delegates_through_the_shared_service() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo containing 1 manual + 1 auto checkpoint ref
    let tmp = seed_repo_with_one_manual_one_auto()?;
    let service = service_for(tmp.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.checkpoint_counts().await is invoked
    let counts = backend
        .checkpoint_counts()
        .await
        .or_fail("checkpoint_counts")?;
    // @step Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 1 });
    Ok(())
}

/// Scenario: WebSocketFspecBackend::checkpoint_counts crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_checkpoint_counts_crosses_tarpc_cleanly() -> TestResult {
    // @step Given an rpc-server bound to the SAME shared service (cwd repo with 1 manual + 1 auto ref)
    let tmp = seed_repo_with_one_manual_one_auto()?;
    let service = service_for(tmp.path())?;
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    // @step And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );
    // @step When backend.checkpoint_counts().await is invoked
    let counts = backend
        .checkpoint_counts()
        .await
        .or_fail("checkpoint_counts")?;
    // @step Then the awaited result is Ok(CheckpointCounts { manual: 1, auto: 1 })
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 1 });
    Ok(())
}
