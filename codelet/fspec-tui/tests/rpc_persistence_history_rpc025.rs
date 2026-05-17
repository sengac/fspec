//! RPC-025 — Cross-transport parity for the three new `persistence_*_history`
//! RPC methods.
//!
//! Feature: spec/features/rpc025-rpc-methods.feature
//!
//! Mirrors the RPC-020 search_files parity pattern: drives the SAME
//! scripted scenario against BOTH transports (Embedded + WebSocket) and
//! asserts identical results. All tests share `codelet_common`'s global
//! data directory via `set_data_directory`, so they are serialized.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // Tests use std::sync::Mutex<()> as a global gate to serialize
    // access to codelet_common::DATA_DIRECTORY. The guard is held for
    // the entire test body (intentionally), and #[tokio::test]
    // defaults to a current-thread runtime so the guard never crosses
    // a thread boundary. clippy's await-holding-lock lint is correct
    // in general but inapplicable here.
    clippy::await_holding_lock
)]

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use codelet_core::persistence::history as core_history;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::SessionId;
use tempfile::TempDir;

/// Tests share the process-wide DATA_DIRECTORY via codelet_common — serialize.
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

fn service_without_cwd(tmp: &TempDir) -> Arc<SharedFspecService> {
    workspace_with_seed(tmp.path());
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("watcher"));
    Arc::new(SharedFspecService::new(watcher))
}

/// Scenario: FspecService trait declares the three new history RPC methods
#[test]
fn fspec_service_trait_declares_three_new_history_rpc_methods() {
    // @step Given the codelet/rpc crate
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .join("rpc")
        .join("src")
        .join("lib.rs");
    let body = std::fs::read_to_string(&path).expect("read codelet/rpc/src/lib.rs");
    // @step Then the FspecService trait declares "async fn persistence_add_history(session: SessionId, text: String) -> Result<(), String>"
    assert!(
        body.contains("async fn persistence_add_history(session: SessionId, text: String)")
            || body.contains("async fn persistence_add_history(\n        session: SessionId"),
        "FspecService must declare persistence_add_history(session: SessionId, text: String)"
    );
    // @step And the FspecService trait declares "async fn persistence_get_history(session: SessionId, limit: u32) -> Result<Vec<String>, String>"
    assert!(
        body.contains("async fn persistence_get_history(session: SessionId, limit: u32)")
            || body.contains("async fn persistence_get_history(\n        session: SessionId"),
        "FspecService must declare persistence_get_history(session: SessionId, limit: u32)"
    );
    // @step And the FspecService trait declares "async fn persistence_search_history(query: String) -> Result<Vec<HistoryMatch>, String>"
    assert!(
        body.contains("async fn persistence_search_history(query: String)")
            || body.contains("async fn persistence_search_history(\n        query: String"),
        "FspecService must declare persistence_search_history(query: String) -> Vec<HistoryMatch>"
    );
}

/// Scenario: FspecBackend trait declares the three new history methods with matching signatures
#[test]
fn fspec_backend_trait_declares_three_new_history_methods() {
    // @step Given the codelet/fspec-tui crate
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("transport")
        .join("mod.rs");
    let body = std::fs::read_to_string(&path).expect("read transport/mod.rs");
    // @step Then the FspecBackend trait in transport/mod.rs declares "async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>"
    assert!(
        body.contains("async fn persistence_add_history(&self, session: SessionId, text: String)"),
        "FspecBackend must declare persistence_add_history(session, text) -> Result<()>"
    );
    // @step And the FspecBackend trait declares "async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>"
    assert!(
        body.contains("async fn persistence_get_history(&self, session: SessionId, limit: u32)"),
        "FspecBackend must declare persistence_get_history(session, limit) -> Result<Vec<String>>"
    );
    // @step And the FspecBackend trait declares "async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>"
    assert!(
        body.contains("async fn persistence_search_history(&self, query: String)"),
        "FspecBackend must declare persistence_search_history(query) -> Result<Vec<HistoryMatch>>"
    );
}

