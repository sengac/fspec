//! RPC-026 — Cross-transport parity for /resume + /search.
//!
//! Feature: spec/features/rpc026-cross-transport-parity.feature
//!
//! Drives the SAME scripted scenario against EmbeddedFspecBackend AND
//! WebSocketFspecBackend and asserts identical observable outcomes.
//! Mirrors the RPC-025 parity pattern (DATA_DIRECTORY mutex + shared
//! seed). The App-level end-to-end behaviour is exercised by the mock
//! backend tests in `app_dispatch_resume_search_rpc026.rs`; this file
//! covers the two backend RPC methods that RPC-026 consumes:
//! `list_sessions` and `persistence_search_history`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // See rpc_persistence_history_rpc025.rs — global DATA_DIRECTORY
    // gate is held for the test body on a current-thread runtime; the
    // await-holding-lock lint is inapplicable.
    clippy::await_holding_lock
)]

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::SessionId;
use tempfile::TempDir;

static DATA_DIR_MUTEX: Mutex<()> = Mutex::new(());

fn setup_parity_temp_dir() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = DATA_DIR_MUTEX.lock().expect("DATA_DIR_MUTEX");
    let temp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(temp.path().to_path_buf())
        .expect("set_data_directory");
    (guard, temp)
}

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    workspace_with_seed(repo_path);
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("watcher"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

/// Scenario: Embedded and WebSocket backends return byte-identical SessionInfo lists for the same SharedFspecService
#[tokio::test]
async fn embedded_and_websocket_list_sessions_byte_identical() {
    // @step Given a SharedFspecService bound to a workspace cwd (no SessionManager attached)
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url).await.expect("connect"),
    );

    // @step When EmbeddedFspecBackend.list_sessions().await is called against the service
    let embedded_list = embedded.list_sessions().await.expect("embedded list");
    // @step And WebSocketFspecBackend.list_sessions().await is called against the same service over a loopback daemon
    let websocket_list = websocket.list_sessions().await.expect("ws list");
    // @step Then both backends return Vec<SessionInfo> of the same length with the same id field in the same order
    let embedded_ids: Vec<&str> = embedded_list.iter().map(|i| i.id.as_str()).collect();
    let websocket_ids: Vec<&str> = websocket_list.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(embedded_ids, websocket_ids);
    // Both empty in this scenario because no SessionManager is bound.
    assert!(embedded_list.is_empty());
}

/// Scenario: Embedded and WebSocket backends return byte-identical HistoryMatch lists for the same query
#[tokio::test]
async fn embedded_and_websocket_search_history_byte_identical() {
    // @step Given a SharedFspecService whose persistence store contains submitted inputs "git status", "git push", "fspec board" under SessionId("s-1")
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url).await.expect("connect"),
    );

    // Seed via the embedded backend (which routes through the lifted
    // core store — both backends share the same on-disk store).
    for text in ["git status", "git push", "fspec board"] {
        embedded
            .persistence_add_history(SessionId::new("s-1"), text.to_string())
            .await
            .expect("seed");
    }

    // @step When EmbeddedFspecBackend.persistence_search_history("git").await is called
    let embedded_out = embedded
        .persistence_search_history("git".to_string())
        .await
        .expect("embedded search");
    // @step And WebSocketFspecBackend.persistence_search_history("git").await is called against the same service over a loopback daemon
    let websocket_out = websocket
        .persistence_search_history("git".to_string())
        .await
        .expect("ws search");

    // @step Then both backends return Vec<HistoryMatch> with the same length and the same text field in the same order
    let embedded_texts: Vec<&str> =
        embedded_out.iter().map(|m| m.text.as_str()).collect();
    let websocket_texts: Vec<&str> =
        websocket_out.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(embedded_texts, websocket_texts);
    // Sanity — we seeded two matches for "git".
    assert_eq!(embedded_out.len(), 2);
}
