//! RPC-338 — Cross-transport parity for the new `ProviderInfo`
//! `profile_name` / `is_unreachable` wire fields.
//!
//! Feature: spec/features/model-selector-profile-sections.feature
//!
//! Seeds a deterministic StubSessionManagerHandle with a local-server
//! profile section AND a cloud provider, then asserts that
//! `list_providers()` returns byte-identical `profile_name` /
//! `is_unreachable` values over both the EmbeddedFspecBackend and the
//! WebSocketFspecBackend. Mirrors the RPC-054 parity harness.

#![allow(clippy::await_holding_lock)]

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{ModelEntry, ProviderInfo};
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) -> Result<(), codelet_test_helpers::TestError> {
    fs::create_dir_all(cwd.join("spec")).or_fail("mkdir spec/")?;
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    Ok(())
}

fn build_service() -> Result<
    (
        TempDir,
        Arc<SharedFspecService>,
        Arc<StubSessionManagerHandle>,
    ),
    codelet_test_helpers::TestError,
> {
    let temp = tempfile::tempdir().or_fail("tempdir")?;
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd)?;
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).or_fail("watcher")?);
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    Ok((temp, service, stub))
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> Result<(Arc<dyn FspecBackend>, Arc<dyn FspecBackend>), codelet_test_helpers::TestError> {
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
    Ok((embedded, websocket))
}

fn seed_providers() -> Vec<ProviderInfo> {
    vec![
        // Cloud provider: no profile, reachable.
        ProviderInfo {
            key: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            models: vec![ModelEntry {
                id: "claude-sonnet".to_string(),
                display_name: "claude-sonnet".to_string(),
                context_window: Some(200_000),
                supports_reasoning: true,
                supports_vision: true,
                is_custom: false,
            }],
            profile_name: None,
            is_unreachable: false,
            provider_id: "anthropic".to_string(),
            unavailable_reason: None,
        },
        // Local-server profile section: unreachable.
        ProviderInfo {
            key: "openai:down-profile".to_string(),
            display_name: "openai: down-profile".to_string(),
            models: Vec::new(),
            profile_name: Some("down-profile".to_string()),
            is_unreachable: true,
            provider_id: "openai".to_string(),
            unavailable_reason: None,
        },
    ]
}

/// Scenario: Both transports return identical profile and reachability fields
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_providers_profile_fields_are_identical_across_transports() -> TestResult {
    // @step Given a provider set containing a local profile section and a cloud provider
    let (_temp, service, stub) = build_service()?;
    stub.set_providers(seed_providers());
    let (embedded, websocket) = dual_backends(service).await?;

    // @step When list_providers() is called over the in-process transport
    let em = embedded
        .list_providers()
        .await
        .or_fail("embedded list_providers")?;

    // @step And list_providers() is called over the websocket transport
    let ws = websocket
        .list_providers()
        .await
        .or_fail("websocket list_providers")?;

    // @step Then both responses contain the same profile_name values for every provider
    let em_profiles: Vec<Option<String>> = em.iter().map(|p| p.profile_name.clone()).collect();
    let ws_profiles: Vec<Option<String>> = ws.iter().map(|p| p.profile_name.clone()).collect();
    assert_eq!(em_profiles, ws_profiles, "profile_name parity");
    assert_eq!(
        em_profiles,
        vec![None, Some("down-profile".to_string())],
        "expected seeded profile_name values"
    );

    // @step And both responses contain the same is_unreachable values for every provider
    let em_reach: Vec<bool> = em.iter().map(|p| p.is_unreachable).collect();
    let ws_reach: Vec<bool> = ws.iter().map(|p| p.is_unreachable).collect();
    assert_eq!(em_reach, ws_reach, "is_unreachable parity");
    assert_eq!(
        em_reach,
        vec![false, true],
        "expected seeded reachability values"
    );

    // Full structural parity as a belt-and-braces check.
    assert_eq!(em, ws, "full ProviderInfo parity across transports");
    Ok(())
}