/// Scenario: EmbeddedFspecBackend.persistence_add_history persists into the lifted core store via FspecServiceImpl
#[tokio::test]
async fn embedded_backend_persistence_add_history_persists_via_core_store() {
    // @step Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When EmbeddedFspecBackend.persistence_add_history(SessionId("s-1"), "embedded hello") is awaited
    backend
        .persistence_add_history(SessionId::new("s-1"), "embedded hello".to_string())
        .await
        .expect("persistence_add_history");
    // @step Then codelet_core::persistence::history::get(Some("/tmp/parity"), Some(1))[0].display equals "embedded hello"
    let entries = core_history::get(Some(&cwd), Some(1)).expect("core get");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display, "embedded hello");
}

/// Scenario: EmbeddedFspecBackend.persistence_get_history returns the texts of the most recent entries newest-first
#[tokio::test]
async fn embedded_backend_persistence_get_history_returns_newest_first_strings() {
    // @step Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step And persistence_add_history is called for SessionId("s-1") with texts "a", "b", "c" in order
    for text in ["a", "b", "c"] {
        backend
            .persistence_add_history(SessionId::new("s-1"), text.to_string())
            .await
            .expect("persistence_add_history");
    }
    // @step When EmbeddedFspecBackend.persistence_get_history(SessionId("s-1"), 10) is awaited
    let out = backend
        .persistence_get_history(SessionId::new("s-1"), 10)
        .await
        .expect("persistence_get_history");
    // @step Then the returned Vec<String> equals ["c", "b", "a"]
    assert_eq!(out, vec!["c".to_string(), "b".to_string(), "a".to_string()]);
}

/// Scenario: EmbeddedFspecBackend.persistence_search_history returns HistoryMatch values with text, session_id, and ISO timestamp
#[tokio::test]
async fn embedded_backend_persistence_search_history_returns_history_match_values() {
    // @step Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step And persistence_add_history is called for SessionId("s-1") with texts "foobar", "baz", "FOOZ"
    for text in ["foobar", "baz", "FOOZ"] {
        backend
            .persistence_add_history(SessionId::new("s-1"), text.to_string())
            .await
            .expect("persistence_add_history");
    }
    // @step When EmbeddedFspecBackend.persistence_search_history("foo") is awaited
    let out = backend
        .persistence_search_history("foo".to_string())
        .await
        .expect("persistence_search_history");
    // @step Then the returned Vec<HistoryMatch> has length 2
    assert_eq!(out.len(), 2, "expected 2 matches for 'foo' in {out:?}");
    // @step And each HistoryMatch.session_id equals SessionId("s-1")
    for m in &out {
        assert_eq!(m.session_id, SessionId::new("s-1"));
    }
    // @step And the HistoryMatch.text values are exactly ["FOOZ", "foobar"] in newest-first order
    let texts: Vec<&str> = out.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["FOOZ", "foobar"]);
    // @step And each HistoryMatch.timestamp_iso is a valid RFC3339 string
    for m in &out {
        assert!(
            chrono::DateTime::parse_from_rfc3339(&m.timestamp_iso).is_ok(),
            "timestamp_iso must round-trip RFC3339: {}",
            m.timestamp_iso
        );
    }
}

/// Scenario: WebSocketFspecBackend round-trips persistence_add_history over tarpc to the same core store
#[tokio::test]
async fn websocket_backend_persistence_add_history_round_trips_to_core_store() {
    // @step Given a WebSocketFspecBackend connected to a local FspecService bound to cwd "/tmp/parity"
    let (_guard, temp) = setup_parity_temp_dir();
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd);
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .expect("connect"),
    );
    // @step When WebSocketFspecBackend.persistence_add_history(SessionId("s-2"), "ws hello") is awaited
    backend
        .persistence_add_history(SessionId::new("s-2"), "ws hello".to_string())
        .await
        .expect("ws persistence_add_history");
    // @step Then codelet_core::persistence::history::get(Some("/tmp/parity"), Some(1))[0].display equals "ws hello"
    let entries = core_history::get(Some(&cwd), Some(1)).expect("core get");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display, "ws hello");
}

