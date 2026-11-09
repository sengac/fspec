//! RPC-022 — Cross-transport parity for `FspecBackend::list_providers`,
//! `set_session_model`, `set_thinking_level`, `get_session_role`,
//! `set_session_role`.
//!
//! Feature: spec/features/rpc022-cross-transport-parity.feature
//!
//! Mirrors the RPC-018 cross-transport-parity pattern: drives the SAME
//! scripted scenario against BOTH transports and asserts identical
//! results.

use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::fs;
use std::sync::Arc;

use codelet_core::session_manager_handle::StubSessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId, ThinkingLevel};
use tempfile::TempDir;

fn service_without_cwd() -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    fs::create_dir_all(tmp.path().join("spec")).or_fail("mkdir spec")?;
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).or_fail("WorkUnitsWatcher::new")?);
    Box::leak(Box::new(tmp));
    Ok(Arc::new(SharedFspecService::new(watcher)))
}

fn service_with_stub() -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    fs::create_dir_all(tmp.path().join("spec")).or_fail("mkdir spec")?;
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).or_fail("WorkUnitsWatcher::new")?);
    Box::leak(Box::new(tmp));
    let stub = Arc::new(StubSessionManagerHandle::new());
    Ok(Arc::new(SharedFspecService::with_session_manager(
        watcher, stub,
    )))
}

fn fixture_openai_provider() -> ProviderInfo {
    ProviderInfo {
        key: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        models: vec![ModelEntry {
            id: "gpt-5.1-codex".to_string(),
            display_name: "gpt-5.1-codex".to_string(),
            context_window: Some(200_000),
            supports_reasoning: true,
            supports_vision: false,
            is_custom: false,
        }],
        profile_name: None,
        is_unreachable: false,
        provider_id: "openai".to_string(),
        unavailable_reason: None,
    }
}

fn service_with_seeded_providers(
    providers: Vec<ProviderInfo>,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let tmp = TempDir::new().or_fail("tempdir")?;
    fs::create_dir_all(tmp.path().join("spec")).or_fail("mkdir spec")?;
    fs::write(
        tmp.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(tmp.path()).or_fail("WorkUnitsWatcher::new")?);
    Box::leak(Box::new(tmp));
    let stub = StubSessionManagerHandle::new();
    stub.set_providers(providers);
    let stub_arc = Arc::new(stub);
    Ok(Arc::new(SharedFspecService::with_session_manager(
        watcher, stub_arc,
    )))
}

/// Scenario: list_providers is refused when no session manager is attached (embedded) (RPC-451)
#[tokio::test]
async fn list_providers_is_refused_when_no_session_manager_attached_embedded() -> TestResult {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd()?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.list_providers().await is invoked
    let listed = backend.list_providers().await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = listed.err().ok_or("providers were listed")?.to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: list_providers crosses tarpc cleanly with the refusal when no session manager is attached (RPC-451)
#[tokio::test]
async fn list_providers_crosses_tarpc_cleanly_with_the_refusal() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd()?;
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
    // @step When backend.list_providers().await is invoked
    let listed = backend.list_providers().await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = listed.err().ok_or("providers were listed")?.to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: Both transports return identical providers for the same SharedFspecService
#[tokio::test]
async fn both_transports_return_identical_providers_for_the_same_shared_service() -> TestResult {
    // @step Given a SharedFspecService with a session manager that returns [ProviderInfo{ key: "openai", display_name: "OpenAI", models: vec![ModelEntry{ id: "gpt-5.1-codex", display_name: "gpt-5.1-codex", context_window: 200_000, supports_reasoning: true, supports_vision: false, is_custom: false }]}]
    let expected = vec![fixture_openai_provider()];
    let service = service_with_seeded_providers(expected.clone())?;
    // @step And an rpc-server bound to that shared service
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    // @step And an EmbeddedFspecBackend wrapping the same shared service
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::clone(&service),
    ));
    // @step And a WebSocketFspecBackend connected to the rpc-server
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );
    // @step When backend.list_providers().await is invoked on BOTH backends
    let embedded_providers = embedded
        .list_providers()
        .await
        .or_fail("embedded list_providers")?;
    let websocket_providers = websocket
        .list_providers()
        .await
        .or_fail("websocket list_providers")?;
    // @step Then both awaited results are equal
    assert_eq!(embedded_providers, expected);
    assert_eq!(websocket_providers, expected);
    assert_eq!(embedded_providers, websocket_providers);
    Ok(())
}

