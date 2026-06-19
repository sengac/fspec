//! RPC-054 — Cross-transport parity for the new provider-credentials surface.
//!
//! Feature: spec/features/rpc054-provider-settings-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-049 / RPC-050 parity pattern.

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
use codelet_rpc_types::{ProviderCredentialInfo, ProviderCredentialInput};
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

/// Scenario: Embedded and WebSocket transports both reach the same StubSessionManagerHandle
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn set_provider_credentials_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;

    let initial_set = stub.set_provider_credentials_calls();

    // @step When set_provider_credentials("openai", ApiKey{"sk-1"}) is called via the embedded transport
    let em_result = embedded
        .set_provider_credentials(
            "openai".to_string(),
            ProviderCredentialInput::api_key("sk-1"),
        )
        .await;
    assert!(em_result.is_ok(), "embedded set: {em_result:?}");

    // @step And set_provider_credentials("openai", ApiKey{"sk-2"}) is called via the WebSocket transport
    let ws_result = websocket
        .set_provider_credentials(
            "openai".to_string(),
            ProviderCredentialInput::api_key("sk-2"),
        )
        .await;
    assert!(ws_result.is_ok(), "websocket set: {ws_result:?}");

    // @step Then the stub's set_provider_credentials_calls counter equals 2
    let final_set = stub.set_provider_credentials_calls();
    assert_eq!(
        final_set - initial_set,
        2,
        "stub.set_provider_credentials_calls() should increment by 2 (once per transport)"
    );
}

/// Scenario: Embedded and WebSocket test_provider_connection both reach the stub
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_connection_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;

    let initial = stub.test_provider_connection_calls();

    // @step When test_provider_connection("openai") is called via the embedded transport
    let em = embedded
        .test_provider_connection("openai".to_string())
        .await
        .expect("embedded test");

    // @step And test_provider_connection("openai") is called via the WebSocket transport
    let ws = websocket
        .test_provider_connection("openai".to_string())
        .await
        .expect("websocket test");

    // @step Then the stub's test_provider_connection_calls counter equals 2
    let final_calls = stub.test_provider_connection_calls();
    assert_eq!(
        final_calls - initial,
        2,
        "stub.test_provider_connection_calls() should increment by 2 (once per transport)"
    );

    // @step And both calls returned a TestConnectionResult with success=true
    assert!(em.success, "embedded result success");
    assert!(ws.success, "websocket result success");
}

/// Verify list/get/delete/refresh all round-trip cleanly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_credential_lifecycle_round_trips_across_transports() {
    let (_temp, service, stub) = build_service();
    // Pre-seed the stub so list/get observe state.
    stub.seed_provider_credential(ProviderCredentialInfo {
        provider_id: "anthropic".to_string(),
        display_name: "anthropic".to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 4,
        masked_key: None,
        source: None,
    });
    let (embedded, websocket) = dual_backends(service).await;

    let em_list = embedded
        .list_provider_credentials()
        .await
        .expect("embedded list");
    let ws_list = websocket
        .list_provider_credentials()
        .await
        .expect("websocket list");
    assert_eq!(em_list, ws_list);
    assert_eq!(em_list.len(), 1);
    assert_eq!(em_list[0].provider_id, "anthropic");

    let em_get = embedded
        .get_provider_credential("anthropic".to_string())
        .await
        .expect("embedded get");
    let ws_get = websocket
        .get_provider_credential("anthropic".to_string())
        .await
        .expect("websocket get");
    assert_eq!(em_get, ws_get);
    assert!(em_get.is_some());

    let initial_refresh = stub.refresh_models_cache_calls();
    embedded
        .refresh_models_cache("anthropic".to_string())
        .await
        .expect("embedded refresh");
    websocket
        .refresh_models_cache("anthropic".to_string())
        .await
        .expect("websocket refresh");
    assert_eq!(
        stub.refresh_models_cache_calls() - initial_refresh,
        2,
        "refresh_models_cache_calls should increment by 2"
    );

    let initial_delete = stub.delete_provider_credentials_calls();
    embedded
        .delete_provider_credentials("anthropic".to_string())
        .await
        .expect("embedded delete");
    websocket
        .delete_provider_credentials("anthropic".to_string())
        .await
        .expect("websocket delete");
    assert_eq!(
        stub.delete_provider_credentials_calls() - initial_delete,
        2,
        "delete_provider_credentials_calls should increment by 2"
    );

    // After delete the row's configured indicator should be false on both transports.
    let post = embedded
        .get_provider_credential("anthropic".to_string())
        .await
        .expect("post-delete get");
    assert!(post.is_some());
    assert!(
        !post.unwrap().configured,
        "row should be unconfigured after delete"
    );
}
