//! RPC-020 — Cross-transport parity for `FspecBackend::search_files`.
//!
//! Feature: spec/features/rpc020-cross-transport-parity.feature
//!
//! Mirrors the RPC-015 / RPC-018 cross-transport-parity pattern: drives
//! the SAME scripted scenario against BOTH transports and asserts
//! identical results.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use tempfile::TempDir;

fn workspace_with_files(files: &[&str]) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    for f in files {
        let p = tmp.path().join(f);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&p, b"x").expect("write");
    }
    // The watcher requires spec/work-units.json.
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    tmp
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

fn service_without_cwd() -> Arc<SharedFspecService> {
    let tmp = workspace_with_files(&[]);
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("watcher"));
    Box::leak(Box::new(tmp));
    Arc::new(SharedFspecService::new(watcher))
}

/// Scenario: EmbeddedFspecBackend::search_files delegates through the shared service
#[tokio::test]
async fn embedded_backend_search_files_delegates_through_the_shared_service() {
    // @step Given a SharedFspecService constructed via with_cwd against a tempdir containing files ["README.md", "src/main.rs"]
    let tmp = workspace_with_files(&["README.md", "src/main.rs"]);
    let service = service_for(tmp.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.search_files("README".to_string(), 10).await is invoked
    let out = backend
        .search_files("README".to_string(), 10)
        .await
        .expect("search_files");
    // @step Then the awaited result is Ok with at least one entry containing "README.md"
    assert!(
        out.iter().any(|p| p.contains("README.md")),
        "expected README.md in {out:?}"
    );
}

/// Scenario: WebSocketFspecBackend::search_files crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_search_files_crosses_tarpc_cleanly() {
    // @step Given an rpc-server bound to a SharedFspecService whose cwd contains files ["README.md", "src/main.rs"]
    let tmp = workspace_with_files(&["README.md", "src/main.rs"]);
    let service = service_for(tmp.path());
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.search_files("README".to_string(), 10).await is invoked
    let out = backend
        .search_files("README".to_string(), 10)
        .await
        .expect("search_files");
    // @step Then the awaited result is Ok with at least one entry containing "README.md"
    assert!(
        out.iter().any(|p| p.contains("README.md")),
        "expected README.md in {out:?}"
    );
}

/// Scenario: Both transports return identical Vec<String> for the same SharedFspecService
#[tokio::test]
async fn both_transports_return_identical_results_for_same_shared_service() {
    // @step Given a SharedFspecService constructed via with_cwd against a tempdir containing 5 files matching the prefix "src"
    let tmp = workspace_with_files(&[
        "src/main.rs",
        "src/lib.rs",
        "src/foo.rs",
        "src/bar.rs",
        "src/baz.rs",
    ]);
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
    // @step When backend.search_files("src".to_string(), 10).await is invoked on BOTH backends
    let a = embedded
        .search_files("src".to_string(), 10)
        .await
        .expect("embedded");
    let b = ws.search_files("src".to_string(), 10).await.expect("ws");
    // @step Then both awaited results are equal
    assert_eq!(a, b, "transports must agree on search_files results");
    assert!(!a.is_empty(), "expected at least one src match");
}

/// Scenario: search_files returns an empty Vec when no cwd is attached
#[tokio::test]
async fn search_files_returns_empty_vec_when_no_cwd_attached() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no cwd attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.search_files("anything".to_string(), 10).await is invoked
    let out = backend
        .search_files("anything".to_string(), 10)
        .await
        .expect("search_files");
    // @step Then the awaited result is Ok with an empty Vec
    assert!(out.is_empty(), "expected empty Vec, got {out:?}");
}

/// Scenario: search_files honours the limit argument across transports
#[tokio::test]
async fn search_files_honours_limit_across_transports() {
    // @step Given a SharedFspecService constructed via with_cwd against a tempdir containing 25 files matching the prefix "doc"
    let files: Vec<String> = (0..25).map(|i| format!("doc_{i}.md")).collect();
    let borrowed: Vec<&str> = files.iter().map(String::as_str).collect();
    let tmp = workspace_with_files(&borrowed);
    let service = service_for(tmp.path());
    // @step And an rpc-server bound to that shared service
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.search_files("doc".to_string(), 5).await is invoked
    let out = backend
        .search_files("doc".to_string(), 5)
        .await
        .expect("search_files");
    // @step Then the awaited result is Ok with exactly 5 entries
    assert_eq!(out.len(), 5, "limit not honoured, got {out:?}");
}
