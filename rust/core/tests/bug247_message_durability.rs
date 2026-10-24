//! Feature: spec/features/durable-message-before-manifest.feature

use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::{ManifestCommitError, MessageSource, MessageStore, SessionStore};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::HashSet;
use std::sync::{Arc, Barrier};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial]
fn scenario_message_barriers_precede_manifest_promotion() -> TestResult {
    // @step Given a temporary store and a committed empty session
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut sessions = SessionStore::new()?;
    let session = sessions.create("durable reference", directory.path())?;
    let mut messages = MessageStore::new()?;

    // @step When a message and its manifest reference are committed
    let (result, events) = observe(None, || {
        sessions.update(session.id, |current| {
            let message_id = messages
                .store("user", "durable 日本語")
                .map_err(ManifestCommitError::Store)?;
            current.add_message(message_id, MessageSource::Native);
            Ok(())
        })
    });
    let committed = result?;

    // @step Then message file and directory barriers precede manifest promotion
    for required in [
        "message.sync",
        "message.directory",
        "index.sync",
        "index.directory",
        "manifest.sync",
        "manifest.rename",
        "manifest.directory",
    ] {
        assert!(events.contains(&required), "missing {required}: {events:?}");
    }
    let position = |event| events.iter().position(|value| *value == event);
    assert!(position("message.sync") < position("message.directory"));
    assert!(position("message.directory") < position("manifest.sync"));
    assert!(position("manifest.sync") < position("manifest.rename"));
    assert!(position("manifest.rename") < position("manifest.directory"));

    // @step And a cold reader retrieves the referenced content
    let reopened = MessageStore::new()?;
    assert_eq!(
        reopened
            .get(committed.messages[0].message_id)
            .ok_or("missing message")?
            .content,
        "durable 日本語"
    );
    Ok(())
}

