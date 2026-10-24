//! Feature: spec/features/session-delete-cleanup-liveness.feature

use codelet_core::persistence::{
    append_message, cleanup_orphaned_messages, create_session, delete_session,
    is_session_store_initialized_for_tests, reset_stores_for_tests, resume_last_session,
    MessageSource, MessageStore, SessionStore,
};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct ResetStores;

impl Drop for ResetStores {
    fn drop(&mut self) {
        reset_stores_for_tests();
    }
}

/// @step Given an open session store owns a committed session referencing a stored message
/// @step When the store deletes that session twice without being dropped
/// @step Then orphan cleanup removes the message without closing the store
/// @step And the deleted session is absent from disk and the store cache
#[test]
#[serial]
fn scenario_cleanup_reclaims_deleted_session_messages_while_the_store_remains_alive() -> TestResult
{
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let _reset = ResetStores;
    let mut messages = MessageStore::new()?;
    let message_id = messages.store("user", "deleted session evidence")?;
    let mut sessions = SessionStore::new()?;
    let mut session = sessions.create("delete then clean", directory.path())?;
    session.add_message(message_id, MessageSource::Native);
    sessions.commit(&session)?;
    assert_eq!(cleanup_orphaned_messages()?, 0);

    sessions.delete(session.id)?;
    sessions.delete(session.id)?;

    assert_eq!(
        cleanup_orphaned_messages()?,
        1,
        "deletion must release retained leases while the store remains alive"
    );
    assert!(MessageStore::new()?.get(message_id).is_none());
    assert!(sessions.get(session.id).is_none());
    assert!(sessions.get_last_session(directory.path()).is_none());
    assert!(!directory
        .path()
        .join(format!("sessions/{}.json", session.id))
        .exists());
    assert!(directory
        .path()
        .join(format!("sessions/{}.lock", session.id))
        .exists());
    Ok(())
}

/// @step Given the session facade owns a committed session referencing a stored message
/// @step When the facade deletes that session twice without resetting its store
/// @step Then orphan cleanup removes the message without resetting the singleton
/// @step And no last session remains for the project
#[test]
#[serial]
fn scenario_cleanup_reclaims_deleted_session_messages_while_the_singleton_remains_alive(
) -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let _reset = ResetStores;
    let mut session = create_session("facade delete then clean", directory.path())?;
    let message_id = append_message(&mut session, "assistant", "singleton evidence")?;
    assert_eq!(cleanup_orphaned_messages()?, 0);

    delete_session(session.id)?;
    delete_session(session.id)?;

    assert!(is_session_store_initialized_for_tests());
    assert_eq!(
        cleanup_orphaned_messages()?,
        1,
        "deletion must release retained leases without resetting the singleton"
    );
    assert!(is_session_store_initialized_for_tests());
    assert!(MessageStore::new()?.get(message_id).is_none());
    assert!(resume_last_session(directory.path()).is_err());
    Ok(())
}
