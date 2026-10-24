#![doc = "Feature: spec/features/causal-transcript-archive.feature"]

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore, TerminalOutcome,
};
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn original(manifest: &mut SessionManifest, text: &str) -> Result<Uuid, String> {
    append_message_with_metadata(
        manifest,
        "assistant",
        text,
        std::collections::HashMap::from([(
            "message".into(),
            json!({"role":"assistant","content":[{"type":"text","text":text}]}),
        )]),
    )
}

fn identity(session_id: Uuid, message_id: Uuid) -> JournalIdentity {
    JournalIdentity {
        session_id,
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id,
    }
}

fn export(session_id: Uuid) -> Result<Vec<(ArchiveFragment, Value)>, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    let mut cursor = None;
    let mut encoded = String::new();
    loop {
        let page = get_session_archival_page(session_id, cursor, 4096)?;
        assert!(serde_json::to_vec(&page)?.len() <= 4096);
        if let Some(fragment) = page.fragment {
            assert_eq!(fragment.byte_offset, encoded.len());
            encoded.push_str(&fragment.json_fragment);
            if encoded.len() == fragment.total_bytes {
                records.push((fragment, serde_json::from_str(&encoded)?));
                encoded.clear();
            }
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            return Ok(records);
        }
    }
}

#[test]
#[serial_test::serial]
fn canonical_archival_export_preserves_causal_journal_identities() -> TestResult {
    // @step Given two original requests with journal outcomes and backward timestamps
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let mut manifest = create_session("causal archive", directory.path())?;
    append_message(&mut manifest, "user", "first request")?;
    let first = original(&mut manifest, "first original")?;
    let store = JournalStore::configured(manifest.id)?;
    let first_identity = identity(manifest.id, first);
    store.append(&JournalRecord::new(
        first_identity.clone(),
        0,
        JournalEvent::Text {
            text: "first original".into(),
        },
    ))?;
    let mut failure = JournalRecord::new(
        first_identity,
        1,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
    );
    failure.timestamp = chrono::DateTime::from_timestamp(20, 0).ok_or("timestamp")?;
    store.append(&failure)?;
    let abandoned = identity(manifest.id, Uuid::new_v4());
    store.append(&JournalRecord::new(
        abandoned.clone(),
        0,
        JournalEvent::Text {
            text: "discarded response attempt".into(),
        },
    ))?;
    store.append(&JournalRecord::new(
        abandoned,
        1,
        JournalEvent::AttemptAborted,
    ))?;
    append_message(&mut manifest, "user", "second request")?;
    let second = original(&mut manifest, "second original")?;
    let mut cancellation = JournalRecord::new(
        identity(manifest.id, second),
        0,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::UserCancelled,
        },
    );
    cancellation.timestamp = chrono::DateTime::from_timestamp(10, 0).ok_or("timestamp")?;
    store.append(&cancellation)?;
    let partial = JournalRecord::new(
        identity(manifest.id, Uuid::new_v4()),
        0,
        JournalEvent::Text {
            text: "unpromoted authentic prefix".into(),
        },
    );
    store.append(&partial)?;
    // @step When the canonical archival export is reassembled
    let records = export(manifest.id)?;
    // @step Then each outcome follows its original content before the next request
    let ids: Vec<_> = records
        .iter()
        .map(|(fragment, _)| fragment.message_id)
        .collect();
    assert_eq!(
        ids,
        vec![
            manifest.messages[0].message_id,
            first,
            failure.id,
            manifest.messages[2].message_id,
            second,
            cancellation.id,
            partial.id
        ]
    );
    // @step And journal identities timestamps and inert partial content survive without model contamination
    for record in [&failure, &cancellation, &partial] {
        let (fragment, value) = records
            .iter()
            .find(|(fragment, _)| fragment.message_id == record.id)
            .ok_or("missing journal export")?;
        assert_eq!(fragment.timestamp, record.timestamp);
        assert_eq!(value, &json!({"_transcript": record}));
        assert_ne!(fragment.record_kind, ArchiveRecordKind::Conversation);
    }
    let model = get_session_message_envelopes(manifest.id)?.join("");
    assert!(!model.contains("_transcript"));
    assert!(!model.contains("unpromoted authentic prefix"));
    // @step And aborted response attempts remain absent from archival dialogue
    assert!(!serde_json::to_string(&records)?.contains("discarded response attempt"));
    // @step And inconsistent causal anchors fail explicitly instead of guessing event order
    store.append(&JournalRecord::new(
        identity(manifest.id, second),
        0,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
    ))?;
    assert!(get_session_archival_page(manifest.id, None, 4096).is_err());
    Ok(())
}

#[test]
#[serial_test::serial]
fn canonical_archival_snapshots_pin_the_journal_prefix() -> TestResult {
    // @step Given an unpromoted journal prefix spanning bounded UTF-8 archive pages
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let mut manifest = create_session("pinned journal", directory.path())?;
    let store = JournalStore::configured(manifest.id)?;
    let text = "雪🙂 quote\" slash\\\n".repeat(1000);
    let partial = JournalRecord::new(
        identity(manifest.id, Uuid::new_v4()),
        0,
        JournalEvent::Text { text: text.clone() },
    );
    store.append(&partial)?;
    let first = get_session_archival_page(manifest.id, None, 4096)?;
    let fragment = first.fragment.ok_or("missing journal-only archive")?;
    assert_eq!(fragment.message_id, partial.id);
    let mut encoded = fragment.json_fragment;
    let mut cursor = first.next_cursor.ok_or("journal must span pages")?;
    let pinned = cursor.clone();
    // @step When a later journal outcome and original promotion are committed during traversal
    let terminal = JournalRecord::new(
        partial.identity.clone(),
        1,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
    );
    store.append(&terminal)?;
    let mut messages = MessageStore::new()?;
    let metadata = std::collections::HashMap::from([(
        "message".into(),
        json!({"role":"assistant","content":[{"type":"text","text":text}]}),
    )]);
    messages.store_with_id(partial.identity.message_id, "assistant", &text, metadata)?;
    manifest.add_message(partial.identity.message_id, MessageSource::Native);
    save_session(&manifest)?;
    // @step Then the pinned export retains the original prefix once without later events
    loop {
        let page = get_session_archival_page(manifest.id, Some(cursor), 4096)?;
        assert!(serde_json::to_vec(&page)?.len() <= 4096);
        let fragment = page.fragment.ok_or("missing continuation")?;
        assert_eq!(fragment.message_id, partial.id);
        assert_eq!(fragment.byte_offset, encoded.len());
        encoded.push_str(&fragment.json_fragment);
        match page.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }
    assert_eq!(
        serde_json::from_str::<Value>(&encoded)?,
        json!({"_transcript":partial})
    );
    // @step And a fresh export includes the outcome and suppresses promoted duplicate content
    let records = export(manifest.id)?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].0.message_id, partial.identity.message_id);
    assert_eq!(records[1].0.message_id, terminal.id);
    // @step And changing an acknowledged journal prefix invalidates the pinned export
    for entry in std::fs::read_dir(store.directory())? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let mut record: JournalRecord = serde_json::from_slice(&std::fs::read(&path)?)?;
            if record.id == partial.id {
                record.timestamp = chrono::DateTime::from_timestamp(10, 0).ok_or("timestamp")?;
                std::fs::write(path, serde_json::to_vec(&record)?)?;
            }
        }
    }
    assert!(matches!(
        get_session_archival_page(manifest.id, Some(pinned), 4096),
        Err(ArchiveError::SourceChanged)
    ));
    Ok(())
}
