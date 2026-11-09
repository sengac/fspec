//! BUG-445 — session getters report lookup failures.
//!
//! Feature: spec/features/session-getters-report-lookup-failures.feature

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

const UNKNOWN: &str = "00000000-0000-4000-8000-000000000445";

/// Every per-session getter's outcome as `Ok(())` or the error text.
fn read_every_getter(
    handle: &dyn SessionManagerHandle,
    id: &SessionId,
) -> Vec<(&'static str, Result<(), String>)> {
    vec![
        ("get_debug_enabled", handle.get_debug_enabled(id).map(drop)),
        ("get_pause_state", handle.get_pause_state(id).map(drop)),
        ("get_hitl_request", handle.get_hitl_request(id).map(drop)),
        (
            "get_exec_stdin_request",
            handle.get_exec_stdin_request(id).map(drop),
        ),
        (
            "get_session_status",
            handle.get_session_status(id).map(drop),
        ),
        ("get_model_info", handle.get_model_info(id).map(drop)),
        (
            "get_continue_state",
            handle.get_continue_state(id).map(drop),
        ),
        ("get_goal_state", handle.get_goal_state(id).map(drop)),
        ("get_role", handle.get_role(id).map(drop)),
        (
            "get_session_tokens",
            handle.get_session_tokens(id).map(drop),
        ),
        ("get_session_model", handle.get_session_model(id).map(drop)),
        (
            "get_compaction_progress",
            handle.get_compaction_progress(id).map(drop),
        ),
        (
            "get_buffered_output",
            handle.get_buffered_output(id, 10).map(drop),
        ),
        (
            "get_work_unit_context",
            handle.get_work_unit_context(id).map(drop),
        ),
        ("get_pending_input", handle.get_pending_input(id).map(drop)),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn every_session_getter_reports_an_unknown_session() -> TestResult {
    // @step Given a session manager with no session "00000000-0000-4000-8000-000000000445"
    let fixture = StorageFixture::new().await?;
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    let id = SessionId {
        value: UNKNOWN.to_string(),
    };
    // @step When each session getter reads that session
    let outcomes = read_every_getter(handle, &id);
    // @step Then every getter fails naming "00000000-0000-4000-8000-000000000445"
    for (getter, outcome) in outcomes {
        let error = outcome
            .err()
            .ok_or(format!("{getter} answered for an unknown session"))?;
        assert!(error.contains(UNKNOWN), "{getter}: {error}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn a_live_session_with_nothing_pending_reads_as_nothing_pending() -> TestResult {
    // @step Given a session manager with a live session that has no role, pause, question or pending input
    let fixture = StorageFixture::new().await?;
    fixture.select_model();
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    let id = handle.create_session_checked(None)?;
    // @step When the role, pause state, question and pending input are read
    let role = handle.get_role(&id);
    let pause = handle.get_pause_state(&id);
    let question = handle.get_hitl_request(&id);
    let pending = handle.get_pending_input(&id);
    // @step Then each read is Ok(None)
    assert_eq!(role, Ok(None));
    assert!(matches!(pause, Ok(None)), "{pause:?}");
    assert!(matches!(question, Ok(None)), "{question:?}");
    assert_eq!(pending, Ok(None));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn session_getters_over_rpc_are_refused_without_a_session_manager() -> TestResult {
    // @step Given an RPC service with no session manager attached
    let directory = tempfile::tempdir()?;
    std::fs::create_dir_all(directory.path().join("spec"))?;
    std::fs::write(
        directory.path().join("spec/work-units.json"),
        "{\"workUnits\":{}}",
    )?;
    let watcher = Arc::new(WorkUnitsWatcher::new(directory.path())?);
    let service = Arc::new(SharedFspecService::new(watcher));
    let backend = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service);
    // @step When the session role is read over the embedded transport
    let read = backend
        .get_session_role(SessionId {
            value: UNKNOWN.to_string(),
        })
        .await;
    // @step Then the read fails saying no session manager is attached
    let error = read
        .err()
        .ok_or("a role was read without a session manager")?
        .to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn schedule_and_blocklist_listings_without_a_start_workspace_are_errors() -> TestResult {
    // @step Given a session manager with no start workspace
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step When the scheduled jobs are listed
    let listed = handle.schedule_list();
    // @step Then the listing fails instead of returning an empty list
    let blocklist = handle.blocklist_list();
    let error = listed
        .err()
        .ok_or("jobs were listed without a start workspace")?;
    assert!(!error.is_empty());
    let error = blocklist
        .err()
        .ok_or("blocklist rules were listed without a start workspace")?;
    assert!(!error.is_empty());
    Ok(())
}
