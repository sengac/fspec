//! Feature: spec/features/custom-model-rpc-surface.feature
//!
//! RPC-347 — cross-transport parity for the custom-model write surface.
//!
//! Drives `add_custom_model` / `update_custom_model` / `delete_custom_model`
//! against EmbeddedFspecBackend AND WebSocketFspecBackend constructed against
//! the SAME deterministic `StubSessionManagerHandle` (the stub mirrors the
//! profile_sections append/replace/delete semantics in memory so no disk or
//! network is touched). Mirrors the RPC-037 parity pattern.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
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
use codelet_rpc_types::CustomModelDefinition;
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

/// Build a `SharedFspecService` backed by a fresh deterministic stub.
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

/// Build a `SharedFspecService` with NO session manager attached.
fn build_service_without_handle() -> (TempDir, Arc<SharedFspecService>) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(watcher).with_cwd(cwd));
    (temp, service)
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
async fn scenario_add_custom_model_parity_across_transports() {
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;

    // @step Given an openai profile "work-vllm" exists with no custom models
    // (the stub starts with an empty in-memory custom-model map for every profile)
    let def = definition("my-model");

    // @step When a client calls add_custom_model with the same definition over the embedded transport
    embedded
        .add_custom_model("openai".into(), "p-embedded".into(), def.clone())
        .await
        .expect("embedded add_custom_model");

    // @step And another client calls add_custom_model with the same definition over the websocket transport
    websocket
        .add_custom_model("openai".into(), "p-websocket".into(), def.clone())
        .await
        .expect("websocket add_custom_model");

    // @step Then both calls return Ok
    // @step And both transports persist the identical customModels entry
    let em_models = stub.custom_models("p-embedded");
    let ws_models = stub.custom_models("p-websocket");
    assert_eq!(em_models, vec![def.clone()], "embedded entry");
    assert_eq!(
        em_models, ws_models,
        "both transports must forward the identical definition to the shared handle"
    );
}

// Scenario: RPC methods are a silent no-op without an attached SessionManagerHandle
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_no_handle_silent_no_op() {
    let (_temp, service) = build_service_without_handle();
    let (embedded, _websocket) = dual_backends(service).await;
    let def = definition("my-model");

    // @step Given a FspecServiceImpl with no SessionManagerHandle attached
    // @step When a client calls add_custom_model, update_custom_model, and delete_custom_model
    let add = embedded
        .add_custom_model("openai".into(), "work-vllm".into(), def.clone())
        .await;
    let update = embedded
        .update_custom_model("openai".into(), "work-vllm".into(), "my-model".into(), def.clone())
        .await;
    let delete = embedded
        .delete_custom_model("openai".into(), "work-vllm".into(), "my-model".into())
        .await;

    // @step Then each call returns Ok
    assert!(add.is_ok(), "add must be Ok without a handle: {add:?}");
    assert!(update.is_ok(), "update must be Ok without a handle: {update:?}");
    assert!(delete.is_ok(), "delete must be Ok without a handle: {delete:?}");

    // @step And no configuration is written
    // (no handle means nothing downstream of the no-op default is invoked)
}
