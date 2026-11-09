//! BUG-465 — RPC lists and session controls report failures.
//!
//! Feature: spec/features/rpc-lists-and-controls-report-failures.feature

mod common; // BUG-487: TaskOutcome lives in tests/common
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use serial_test::serial;

#[path = "common/session_storage_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::StorageFixture;

const UNKNOWN: &str = "00000000-0000-4000-8000-000000000465";

/// An embedded backend over a service with no session manager and no
/// workspace directory.
fn bare_backend() -> Result<(tempfile::TempDir, EmbeddedFspecBackend), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    std::fs::create_dir_all(directory.path().join("spec"))?;
    std::fs::write(
        directory.path().join("spec/work-units.json"),
        "{\"workUnits\":{}}",
    )?;
    let watcher = Arc::new(WorkUnitsWatcher::new(directory.path())?);
    let service = Arc::new(SharedFspecService::new(watcher));
    Ok((
        directory,
        EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn session_lists_and_controls_report_an_unknown_session() -> TestResult {
    // @step Given a session manager with no session "00000000-0000-4000-8000-000000000465"
    let fixture = StorageFixture::new().await?;
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    let id = SessionId::new(UNKNOWN);
    // @step When its loops and subordinates are listed and it is interrupted
    let outcomes = vec![
        ("loop_list", handle.loop_list(&id).map(drop)),
        ("get_subordinates", handle.get_subordinates(&id).map(drop)),
        ("interrupt", handle.interrupt(&id)),
    ];
    // @step Then every call fails naming "00000000-0000-4000-8000-000000000465"
    for (method, outcome) in outcomes {
        let error = outcome
            .err()
            .ok_or(format!("{method} answered for an unknown session"))?;
        assert!(error.contains(UNKNOWN), "{method}: {error}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lists_and_controls_over_rpc_are_refused_without_a_session_manager() -> TestResult {
    // @step Given an RPC service with no session manager attached
    let (_directory, backend) = bare_backend()?;
    // @step When the sessions, live sessions and provider credentials are listed and a session is interrupted over the embedded transport
    let outcomes = vec![
        (
            "list_sessions",
            backend.list_sessions(String::new()).await.map(drop),
        ),
        (
            "list_live_sessions",
            backend.list_live_sessions().await.map(drop),
        ),
        (
            "list_provider_credentials",
            backend.list_provider_credentials().await.map(drop),
        ),
        (
            "interrupt",
            backend.interrupt(SessionId::new(UNKNOWN)).await,
        ),
    ];
    // @step Then every call fails saying no session manager is attached
    for (method, outcome) in outcomes {
        let error = outcome
            .err()
            .ok_or(format!("{method} answered without a session manager"))?
            .to_string();
        assert!(
            error.contains("no session manager is attached"),
            "{method}: {error}"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_lists_over_rpc_are_refused_without_a_workspace() -> TestResult {
    // @step Given an RPC service with no workspace directory
    let (_directory, backend) = bare_backend()?;
    // @step When the changed files, checkpoints and file search are requested over the embedded transport
    let outcomes = vec![
        ("changed_files", backend.changed_files().await.map(drop)),
        (
            "list_checkpoints",
            backend.list_checkpoints().await.map(drop),
        ),
        (
            "search_files",
            backend.search_files("src".to_string(), 10).await.map(drop),
        ),
    ];
    // @step Then every call fails instead of returning an empty list
    for (method, outcome) in outcomes {
        assert!(outcome.is_err(), "{method} answered without a workspace");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn a_failed_session_listing_is_reported() -> TestResult {
    // @step Given a data directory whose sessions path is a file, not a directory
    let data = tempfile::tempdir()?;
    std::fs::write(data.path().join("sessions"), "not a directory")?;
    let previous = codelet_common::get_data_dir().ok();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step When the sessions are listed through the session manager
    let listed = handle.list_sessions("");
    codelet_core::persistence::reset_stores_for_tests();
    if let Some(previous) = previous {
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(previous)?;
    }
    // @step Then the listing fails instead of returning an empty list
    let error = listed.err().ok_or("sessions were listed")?;
    assert!(error.contains("sessions"), "{error}");
    Ok(())
}
