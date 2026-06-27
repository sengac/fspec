//! RPC-355 — cross-transport parity for `FspecBackend::changed_files` and
//! `FspecBackend::file_diff`.
//!
//! Feature: spec/features/git-changed-files-transport.feature
//!
//! Mirrors the RPC-015 cross-transport-parity pattern: builds a real temp git
//! repo with one modified + one untracked file, then drives the SAME scripted
//! scenario against BOTH transports (embedded + websocket) and asserts
//! identical results. Integration-first: real fs + real gitoxide, no mocking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
fn seed_repo_one_modified_one_untracked() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
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
            .expect("git setup");
    }
    fs::write(repo.join("tracked.txt"), "line one\n").expect("write tracked");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .expect("git commit");

    // Modify the tracked file (unstaged modification → change_type M).
    fs::write(repo.join("tracked.txt"), "line one\nline two\n").expect("modify tracked");
    // Add an untracked file (→ change_type A, staged=false).
    fs::write(repo.join("untracked.txt"), "brand new\n").expect("write untracked");
    tmp
}

/// Initialize a repo with a single committed-then-modified file.
fn seed_repo_one_modified() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
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
            .expect("git setup");
    }
    fs::write(repo.join("tracked.txt"), "alpha\n").expect("write tracked");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .expect("git commit");
    fs::write(repo.join("tracked.txt"), "alpha\nbeta\n").expect("modify tracked");
    tmp
}

/// Initialize a repo with a single committed binary file, then MODIFY it with
/// differing bytes that include a NUL byte so gitoxide classifies it binary.
fn seed_repo_one_modified_binary() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
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
            .expect("git setup");
    }
    // Commit an initial binary blob (contains a NUL byte).
    fs::write(repo.join("blob.bin"), vec![0u8, 159, 146, 150, 1, 2, 3]).expect("write binary");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .expect("git commit");
    // Modify with a different binary byte sequence (still contains a NUL byte).
    fs::write(repo.join("blob.bin"), vec![0u8, 200, 201, 202, 9, 8, 7, 6]).expect("modify binary");
    tmp
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

/// Service with NO cwd attached (constructed without `with_cwd`).
fn service_no_cwd(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher))
}

/// Scenario: Embedded backend changed_files returns modified and untracked entries
#[tokio::test]
async fn embedded_backend_changed_files_returns_modified_and_untracked_entries() {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with one modified and one untracked file
    let tmp = seed_repo_one_modified_one_untracked();
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.changed_files().await is invoked
    let files = backend.changed_files().await.expect("changed_files");
    // @step Then the result contains an entry for the modified file and an entry for the untracked file
    let modified = files
        .iter()
        .find(|c| c.path == "tracked.txt")
        .expect("modified entry");
    assert_eq!(modified.change_type, "M");
    let untracked = files
        .iter()
        .find(|c| c.path == "untracked.txt")
        .expect("untracked entry");
    assert_eq!(untracked.change_type, "A");
    assert!(!untracked.staged, "untracked entry must be staged=false");
}

/// Scenario: file_diff returns the binary-file sentinel for a binary file
#[tokio::test]
async fn file_diff_returns_the_binary_file_sentinel_for_a_binary_file() {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with a committed-then-modified binary file
    let tmp = seed_repo_one_modified_binary();
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.file_diff(path).await is invoked for the binary file
    let diff = backend
        .file_diff("blob.bin".to_string())
        .await
        .expect("file_diff");
    // @step Then the result is Some text equal to the binary-file sentinel
    let text = diff.expect("Some diff");
    assert_eq!(text, "[Binary file - no diff available]");
}

/// Scenario: Embedded backend file_diff returns the unified diff for a modified file
#[tokio::test]
async fn embedded_backend_file_diff_returns_the_unified_diff_for_a_modified_file() {
    // @step Given a SharedFspecService constructed via with_cwd against a git repo with a modified file
    let tmp = seed_repo_one_modified();
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.file_diff(path).await is invoked for the modified file
    let diff = backend
        .file_diff("tracked.txt".to_string())
        .await
        .expect("file_diff");
    // @step Then the result is Some diff text containing the changed lines
    let text = diff.expect("Some diff");
    assert!(text.contains("beta"), "diff must contain the added line");
}

/// Scenario: changed_files returns an empty Vec when no cwd is attached
#[tokio::test]
async fn changed_files_returns_an_empty_vec_when_no_cwd_is_attached() {
    // @step Given a SharedFspecService constructed without with_cwd
    let tmp = seed_repo_one_modified_one_untracked();
    let service = service_no_cwd(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.changed_files().await is invoked
    let files = backend.changed_files().await.expect("changed_files");
    // @step Then the awaited result is an empty Vec
    assert!(files.is_empty(), "no cwd must yield an empty Vec");
}

/// Scenario: WebSocket backend changed_files crosses tarpc and matches the embedded backend
#[tokio::test]
async fn websocket_backend_changed_files_crosses_tarpc_and_matches_the_embedded_backend() {
    // @step Given an rpc-server bound to a SharedFspecService with a cwd repo containing one modified and one untracked file
    let tmp = seed_repo_one_modified_one_untracked();
    let service = service_for(tmp.path());
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.changed_files().await is invoked
    let files = backend.changed_files().await.expect("changed_files");
    // @step Then the result contains an entry for the modified file and an entry for the untracked file
    let modified = files
        .iter()
        .find(|c| c.path == "tracked.txt")
        .expect("modified entry");
    assert_eq!(modified.change_type, "M");
    let untracked = files
        .iter()
        .find(|c| c.path == "untracked.txt")
        .expect("untracked entry");
    assert_eq!(untracked.change_type, "A");
    assert!(!untracked.staged, "untracked entry must be staged=false");
}
