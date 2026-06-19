//! RPC-022 — Cross-transport parity for `FspecBackend::list_providers`,
//! `set_session_model`, `set_thinking_level`, `get_session_role`,
//! `set_session_role`.
//!
//! Feature: spec/features/rpc022-cross-transport-parity.feature
//!
//! Mirrors the RPC-018 cross-transport-parity pattern: drives the SAME
//! scripted scenario against BOTH transports and asserts identical
//! results.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::Arc;

use codelet_core::session_manager_handle::StubSessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId, ThinkingLevel};
use tempfile::TempDir;

fn service_without_cwd() -> Arc<SharedFspecService> {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    Box::leak(Box::new(tmp));
    Arc::new(SharedFspecService::new(watcher))
}

fn service_with_stub() -> Arc<SharedFspecService> {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    Box::leak(Box::new(tmp));
    let stub = Arc::new(StubSessionManagerHandle::new());
    Arc::new(SharedFspecService::with_session_manager(watcher, stub))
}

fn fixture_openai_provider() -> ProviderInfo {
    ProviderInfo {
        key: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        models: vec![ModelEntry {
            id: "gpt-5.1-codex".to_string(),
            display_name: "gpt-5.1-codex".to_string(),
            context_window: 200_000,
            supports_reasoning: true,
            supports_vision: false,
            is_custom: false,
        }],
        profile_name: None,
        is_unreachable: false,
    }
}

fn service_with_seeded_providers(providers: Vec<ProviderInfo>) -> Arc<SharedFspecService> {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec")).expect("mkdir spec");
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).expect("WorkUnitsWatcher::new"));
    Box::leak(Box::new(tmp));
    let stub = StubSessionManagerHandle::new();
    stub.set_providers(providers);
    let stub_arc = Arc::new(stub);
    Arc::new(SharedFspecService::with_session_manager(watcher, stub_arc))
}

/// Scenario: list_providers returns empty Vec when no session manager is attached (embedded)
#[tokio::test]
async fn list_providers_returns_empty_vec_when_no_session_manager_attached_embedded() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.list_providers().await is invoked
    let providers = backend.list_providers().await.expect("list_providers");
    // @step Then the awaited result is Ok(vec![])
    assert!(providers.is_empty(), "expected empty providers");
}

/// Scenario: list_providers crosses tarpc cleanly when no session manager is attached
#[tokio::test]
async fn list_providers_crosses_tarpc_cleanly_when_no_session_manager_attached() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.list_providers().await is invoked
    let providers = backend.list_providers().await.expect("list_providers");
    // @step Then the awaited result is Ok(vec![])
    assert!(providers.is_empty());
}

/// Scenario: Both transports return identical providers for the same SharedFspecService
#[tokio::test]
async fn both_transports_return_identical_providers_for_the_same_shared_service() {
    // @step Given a SharedFspecService with a session manager that returns [ProviderInfo{ key: "openai", display_name: "OpenAI", models: vec![ModelEntry{ id: "gpt-5.1-codex", display_name: "gpt-5.1-codex", context_window: 200_000, supports_reasoning: true, supports_vision: false, is_custom: false }]}]
    let expected = vec![fixture_openai_provider()];
    let service = service_with_seeded_providers(expected.clone());
    // @step And an rpc-server bound to that shared service
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And an EmbeddedFspecBackend wrapping the same shared service
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::clone(&service),
    ));
    // @step And a WebSocketFspecBackend connected to the rpc-server
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.list_providers().await is invoked on BOTH backends
    let embedded_providers = embedded
        .list_providers()
        .await
        .expect("embedded list_providers");
    let websocket_providers = websocket
        .list_providers()
        .await
        .expect("websocket list_providers");
    // @step Then both awaited results are equal
    assert_eq!(embedded_providers, expected);
    assert_eq!(websocket_providers, expected);
    assert_eq!(embedded_providers, websocket_providers);
}