/// Scenario: set_session_model is refused when no session manager is attached (embedded) (BUG-453)
#[tokio::test]
async fn set_session_model_is_refused_when_no_session_manager_attached_embedded() -> TestResult {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd()?;
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
    // @step Then the awaited result is an error saying no session manager is attached
    let error = result
        .err()
        .ok_or("set_session_model succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: set_session_model crosses tarpc cleanly with the refusal when no session manager is attached (BUG-453)
#[tokio::test]
async fn set_session_model_crosses_tarpc_cleanly_with_the_refusal() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd()?;
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
    // @step When backend.set_session_model(SessionId::new("anything"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    let result = backend
        .set_session_model(
            SessionId::new("anything"),
            "openai".to_string(),
            "gpt-5.1-codex".to_string(),
        )
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = result
        .err()
        .ok_or("set_session_model succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: set_thinking_level is refused when no session manager is attached (embedded) (BUG-453)
#[tokio::test]
async fn set_thinking_level_is_refused_when_no_session_manager_attached_embedded() -> TestResult {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd()?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::Effort("high")).await is invoked
    let result = backend
        .set_thinking_level(
            SessionId::new("anything"),
            ThinkingLevel::Effort("high".to_string()),
        )
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = result
        .err()
        .ok_or("set_thinking_level succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: set_thinking_level crosses tarpc cleanly with the refusal (BUG-453)
#[tokio::test]
async fn set_thinking_level_crosses_tarpc_cleanly_with_the_refusal() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd()?;
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
    // @step When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::Effort("medium")).await is invoked
    let result = backend
        .set_thinking_level(
            SessionId::new("anything"),
            ThinkingLevel::Effort("medium".to_string()),
        )
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = result
        .err()
        .ok_or("set_thinking_level succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: get_session_role is refused when no session manager is attached (embedded) (BUG-445)
#[tokio::test]
async fn get_session_role_is_refused_when_no_session_manager_attached_embedded() -> TestResult {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd()?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.get_session_role(SessionId::new("anything")).await is invoked
    let read = backend.get_session_role(SessionId::new("anything")).await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = read.err().ok_or("a role was read")?.to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: get_session_role crosses tarpc cleanly with the refusal when no session manager is attached (BUG-445)
#[tokio::test]
async fn get_session_role_crosses_tarpc_cleanly_with_the_refusal() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd()?;
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
    // @step When backend.get_session_role(SessionId::new("anything")).await is invoked
    let read = backend.get_session_role(SessionId::new("anything")).await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = read.err().ok_or("a role was read")?.to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: set_session_role is refused when no session manager is attached (embedded) (BUG-453)
#[tokio::test]
async fn set_session_role_is_refused_when_no_session_manager_attached_embedded() -> TestResult {
    // @step Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    let service = service_without_cwd()?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    let some = backend
        .set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string()))
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = some
        .err()
        .ok_or("set_session_role(Some) succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    // @step When backend.set_session_role(SessionId::new("anything"), None).await is invoked
    let none = backend
        .set_session_role(SessionId::new("anything"), None)
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = none
        .err()
        .ok_or("set_session_role(None) succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: set_session_role crosses tarpc cleanly with the refusal (BUG-453)
#[tokio::test]
async fn set_session_role_crosses_tarpc_cleanly_with_the_refusal() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService with NO session manager attached
    let service = service_without_cwd()?;
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
    // @step When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    let refused = backend
        .set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string()))
        .await;
    // @step Then the awaited result is an error saying no session manager is attached
    let error = refused
        .err()
        .ok_or("set_session_role succeeded")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

/// Scenario: StubSessionManagerHandle inherits the default SessionManagerHandle implementations for the five new RPC methods
#[tokio::test]
async fn stub_session_manager_handle_inherits_default_impls_for_rpc022() -> TestResult {
    // @step Given a SharedFspecService constructed via with_session_manager(stub_handle, watcher)
    let service = service_with_stub()?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    // @step When backend.list_providers().await is invoked
    let providers = backend.list_providers().await.or_fail("list_providers")?;
    // @step Then the awaited result is Ok(vec![])
    assert!(providers.is_empty());
    // @step When backend.get_session_role(SessionId::new("stub-1")).await is invoked
    let read = backend.get_session_role(SessionId::new("stub-1")).await;
    // @step Then the awaited result is an error saying get_role is not implemented by this session manager
    let error = read
        .or_fail_err("the stub refuses get_session_role")?
        .to_string();
    assert!(error.contains("get_role is not implemented"), "{error}");
    // @step When backend.set_session_model(SessionId::new("stub-1"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    backend
        .set_session_model(
            SessionId::new("stub-1"),
            "openai".to_string(),
            "gpt-5.1-codex".to_string(),
        )
        .await
        .or_fail("set_session_model")?;
    // @step Then the awaited result is Ok(())
    // @step When backend.set_thinking_level(SessionId::new("stub-1"), ThinkingLevel::Off).await is invoked
    backend
        .set_thinking_level(SessionId::new("stub-1"), ThinkingLevel::Off)
        .await
        .or_fail("set_thinking_level")?;
    // @step Then the awaited result is Ok(())
    // @step When backend.set_session_role(SessionId::new("stub-1"), None).await is invoked
    let set = backend
        .set_session_role(SessionId::new("stub-1"), None)
        .await;
    // @step Then the awaited result is an error saying set_role is not implemented by this session manager
    let error = set.or_fail_err("the stub refuses set_role")?.to_string();
    assert!(error.contains("set_role is not implemented"), "{error}");
    Ok(())
}
