//! Feature: spec/features/shared-message-store-index-coherence.feature

use codelet_core::persistence::{
    cleanup_orphaned_messages, get_session_message_envelopes, load_session, reset_stores_for_tests,
    MessageSource, MessageStore, SessionStore,
};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::sync::mpsc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct ResetStores;

impl Drop for ResetStores {
    fn drop(&mut self) {
        reset_stores_for_tests();
    }
}

/// @step Given stored messages exist but the session directory is absent
/// @step When cleanup discovers orphaned messages
/// @step Then cleanup defers deletion without treating missing evidence as an empty reference set
/// @step And the original messages remain available after cold reopening
#[test]
#[serial]
fn scenario_cleanup_preserves_messages_when_committed_session_evidence_is_unavailable() -> TestResult
{
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let _reset = ResetStores;
    let mut messages = MessageStore::new()?;
    let message_id = messages.store("assistant", "original evidence")?;
    assert!(!directory.path().join("sessions").exists());

    assert_eq!(cleanup_orphaned_messages()?, 0);
    let message = MessageStore::new()?
        .get(message_id)
        .ok_or("original was deleted")?;
    assert_eq!(message.id, message_id);
    assert_eq!(message.content, "original evidence");
    Ok(())
}

/// @step Given the process session cache was loaded before a peer committed a new session
/// @step And message history contains an unreferenced message
/// @step When cleanup discovers orphaned messages
/// @step Then it removes only the unreferenced message
/// @step And the peer session's original messages remain available after cold reopening
#[test]
#[serial]
fn scenario_cleanup_discovers_committed_references_missing_from_the_session_cache() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let _reset = ResetStores;
    let baseline = {
        let mut sessions = SessionStore::new()?;
        sessions.create("cached baseline", directory.path())?
    };
    load_session(baseline.id)?;
    let mut messages = MessageStore::new()?;
    let orphan_id = messages.store("user", "unreferenced")?;
    let message_id = messages.store("assistant", "peer original evidence")?;
    let peer = {
        let mut sessions = SessionStore::new()?;
        let mut peer = sessions.create("uncached peer", directory.path())?;
        peer.add_message(message_id, MessageSource::Native);
        sessions.commit(&peer)?
    };

    assert_eq!(cleanup_orphaned_messages()?, 1);
    let reopened = MessageStore::new()?;
    assert!(reopened.get(orphan_id).is_none());
    let message = reopened
        .get(message_id)
        .ok_or("peer original was deleted")?;
    assert_eq!(message.id, message_id);
    assert_eq!(message.content, "peer original evidence");
    assert_eq!(get_session_message_envelopes(peer.id)?.len(), 1);
    Ok(())
}

/// @step Given a writer has appended a message but has not committed its session reference
/// @step When cleanup runs while the writer retains its session lease
/// @step Then cleanup defers deletion without waiting for the writer
/// @step And the writer can commit its reference and cold reopen the original message
#[test]
#[serial]
fn scenario_cleanup_preserves_messages_awaiting_a_writers_manifest_commit() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let _reset = ResetStores;
    let manifest = {
        let mut sessions = SessionStore::new()?;
        sessions.create("in-flight peer", directory.path())?
    };
    load_session(manifest.id)?;
    let (appended_tx, appended_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || -> Result<(), String> {
        let mut sessions = SessionStore::new()?;
        let mut messages = MessageStore::new()?;
        sessions
            .update(manifest.id, |current| {
                let message_id = messages
                    .store("assistant", "awaiting manifest commit")
                    .map_err(codelet_core::persistence::ManifestCommitError::Store)?;
                current.add_message(message_id, MessageSource::Native);
                appended_tx.send(message_id).map_err(|error| {
                    codelet_core::persistence::ManifestCommitError::Store(error.to_string())
                })?;
                continue_rx
                    .recv_timeout(Duration::from_secs(10))
                    .map_err(|error| {
                        codelet_core::persistence::ManifestCommitError::Store(error.to_string())
                    })?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(())
    });
    let message_id = appended_rx.recv_timeout(Duration::from_secs(10))?;

    let cleanup = cleanup_orphaned_messages();
    continue_tx.send(())?;
    writer.join().map_err(|_| "writer thread failed")??;
    assert_eq!(cleanup?, 0);
    let reopened = MessageStore::new()?;
    let message = reopened
        .get(message_id)
        .ok_or("in-flight original was deleted")?;
    assert_eq!(message.id, message_id);
    assert_eq!(message.content, "awaiting manifest commit");
    assert_eq!(get_session_message_envelopes(manifest.id)?.len(), 1);
    Ok(())
}
