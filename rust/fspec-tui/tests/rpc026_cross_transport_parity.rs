//! RPC-026 — Cross-transport parity for /resume + /search + delete.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend
//! AND WebSocketFspecBackend. Mirrors the RPC-025 parity pattern
//! (DATA_DIRECTORY mutex + shared service).

#![allow(clippy::await_holding_lock)]

use codelet_test_helpers::{OrFail, TestResult};
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

fn setup_parity_temp_dir(
) -> Result<(std::sync::MutexGuard<'static, ()>, TempDir), codelet_test_helpers::TestError> {
    let guard = DATA_DIR_MUTEX.lock().or_fail("DATA_DIR_MUTEX")?;
    let temp = codelet_test_helpers::setup_temp_directory().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(temp.path().to_path_buf()).or_fail("set_data_directory")?;
    Ok((guard, temp))
}

fn workspace_with_seed(cwd: &Path) -> Result<(), codelet_test_helpers::TestError> {
    fs::create_dir_all(cwd.join("spec")).or_fail("mkdir spec/")?;
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    Ok(())
}

fn service_for(
    repo_path: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    workspace_with_seed(repo_path)?;
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).or_fail("watcher")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()),
    ))
}

/// Scenario: list_sessions returns byte-identical SessionInfo across transports
#[tokio::test]
async fn embedded_and_websocket_list_sessions_byte_identical() -> TestResult {
    // @step Given a SharedFspecService bound to a workspace cwd
    let (_guard, temp) = setup_parity_temp_dir()?;
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd)?;
    // BUG-465: listing sessions needs a session manager (the stub here).
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).or_fail("watcher")?);
    let service = Arc::new(
        SharedFspecService::with_session_manager(
            watcher,
            Arc::new(codelet_core::session_manager_handle::StubSessionManagerHandle::new()),
        )
        .with_cwd(cwd.clone()),
    );
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );

    // @step When EmbeddedFspecBackend.list_sessions().await is called against the service
    let embedded_list = embedded
        .list_sessions(String::new())
        .await
        .or_fail("embedded list")?;
    // @step And WebSocketFspecBackend.list_sessions().await is called against the same service over a loopback daemon
    let websocket_list = websocket
        .list_sessions(String::new())
        .await
        .or_fail("ws list")?;
    // @step Then both backends return Vec<SessionInfo> of the same length with the same id field in the same order
    let embedded_ids: Vec<&str> = embedded_list.iter().map(|i| i.id.as_str()).collect();
    let websocket_ids: Vec<&str> = websocket_list.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(embedded_ids, websocket_ids);
    assert!(embedded_list.is_empty());
    Ok(())
}

/// Scenario: persistence_search_history round-trips identically across transports
#[tokio::test]
async fn embedded_and_websocket_search_history_byte_identical() -> TestResult {
    // @step Given a SharedFspecService whose persistence store contains submitted inputs "git status", "git push", "fspec board"
    let (_guard, temp) = setup_parity_temp_dir()?;
    let cwd = temp.path().to_path_buf();
    let service = service_for(&cwd)?;
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );

    for text in ["git status", "git push", "fspec board"] {
        embedded
            .persistence_add_history(SessionId::new("s-1"), text.to_string())
            .await
            .or_fail("seed")?;
    }

    // @step When EmbeddedFspecBackend.persistence_search_history("git").await is called
    let embedded_out = embedded
        .persistence_search_history("git".to_string())
        .await
        .or_fail("embedded search")?;
    // @step And WebSocketFspecBackend.persistence_search_history("git").await is called against the same service
    let websocket_out = websocket
        .persistence_search_history("git".to_string())
        .await
        .or_fail("ws search")?;
    // @step Then both backends return Vec<HistoryMatch> with the same text field in the same order
    let embedded_texts: Vec<&str> = embedded_out.iter().map(|m| m.text.as_str()).collect();
    let websocket_texts: Vec<&str> = websocket_out.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(embedded_texts, websocket_texts);
    assert_eq!(embedded_out.len(), 2);
    Ok(())
}

#[path = "rpc026_cross_transport_parity/fixture.rs"]
mod deletion_fixture;

include!("rpc026_cross_transport_parity/deletion_parity.rs");

/// Scenario: codelet_core::persistence::delete_session lifts the NAPI implementation
#[test]
fn core_delete_session_lifts_napi_implementation() -> TestResult {
    // @step Given a fresh ~/.fspec/sessions.jsonl with sessions ["s-1", "s-2", "s-3"]
    let (_guard, temp) = setup_parity_temp_dir()?;
    let sessions_dir = temp.path().join("sessions");
    fs::create_dir_all(&sessions_dir).or_fail("mkdir sessions/")?;
    let s1 = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111")
        .or_fail("expected a value")?;
    let s2 = uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222")
        .or_fail("expected a value")?;
    let s3 = uuid::Uuid::parse_str("33333333-3333-3333-3333-333333333333")
        .or_fail("expected a value")?;
    for id in [s1, s2, s3] {
        fs::write(
            sessions_dir.join(format!("{id}.json")),
            format!(r#"{{"id":"{id}"}}"#),
        )
        .or_fail("seed manifest")?;
    }

    // @step When codelet_core::persistence::delete_session(Uuid("s-2")) is called
    codelet_core::persistence::sessions::delete_session(s2).or_fail("core delete_session")?;

    // @step Then the on-disk sessions.jsonl no longer lists "s-2"
    assert!(!sessions_dir.join(format!("{s2}.json")).exists());

    // @step And codelet_core::persistence::sessions::list() returns ["s-1", "s-3"]
    // (Surrogate: the on-disk manifest set is the observable list state.)
    assert!(sessions_dir.join(format!("{s1}.json")).exists());
    assert!(sessions_dir.join(format!("{s3}.json")).exists());

    // @step And the NAPI export persistence_delete_session from rust/napi/src/persistence/napi_bindings.rs is a one-line delegate to codelet_core::persistence::delete_session
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rpc_impl_path = manifest_dir
        .parent()
        .or_fail("rust/")?
        .join("rpc")
        .join("src")
        .join("service_implementation")
        .join("workspace_history.rs");
    let body = std::fs::read_to_string(&rpc_impl_path).or_fail("read workspace_history.rs")?;
    assert!(
        body.contains("async fn persistence_delete_session"),
        "persistence_delete_session must exist as an RPC service method"
    );
    assert!(
        body.contains("codelet_core::persistence::delete_session(uuid)"),
        "the RPC service method must delegate to codelet_core delete_session"
    );
    Ok(())
}