/// Scenario: Cross-transport-parity for persistence_get_history
#[tokio::test]
async fn cross_transport_parity_for_persistence_get_history() {
    // @step Given an EmbeddedFspecBackend and a WebSocketFspecBackend both bound to the same workspace cwd "/tmp/parity"
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
    let ws: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .expect("connect"),
    );

    // @step And persistence_add_history is called via the EmbeddedFspecBackend for SessionId("s-1") with text "shared"
    embedded
        .persistence_add_history(SessionId::new("s-1"), "shared".to_string())
        .await
        .expect("embedded add");

    // @step When persistence_get_history(SessionId("s-1"), 10) is awaited on BOTH backends
    let emb_out = embedded
        .persistence_get_history(SessionId::new("s-1"), 10)
        .await
        .expect("embedded get");
    let ws_out = ws
        .persistence_get_history(SessionId::new("s-1"), 10)
        .await
        .expect("ws get");

    // @step Then both return Vec<String> == ["shared"]
    assert_eq!(emb_out, vec!["shared".to_string()]);
    assert_eq!(ws_out, vec!["shared".to_string()]);
}

/// Scenario: Cross-transport-parity for persistence_search_history
#[tokio::test]
async fn cross_transport_parity_for_persistence_search_history() {
    // @step Given an EmbeddedFspecBackend and a WebSocketFspecBackend both bound to the same workspace cwd "/tmp/parity"
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
    let ws: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .expect("connect"),
    );

    // @step And persistence_add_history is called via the WebSocketFspecBackend for SessionId("s-1") with texts "alpha", "beta"
    ws.persistence_add_history(SessionId::new("s-1"), "alpha".to_string())
        .await
        .expect("ws add alpha");
    ws.persistence_add_history(SessionId::new("s-1"), "beta".to_string())
        .await
        .expect("ws add beta");

    // @step When persistence_search_history("alp") is awaited on BOTH backends
    let emb_out = embedded
        .persistence_search_history("alp".to_string())
        .await
        .expect("embedded search");
    let ws_out = ws
        .persistence_search_history("alp".to_string())
        .await
        .expect("ws search");

    // @step Then both return Vec<HistoryMatch> whose text values equal ["alpha"]
    let emb_texts: Vec<&str> = emb_out.iter().map(|m| m.text.as_str()).collect();
    let ws_texts: Vec<&str> = ws_out.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(emb_texts, vec!["alpha"]);
    assert_eq!(ws_texts, vec!["alpha"]);
    // @step And the timestamp_iso values are equal between the two backends for the same entry
    assert_eq!(emb_out.len(), 1);
    assert_eq!(ws_out.len(), 1);
    assert_eq!(emb_out[0].timestamp_iso, ws_out[0].timestamp_iso);
}

/// Scenario: FspecServiceImpl falls back to a None project filter when the shared service has no workspace cwd attached
#[tokio::test]
async fn fspec_service_impl_falls_back_to_none_project_when_no_cwd() {
    // @step Given a FspecServiceImpl with workspace_cwd == None
    let (_guard, temp) = setup_parity_temp_dir();
    let service = service_without_cwd(&temp);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step And persistence_add_history is called for SessionId("s-1") with text "global hello"
    backend
        .persistence_add_history(SessionId::new("s-1"), "global hello".to_string())
        .await
        .expect("persistence_add_history");
    // @step When persistence_get_history(SessionId("s-1"), 10) is awaited
    let out = backend
        .persistence_get_history(SessionId::new("s-1"), 10)
        .await
        .expect("persistence_get_history");
    // @step Then the returned Vec<String> contains "global hello" regardless of project
    assert!(
        out.iter().any(|t| t == "global hello"),
        "expected 'global hello' in {out:?}"
    );
}