#[test]
#[serial]
fn scenario_blob_barriers_precede_message_acknowledgement() -> TestResult {
    // @step Given a temporary blob store and message store
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let blobs = codelet_core::persistence::BlobStore::new()?;
    let mut messages = MessageStore::new()?;

    // @step When blob content and a message referencing it are stored
    let (result, events) = observe(None, || -> Result<_, String> {
        let hash = blobs.store(b"large original content")?;
        let id = messages.store("assistant", &format!("blob:{hash}"))?;
        Ok((hash, id))
    });
    let (hash, id) = result?;

    // @step Then blob file and directory barriers precede message acknowledgement
    for event in ["blob.sync", "blob.directory", "message.sync"] {
        assert!(events.contains(&event), "missing {event}: {events:?}");
    }
    assert!(
        events.iter().position(|event| *event == "blob.directory")
            < events.iter().position(|event| *event == "message.sync")
    );
    assert_eq!(blobs.get(&hash)?, b"large original content");
    assert!(MessageStore::new()?.get(id).is_some());

    // @step And blob barrier failures prevent acknowledgement and remain retryable
    for boundary in ["blob.sync", "blob.directory"] {
        let content = boundary.as_bytes();
        let (result, _) = observe(Some(boundary), || blobs.store(content));
        assert!(result.is_err(), "{boundary} not reached");
        let hash = blobs.store(content)?;
        assert_eq!(blobs.get(&hash)?, content);
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_failed_acknowledgement_boundaries_preserve_recoverable_content() -> TestResult {
    // @step Given a temporary store and a committed empty session
    for boundary in [
        "message.sync",
        "message.directory",
        "index.write",
        "index.sync",
        "index.rename",
        "index.directory",
        "manifest.sync",
        "manifest.rename",
        "manifest.directory",
    ] {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        let mut sessions = SessionStore::new()?;
        let session = sessions.create("fault reference", directory.path())?;
        let mut messages = MessageStore::new()?;

        // @step When each message index and manifest boundary fails in turn
        let (result, _) = observe(Some(boundary), || {
            sessions.update(session.id, |current| {
                let message_id = messages
                    .store("user", boundary)
                    .map_err(ManifestCommitError::Store)?;
                current.add_message(message_id, MessageSource::Native);
                Ok(())
            })
        });

        // @step Then pre-promotion failures leave the old manifest and late failures report promotion
        assert!(result.is_err(), "fault {boundary} was not reached");
        let path = directory
            .path()
            .join(format!("sessions/{}.json", session.id));
        let committed: codelet_core::persistence::SessionManifest =
            serde_json::from_slice(&std::fs::read(path)?)?;
        if boundary == "manifest.directory" {
            assert!(matches!(result, Err(ManifestCommitError::Promoted { .. })));
            assert_eq!(committed.messages.len(), 1);
        } else {
            assert!(committed.messages.is_empty(), "{boundary}");
        }

        // @step And all published references remain readable after rebuilding the index
        let index = directory.path().join("messages/messages.idx");
        if index.exists() {
            std::fs::remove_file(index)?;
        }
        let reopened = MessageStore::new()?;
        let stored: codelet_core::persistence::StoredMessage = serde_json::from_slice(
            &std::fs::read(directory.path().join("messages/messages.jsonl"))?,
        )?;
        assert_eq!(
            reopened
                .get(stored.id)
                .ok_or("durable orphan missing")?
                .content,
            boundary
        );
        for reference in committed.messages {
            assert_eq!(
                reopened
                    .get(reference.message_id)
                    .ok_or("unavailable reference")?
                    .content,
                boundary
            );
        }
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_cleanup_replacement_preserves_referenced_content_across_failures() -> TestResult {
    // @step Given a temporary store with retained and orphaned records
    for boundary in [
        "cleanup.sync",
        "cleanup.invalidate",
        "cleanup.rename",
        "cleanup.directory",
        "index.sync",
        "index.rename",
        "index.directory",
    ] {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        let mut messages = MessageStore::new()?;
        messages.store("user", &"orphan".repeat(40))?;
        let retained = messages.store("assistant", "retained Ω")?;

        // @step When each cleanup durability boundary fails in turn
        let (result, events) = observe(Some(boundary), || {
            messages.cleanup_orphans(&HashSet::from([retained]))
        });

        // @step Then every retained record survives with correct offsets and promoted failures are explicit
        assert!(result.is_err(), "fault {boundary} was not reached");
        if events.contains(&"cleanup.rename") {
            assert!(result.err().ok_or("expected error")?.contains("promoted"));
        }
        let reopened = MessageStore::new()?;
        assert_eq!(
            reopened
                .get(retained)
                .ok_or("retained record missing")?
                .content,
            "retained Ω"
        );
        assert_eq!(
            reopened.index_len(),
            if events.contains(&"cleanup.rename") {
                1
            } else {
                2
            }
        );
    }
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut messages = MessageStore::new()?;
    messages.store("user", "orphan")?;
    let retained = messages.store("assistant", "retained")?;
    let (result, events) = observe(None, || {
        messages.cleanup_orphans(&HashSet::from([retained]))
    });
    assert_eq!(result?, 1);
    let ordered = [
        "cleanup.sync",
        "cleanup.invalidate",
        "cleanup.rename",
        "cleanup.directory",
        "index.sync",
        "index.rename",
        "index.directory",
    ];
    let completed: Vec<_> = events
        .into_iter()
        .filter(|event| ordered.contains(event))
        .collect();
    assert_eq!(completed, ordered);
    Ok(())
}

#[test]
#[serial]
fn scenario_concurrent_durable_appends_survive_a_missing_derived_index() -> TestResult {
    // @step Given independent writers sharing a temporary store
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let barrier = Arc::new(Barrier::new(2));
    let writers: Vec<_> = (0..2)
        .map(|writer| {
            let barrier = barrier.clone();
            std::thread::spawn(move || -> Result<_, String> {
                let mut messages = MessageStore::new()?;
                barrier.wait();
                (0..8)
                    .map(|number| {
                        let content = format!("writer {writer} record {number}");
                        messages.store("user", &content).map(|id| (id, content))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
        })
        .collect();

    // @step When the writers commit messages concurrently and the index is removed
    let mut expected = Vec::new();
    for writer in writers {
        expected.extend(writer.join().map_err(|_| "writer failed")??);
    }
    std::fs::remove_file(directory.path().join("messages/messages.idx"))?;

    // @step Then rebuilding retrieves every acknowledged message exactly once
    let reopened = MessageStore::new()?;
    assert_eq!(reopened.index_len(), expected.len());
    for (id, content) in expected {
        assert_eq!(reopened.get(id).ok_or("missing record")?.content, content);
    }
    Ok(())
}
