//! Feature: spec/features/a-panic-in-one-session-never-stops-other-sessions-persistence.feature
//!
//! BUG-610: a panic inside one session's manifest update poisoned the
//! daemon-wide session store lock, so every other session's turn failed
//! with "session store unavailable: poisoned lock" until the daemon
//! restarted. The panic is injected deterministically inside session A's
//! update closure on a named worker thread; no timing is involved.

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn configure(path: &std::path::Path) -> Result<(), String> {
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

/// Run one manifest update of `session_id` whose closure panics, on a
/// worker thread, and return the panic message.
fn panic_inside_update(session_id: Uuid) -> Result<String, Box<dyn std::error::Error>> {
    let worker = std::thread::Builder::new()
        .name("bug610-worker".into())
        .spawn(move || {
            update_session_manifest(session_id, |current| {
                current.name = "never committed".into();
                std::panic::resume_unwind(Box::new("injected BUG-610 panic"))
            })
        })?;
    match worker.join() {
        Ok(result) => Err(format!("the update returned instead of panicking: {result:?}").into()),
        Err(payload) => Ok(payload
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .ok_or("unexpected panic payload")?),
    }
}

fn poisoned(session_id: Uuid) -> String {
    format!(
        "the manifest writer lock of session {session_id} is poisoned: an earlier update of \
         this session panicked; close and reopen the session to recover"
    )
}

#[test]
#[serial_test::serial]
fn another_session_still_commits_after_one_sessions_update_panics() -> TestResult {
    // @step Given two open sessions A and B in one daemon
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let a = create_session("session A", directory.path())?;
    let b = create_session("session B", directory.path())?;

    // @step When session A's manifest update panics inside its update closure
    assert_eq!(panic_inside_update(a.id)?, "injected BUG-610 panic");

    // @step Then session B's next manifest update commits its next revision
    let committed = update_session_manifest(b.id, |current| {
        current.name = "B still writes".into();
        Ok(())
    })?;
    assert_eq!(committed.revision, b.revision + 1);
    assert_eq!(load_session(b.id)?.name, "B still writes");

    // @step And session A's committed manifest keeps its revision from before the panic
    let durable = load_session(a.id)?;
    assert_eq!(durable.revision, a.revision);
    assert_eq!(durable.name, "session A");
    Ok(())
}

#[test]
#[serial_test::serial]
fn the_panicked_session_reports_its_poisoned_writer_until_it_is_reopened() -> TestResult {
    // @step Given session A's manifest update panicked inside its update closure
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let a = create_session("session A", directory.path())?;
    panic_inside_update(a.id)?;

    // @step When session A writes its manifest again
    let again = update_session_manifest(a.id, |current| {
        current.name = "after the panic".into();
        Ok(())
    });

    // @step Then the write fails with the poisoned writer error naming session A
    let error = again.err().ok_or("a poisoned writer must not commit")?;
    assert_eq!(error.to_string(), poisoned(a.id));
    let mut stale = load_session(a.id)?;
    let appended = append_message(&mut stale, "user", "after the panic");
    assert_eq!(appended.err().as_deref(), Some(poisoned(a.id).as_str()));

    // @step When session A is closed, releasing its writer ownership
    release_session_ownership(a.id);

    // @step Then session A's next manifest update commits from the manifest on disk
    let reopened = update_session_manifest(a.id, |current| {
        current.name = "reopened".into();
        Ok(())
    })?;
    assert_eq!(reopened.revision, a.revision + 1);
    assert_eq!(load_session(a.id)?.name, "reopened");
    Ok(())
}
