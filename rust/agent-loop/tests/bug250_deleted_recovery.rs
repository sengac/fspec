#![doc = "Feature: spec/features/deleted-transcript-recovery.feature"]

use codelet_agent_loop::{persist::retry_pending_persistence, BackgroundOutput};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::{
    delete_session, messages::durability::observe, transcript::JournalStore,
};

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn recovery_cannot_recreate_a_deleted_session_transcript() -> TestResult {
    // @step Given a deleted session with a retained unacknowledged transcript record
    let (_directory, session, _) = fixture()?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    observe(Some("transcript.record.sync"), || {
        output.emit(StreamEvent::Text("deleted original".into()));
    });
    drop(output);
    delete_session(session.id)?;
    let journal = JournalStore::configured(session.id)?;
    journal.delete()?;
    // @step When explicit persistence recovery is attempted after transcript removal
    let recovery = retry_pending_persistence(&session.id);
    // @step Then recovery retires pending ownership without recreating the deleted transcript
    assert!(recovery.is_ok(), "{recovery:?}");
    assert!(!journal.directory().exists());
    assert!(support::read(session.id).is_err());
    Ok(())
}
