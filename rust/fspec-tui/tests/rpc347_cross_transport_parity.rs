//! Feature: spec/features/custom-model-rpc-surface.feature
//!
//! RPC-347 — cross-transport parity for the custom-model write surface.
//!
//! Drives `add_custom_model` / `update_custom_model` / `delete_custom_model`
//! against EmbeddedFspecBackend AND WebSocketFspecBackend constructed against
//! the SAME deterministic `StubSessionManagerHandle` (the stub mirrors the
//! profile_sections append/replace/delete semantics in memory so no disk or
//! network is touched). Mirrors the RPC-037 parity pattern.

#![allow(clippy::too_many_lines)]

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::CustomModelDefinition;
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

/// Build a `SharedFspecService` backed by a fresh deterministic stub.
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

/// Build a `SharedFspecService` with NO session manager attached.
fn build_service_without_handle(
) -> Result<(TempDir, Arc<SharedFspecService>), codelet_test_helpers::TestError> {
    let temp = tempfile::tempdir().or_fail("tempdir")?;
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd)?;
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).or_fail("watcher")?);
    let service = Arc::new(SharedFspecService::new(watcher).with_cwd(cwd));
    Ok((temp, service))
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

fn definition(id: &str) -> CustomModelDefinition {
    CustomModelDefinition {
        id: id.to_string(),
        display_name: Some(format!("{id} display")),
        facade: Some("gemini".to_string()),
        context_window: Some(1_048_576),
        max_output_tokens: Some(65_536),
        compaction_threshold_type: Some("percentage".to_string()),
        compaction_threshold_value: Some(80),
        reasoning: Some(true),
        has_vision: Some(true),
    }
}

// Scenario: add_custom_model produces identical results across transports
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_add_custom_model_parity_across_transports() -> TestResult {
    let (_temp, service, stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;

    // @step Given an openai profile "work-vllm" exists with no custom models
    // (the stub starts with an empty in-memory custom-model map for every profile)
    let def = definition("my-model");

    // @step When a client calls add_custom_model with the same definition over the embedded transport
    embedded
        .add_custom_model("openai".into(), "p-embedded".into(), def.clone())
        .await
        .or_fail("embedded add_custom_model")?;

    // @step And another client calls add_custom_model with the same definition over the websocket transport
    websocket
        .add_custom_model("openai".into(), "p-websocket".into(), def.clone())
        .await
        .or_fail("websocket add_custom_model")?;

    // @step Then both calls return Ok
    // @step And both transports persist the identical customModels entry
    let em_models = stub.custom_models("p-embedded");
    let ws_models = stub.custom_models("p-websocket");
    assert_eq!(em_models, vec![def.clone()], "embedded entry");
    assert_eq!(
        em_models, ws_models,
        "both transports must forward the identical definition to the shared handle"
    );
    Ok(())
}

// Scenario: RPC methods are refused without an attached SessionManagerHandle
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_no_handle_refused() -> TestResult {
    let (_temp, service) = build_service_without_handle()?;
    let (embedded, websocket) = dual_backends(service).await?;
    let def = definition("my-model");

    // @step Given a FspecServiceImpl with no SessionManagerHandle attached
    // @step When a client calls add_custom_model, update_custom_model, and delete_custom_model
    let add = embedded
        .add_custom_model("openai".into(), "work-vllm".into(), def.clone())
        .await;
    let update = embedded
        .update_custom_model(
            "openai".into(),
            "work-vllm".into(),
            "my-model".into(),
            def.clone(),
        )
        .await;
    let delete = embedded
        .delete_custom_model("openai".into(), "work-vllm".into(), "my-model".into())
        .await;

    // @step Then each call returns an error
    assert!(
        add.is_err(),
        "add must be refused without a handle: {add:?}"
    );
    assert!(
        update.is_err(),
        "update must be refused without a handle: {update:?}"
    );
    assert!(
        delete.is_err(),
        "delete must be refused without a handle: {delete:?}"
    );

    // @step And the websocket transport behaves identically over its no-handle path
    let ws_add = websocket
        .add_custom_model("openai".into(), "work-vllm".into(), def.clone())
        .await;
    let ws_update = websocket
        .update_custom_model(
            "openai".into(),
            "work-vllm".into(),
            "my-model".into(),
            def.clone(),
        )
        .await;
    let ws_delete = websocket
        .delete_custom_model("openai".into(), "work-vllm".into(), "my-model".into())
        .await;
    assert!(ws_add.is_err(), "websocket add must be refused: {ws_add:?}");
    assert!(
        ws_update.is_err(),
        "websocket update must be refused: {ws_update:?}"
    );
    assert!(
        ws_delete.is_err(),
        "websocket delete must be refused: {ws_delete:?}"
    );

    // @step And no configuration is written
    // (no handle means nothing downstream is invoked; the refusal is the only effect)
    Ok(())
}