/// Scenario: set_session_model returns Ok when no session manager is attached (embedded)
#[tokio::test]
async fn set_session_model_returns_ok_when_no_session_manager_attached_embedded() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.set_session_model(SessionId::new("anything"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    let result = backend
        .set_session_model(
            SessionId::new("anything"),
            "openai".to_string(),
            "gpt-5.1-codex".to_string(),
        )
        .await;
    // @step Then the awaited result is Ok(())
    result.expect("set_session_model");
}

/// Scenario: set_session_model crosses tarpc cleanly when no session manager is attached
#[tokio::test]
async fn set_session_model_crosses_tarpc_cleanly_when_no_session_manager_attached() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.set_session_model(SessionId::new("anything"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    let result = backend
        .set_session_model(
            SessionId::new("anything"),
            "openai".to_string(),
            "gpt-5.1-codex".to_string(),
        )
        .await;
    // @step Then the awaited result is Ok(())
    result.expect("set_session_model");
}

/// Scenario: set_thinking_level returns Ok when no session manager is attached (embedded)
#[tokio::test]
async fn set_thinking_level_returns_ok_when_no_session_manager_attached_embedded() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::High).await is invoked
    let result = backend
        .set_thinking_level(SessionId::new("anything"), ThinkingLevel::High)
        .await;
    // @step Then the awaited result is Ok(())
    result.expect("set_thinking_level");
}

/// Scenario: set_thinking_level crosses tarpc cleanly with safe default
#[tokio::test]
async fn set_thinking_level_crosses_tarpc_cleanly_with_safe_default() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::Medium).await is invoked
    let result = backend
        .set_thinking_level(SessionId::new("anything"), ThinkingLevel::Medium)
        .await;
    // @step Then the awaited result is Ok(())
    result.expect("set_thinking_level");
}

/// Scenario: get_session_role returns None when no session manager is attached (embedded)
#[tokio::test]
async fn get_session_role_returns_none_when_no_session_manager_attached_embedded() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_session_role(SessionId::new("anything")).await is invoked
    let role = backend
        .get_session_role(SessionId::new("anything"))
        .await
        .expect("get_session_role");
    // @step Then the awaited result is Ok(None)
    assert!(role.is_none());
}

/// Scenario: get_session_role crosses tarpc cleanly with safe default
#[tokio::test]
async fn get_session_role_crosses_tarpc_cleanly_with_safe_default() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.get_session_role(SessionId::new("anything")).await is invoked
    let role = backend
        .get_session_role(SessionId::new("anything"))
        .await
        .expect("get_session_role");
    // @step Then the awaited result is Ok(None)
    assert!(role.is_none());
}

/// Scenario: set_session_role returns Ok when no session manager is attached (embedded)
#[tokio::test]
async fn set_session_role_returns_ok_when_no_session_manager_attached_embedded() {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    backend
        .set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string()))
        .await
        .expect("set_session_role(Some)");
    // @step Then the awaited result is Ok(())
    // @step When backend.set_session_role(SessionId::new("anything"), None).await is invoked
    backend
        .set_session_role(SessionId::new("anything"), None)
        .await
        .expect("set_session_role(None)");
    // @step Then the awaited result is Ok(())
}

/// Scenario: set_session_role crosses tarpc cleanly with safe default
#[tokio::test]
async fn set_session_role_crosses_tarpc_cleanly_with_safe_default() {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd();
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    // @step When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    backend
        .set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string()))
        .await
        .expect("set_session_role");
    // @step Then the awaited result is Ok(())
}

/// Scenario: StubSessionManagerHandle inherits the default SessionManagerHandle implementations for the five new RPC methods
#[tokio::test]
async fn stub_session_manager_handle_inherits_default_impls_for_rpc022() {
    // @step Given a SharedFspecService constructed via with_session_manager(stub_handle, watcher)
    let service = service_with_stub();
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.list_providers().await is invoked
    let providers = backend.list_providers().await.expect("list_providers");
    // @step Then the awaited result is Ok(vec![])
    assert!(providers.is_empty());
    // @step When backend.get_session_role(SessionId::new("stub-1")).await is invoked
    let role = backend
        .get_session_role(SessionId::new("stub-1"))
        .await
        .expect("get_session_role");
    // @step Then the awaited result is Ok(None)
    assert!(role.is_none());
    // @step When backend.set_session_model(SessionId::new("stub-1"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    backend
        .set_session_model(
            SessionId::new("stub-1"),
            "openai".to_string(),
            "gpt-5.1-codex".to_string(),
        )
        .await
        .expect("set_session_model");
    // @step Then the awaited result is Ok(())
    // @step When backend.set_thinking_level(SessionId::new("stub-1"), ThinkingLevel::Off).await is invoked
    backend
        .set_thinking_level(SessionId::new("stub-1"), ThinkingLevel::Off)
        .await
        .expect("set_thinking_level");
    // @step Then the awaited result is Ok(())
    // @step When backend.set_session_role(SessionId::new("stub-1"), None).await is invoked
    backend
        .set_session_role(SessionId::new("stub-1"), None)
        .await
        .expect("set_session_role");
    // @step Then the awaited result is Ok(())
}
