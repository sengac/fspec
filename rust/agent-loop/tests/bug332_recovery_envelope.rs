#![doc = "Feature: spec/features/conversation-recovery-requires-a-message-envelope.feature"]

use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_core::persistence::{create_session, load_session, reset_stores_for_tests};
use codelet_test_helpers::setup_temp_directory;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn a_recovery_record_without_a_message_envelope_is_not_committed() -> TestResult {
    // @step Given a session whose recovery journal holds a conversation record without a message envelope
    let directory = setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let session = create_session("bug332", directory.path())?;
    let spool = PersistenceSpool::open(session.id)?;
    spool.enqueue(&RecoveryRecord {
        version: 1,
        id: uuid::Uuid::new_v4(),
        role: "user".into(),
        content: "bare".into(),
        metadata: Default::default(),
    })?;

    // @step When the journal is committed
    let committed = spool.retry();

    // @step Then the commit fails because the record carries no message envelope
    let error = committed.err().ok_or("a bare record was committed")?;
    assert!(error.contains("no message envelope"), "{error}");

    // @step And the session gains no conversation row
    assert!(load_session(session.id)?.messages.is_empty());
    reset_stores_for_tests();
    Ok(())
}
