//! RPC-440: read fspec's default model over the RPC.
//!
//! Feature: spec/features/get-default-model-rpc.feature

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_sessions::SessionManager;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A service backed by a real `SessionManager` whose data directory is a
/// fresh temp dir, so no persisted default model leaks in.
fn service(
    data: &Path,
    workspace: &Path,
) -> Result<(FspecServiceImpl, Arc<SessionManager>), Box<dyn std::error::Error>> {
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    std::fs::create_dir_all(workspace.join("spec"))?;
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let manager = SessionManager::new();
    let handle: Arc<dyn SessionManagerHandle> = manager.clone();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace)?);
    let shared = SharedFspecService::with_session_manager(watcher, handle);
    Ok((FspecServiceImpl::new(Arc::new(shared)), manager))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn the_default_model_round_trips_over_the_rpc() -> TestResult {
    // @step Given a daemon whose default model was set to "anthropic/claude-opus-4-5"
    let data = tempfile::tempdir()?;
    let workspace = tempfile::tempdir()?;
    let (service, _manager) = service(data.path(), workspace.path())?;
    service
        .clone()
        .set_default_model(
            tarpc::context::current(),
            "anthropic/claude-opus-4-5".into(),
        )
        .await?;

    // @step When a client calls get_default_model
    let model = service.get_default_model(tarpc::context::current()).await;

    // @step Then it receives "anthropic/claude-opus-4-5"
    assert_eq!(model.as_deref(), Some("anthropic/claude-opus-4-5"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn no_default_model_returns_none() -> TestResult {
    // @step Given a daemon with no default model set
    let data = tempfile::tempdir()?;
    let workspace = tempfile::tempdir()?;
    let (service, manager) = service(data.path(), workspace.path())?;
    assert_eq!(manager.get_default_model(), None);

    // @step When a client calls get_default_model
    let model = service.get_default_model(tarpc::context::current()).await;

    // @step Then it receives no model
    assert_eq!(model, None);
    Ok(())
}

#[test]
fn get_default_model_is_appended_after_every_existing_method() -> TestResult {
    // @step Given the FspecService methods in declaration order
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rpc/src/service_declaration");
    let mut methods = Vec::new();
    for fragment in [
        "workspace.rs",
        "providers_sessions.rs",
        "supervision_credentials.rs",
        "session_mcp.rs",
    ] {
        for line in std::fs::read_to_string(dir.join(fragment))?.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("async fn ") {
                methods.push(
                    rest.split(['(', '<'])
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                );
            }
        }
    }

    // @step When the index of get_default_model is read
    let index = methods.iter().position(|name| name == "get_default_model");

    // @step Then get_default_model keeps index 127, after every method declared before it
    // (RPC-449: later methods, e.g. RIG-016 submit_input, are appended after it.)
    assert_eq!(index, Some(127));
    Ok(())
}
