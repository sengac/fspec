//! BUG-464 — remaining session methods report failures.
//!
//! Feature: spec/features/remaining-session-methods-report-failures.feature

mod common; // BUG-487: TaskOutcome lives in tests/common
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::SessionId;
use serial_test::serial;

#[path = "common/session_storage_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::StorageFixture;

const UNKNOWN: &str = "00000000-0000-4000-8000-000000000464";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn thinking_level_supervisors_pending_input_and_debug_writes_report_an_unknown_session(
) -> TestResult {
    // @step Given a session manager with no session "00000000-0000-4000-8000-000000000464"
    let fixture = StorageFixture::new().await?;
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    let id = SessionId::new(UNKNOWN);
    // @step When the thinking level and supervisors are read and the pending input and debug flag are written for that session
    let outcomes = vec![
        (
            "get_thinking_level",
            handle.get_thinking_level(&id).map(drop),
        ),
        ("get_supervisors", handle.get_supervisors(&id).map(drop)),
        (
            "set_pending_input",
            handle.set_pending_input(&id, Some("draft".to_string())),
        ),
        ("set_debug_enabled", handle.set_debug_enabled(&id, true)),
    ];
    // @step Then every call fails naming "00000000-0000-4000-8000-000000000464"
    for (method, outcome) in outcomes {
        let error = outcome
            .err()
            .ok_or(format!("{method} answered for an unknown session"))?;
        assert!(error.contains(UNKNOWN), "{method}: {error}");
    }
    Ok(())
}

#[test]
fn handle_defaults_refuse_instead_of_answering() -> TestResult {
    // @step Given a session manager handle that does not implement set_role or get_thinking_level
    let stub = StubSessionManagerHandle::new();
    let handle: &dyn SessionManagerHandle = &stub;
    let id = SessionId::new("stub-1");
    // @step When the role is set and the thinking level is read through it
    let role = handle.set_role(&id, Some("reviewer".to_string()));
    let level = handle.get_thinking_level(&id);
    // @step Then both calls fail saying the method is not implemented by this session manager
    let error = role.err().ok_or("set_role answered")?;
    assert!(
        error.contains("set_role is not implemented by this session manager"),
        "{error}"
    );
    let error = level.err().ok_or("get_thinking_level answered")?;
    assert!(
        error.contains("get_thinking_level is not implemented by this session manager"),
        "{error}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_writes_over_rpc_are_refused_without_a_session_manager() -> TestResult {
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
    // @step When the pending input is written and the thinking level is read over the embedded transport
    let written = backend
        .set_pending_input(SessionId::new(UNKNOWN), Some("draft".to_string()))
        .await;
    let read = backend.get_thinking_level(SessionId::new(UNKNOWN)).await;
    // @step Then both calls fail saying no session manager is attached
    for outcome in [written.err(), read.err()] {
        let error = outcome
            .ok_or("a call answered without a session manager")?
            .to_string();
        assert!(error.contains("no session manager is attached"), "{error}");
    }
    Ok(())
}
