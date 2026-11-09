//! BUG-369 — session handles report malformed ids, a poisoned exec-stdin
//! state lock, and an unsupported write_exec_stdin.
//!
//! Feature: spec/features/session-exec-stdin-and-id-errors.feature

use std::sync::{Arc, RwLock};

use codelet_core::session_manager_handle::StubSessionManagerHandle;
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::lock_state::{read_state, write_state};
use codelet_sessions::session_manager::SessionManager;

#[test]
fn a_malformed_session_id_is_rejected() {
    // @step Given a session manager handle
    let manager = SessionManager::new();
    let supervisor = SessionId::new(uuid::Uuid::new_v4().to_string());
    // @step When a supervisor is added with a malformed subordinate id
    let result = SessionManagerHandle::add_supervisor(
        manager.as_ref(),
        &SessionId::new("not-a-uuid".to_string()),
        &supervisor,
    );
    // @step Then the call fails naming the invalid session id
    let error = result.err().unwrap_or_default();
    assert!(error.contains("Invalid session ID: not-a-uuid"), "{error}");
}

#[test]
fn a_poisoned_exec_stdin_state_lock_is_reported() -> Result<(), String> {
    // @step Given exec-stdin state whose lock was poisoned
    let lock = Arc::new(RwLock::new(Some(7_u32)));
    let held = Arc::clone(&lock);
    let poisoner = std::thread::spawn(move || {
        let _guard = held.write();
        std::panic::resume_unwind(Box::new("poison the exec-stdin state lock"));
    });
    if poisoner.join().is_ok() {
        return Err("the poisoning thread did not panic".to_string());
    }
    // @step When the state is read and written
    let read = read_state(&lock, "exec-stdin request");
    let written = write_state(&lock, "exec-stdin request", |slot| *slot = None);
    // @step Then both operations fail instead of reporting no request
    assert!(read.is_err(), "{read:?}");
    assert!(written.is_err(), "{written:?}");
    Ok(())
}

#[test]
fn the_default_write_exec_stdin_reports_that_it_is_unsupported() {
    // @step Given a session manager handle that does not implement exec stdin writes
    let handle = StubSessionManagerHandle::new();
    // @step When text is written to an exec session through it
    let result = handle.write_exec_stdin(
        &SessionId::new(uuid::Uuid::new_v4().to_string()),
        "exec-1",
        "yes",
    );
    // @step Then the write fails instead of claiming the text was written
    let error = result.err().unwrap_or_default();
    assert!(error.contains("not supported"), "{error}");
}
