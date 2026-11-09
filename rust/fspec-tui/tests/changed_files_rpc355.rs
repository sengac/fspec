//! RPC-355 — cross-transport parity for `FspecBackend::changed_files` and
//! `FspecBackend::file_diff`.
//!
//! Feature: spec/features/git-changed-files-transport.feature
//!
//! Mirrors the RPC-015 cross-transport-parity pattern: builds a real temp git
//! repo with one modified + one untracked file, then drives the SAME scripted
//! scenario against BOTH transports (embedded + websocket) and asserts
//! identical results. Integration-first: real fs + real gitoxide, no mocking.

use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use tempfile::TempDir;

/// Initialize a fresh git repo with a committed README, then leave the working
/// tree with one MODIFIED tracked file and one UNTRACKED new file.
fn seed_repo_one_modified_one_untracked() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    let repo = tmp.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .or_fail("git setup")?;
    }
    fs::write(repo.join("tracked.txt"), "line one\n").or_fail("write tracked")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .or_fail("git add")?;
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .or_fail("git commit")?;

    // Modify the tracked file (unstaged modification → change_type M).
    fs::write(repo.join("tracked.txt"), "line one\nline two\n").or_fail("modify tracked")?;
    // Add an untracked file (→ change_type A, staged=false).
    fs::write(repo.join("untracked.txt"), "brand new\n").or_fail("write untracked")?;
    Ok(tmp)
}

/// Initialize a repo with a single committed-then-modified file.
fn seed_repo_one_modified() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    let repo = tmp.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .or_fail("git setup")?;
    }
    fs::write(repo.join("tracked.txt"), "alpha\n").or_fail("write tracked")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .or_fail("git add")?;
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .or_fail("git commit")?;
    fs::write(repo.join("tracked.txt"), "alpha\nbeta\n").or_fail("modify tracked")?;
    Ok(tmp)
}

/// Initialize a repo with a single committed binary file, then MODIFY it with
/// differing bytes that include a NUL byte so gitoxide classifies it binary.
fn seed_repo_one_modified_binary() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    let repo = tmp.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(repo)
            .output()
            .or_fail("git setup")?;
    }
    // Commit an initial binary blob (contains a NUL byte).
    fs::write(repo.join("blob.bin"), vec![0u8, 159, 146, 150, 1, 2, 3]).or_fail("write binary")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .or_fail("git add")?;
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .or_fail("git commit")?;
    // Modify with a different binary byte sequence (still contains a NUL byte).
    fs::write(repo.join("blob.bin"), vec![0u8, 200, 201, 202, 9, 8, 7, 6])
        .or_fail("modify binary")?;
    Ok(tmp)
}

fn service_for(
    repo_path: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).or_fail("WorkUnitsWatcher::new")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()),
    ))
}

/// Service with NO cwd attached (constructed without `with_cwd`).
fn service_no_cwd(
    repo_path: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).or_fail("WorkUnitsWatcher::new")?);
    Ok(Arc::new(SharedFspecService::new(watcher)))
}

/// Scenario: Embedded backend changed_files returns modified and untracked entries
#[tokio::test]
async fn embedded_backend_changed_files_returns_modified_and_untracked_entries() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with one modified and one untracked file
    let tmp = seed_repo_one_modified_one_untracked()?;
    let service = service_for(tmp.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.changed_files().await is invoked
    let files = backend.changed_files().await.or_fail("changed_files")?;
    // @step Then the result contains an entry for the modified file and an entry for the untracked file
    let modified = files
        .iter()
        .find(|c| c.path == "tracked.txt")
        .or_fail("modified entry")?;
    assert_eq!(modified.change_type, "M");
    let untracked = files
        .iter()
        .find(|c| c.path == "untracked.txt")
        .or_fail("untracked entry")?;
    assert_eq!(untracked.change_type, "A");
    assert!(!untracked.staged, "untracked entry must be staged=false");
    Ok(())
}

/// Scenario: file_diff returns the binary-file sentinel for a binary file
#[tokio::test]
async fn file_diff_returns_the_binary_file_sentinel_for_a_binary_file() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with a committed-then-modified binary file
    let tmp = seed_repo_one_modified_binary()?;
    let service = service_for(tmp.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.file_diff(path).await is invoked for the binary file
    let diff = backend
        .file_diff("blob.bin".to_string())
        .await
        .or_fail("file_diff")?;
    // @step Then the result is Some text equal to the binary-file sentinel
    let text = diff.or_fail("Some diff")?;
    assert_eq!(text, "[Binary file - no diff available]");
    Ok(())
}

/// Scenario: Embedded backend file_diff returns the unified diff for a modified file
#[tokio::test]
async fn embedded_backend_file_diff_returns_the_unified_diff_for_a_modified_file() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with a modified file
    let tmp = seed_repo_one_modified()?;
    let service = service_for(tmp.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.file_diff(path).await is invoked for the modified file
    let diff = backend
        .file_diff("tracked.txt".to_string())
        .await
        .or_fail("file_diff")?;
    // @step Then the result is Some diff text containing the changed lines
    let text = diff.or_fail("Some diff")?;
    assert!(text.contains("beta"), "diff must contain the added line");
    Ok(())
}

/// Scenario: changed_files is refused when no cwd is attached (BUG-465)
#[tokio::test]
async fn changed_files_is_refused_when_no_cwd_is_attached() -> TestResult {
    // @step Given a SharedFspecService constructed without with_cwd
    let tmp = seed_repo_one_modified_one_untracked()?;
    let service = service_no_cwd(tmp.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.changed_files().await is invoked
    let listed = backend.changed_files().await;
    // @step Then the awaited result is an error saying no workspace directory is attached
    let error = listed.or_fail_err("no cwd is refused")?.to_string();
    assert!(
        error.contains("no workspace directory is attached"),
        "{error}"
    );
    Ok(())
}

/// Scenario: WebSocket backend changed_files crosses tarpc and matches the embedded backend
#[tokio::test]
async fn websocket_backend_changed_files_crosses_tarpc_and_matches_the_embedded_backend(
) -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with a cwd repo containing one modified and one untracked file
    let tmp = seed_repo_one_modified_one_untracked()?;
    let service = service_for(tmp.path())?;
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );
    // @step When backend.changed_files().await is invoked
    let files = backend.changed_files().await.or_fail("changed_files")?;
    // @step Then the result contains an entry for the modified file and an entry for the untracked file
    let modified = files
        .iter()
        .find(|c| c.path == "tracked.txt")
        .or_fail("modified entry")?;
    assert_eq!(modified.change_type, "M");
    let untracked = files
        .iter()
        .find(|c| c.path == "untracked.txt")
        .or_fail("untracked entry")?;
    assert_eq!(untracked.change_type, "A");
    assert!(!untracked.staged, "untracked entry must be staged=false");
    Ok(())
}
