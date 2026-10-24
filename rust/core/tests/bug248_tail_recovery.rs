//! Feature: spec/features/message-jsonl-tail-recovery.feature

#[path = "bug248/support.rs"]
mod support;

use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::messages::FramingError;
use codelet_core::persistence::{MessageSource, MessageStore, SessionStore};
use proptest::prelude::*;
use serial_test::serial;
use std::sync::{Arc, Barrier};
use support::{append, fixture, quarantines, rebuild, record, TestResult};

#[test]
#[serial]
fn scenario_torn_tails_remain_recoverable_after_append_and_full_rebuild() -> TestResult {
    // @step Given valid history followed by an uncommitted partial JSON tail
    let (directory, _, prefix) = fixture()?;
    let torn = format!(
        "{{\"id\":\"{}\",\"content\":\"partial",
        uuid::Uuid::new_v4()
    );
    append(directory.path(), torn.as_bytes())?;

    // @step When the store reopens and appends a complete Unicode record
    let mut store = MessageStore::new()?;
    let new_id = store.store("assistant", "later 🦀 日本語")?;

    // @step Then a full index rebuild preserves exact valid IDs and content
    let reopened = rebuild(directory.path())?;
    assert_eq!(reopened.index_len(), 2);
    assert_eq!(
        reopened.get(prefix).ok_or("prefix missing")?.content,
        "valid prefix 日本語"
    );
    assert_eq!(
        reopened.get(new_id).ok_or("new record missing")?.content,
        "later 🦀 日本語"
    );

    // @step And the torn bytes remain quarantined across repeated recovery
    for _ in 0..3 {
        assert_eq!(rebuild(directory.path())?.index_len(), 2);
        assert_eq!(quarantines(directory.path())?, vec![torn.as_bytes()]);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4, failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    #[serial]
    fn scenario_every_unicode_byte_truncation_has_controlled_recovery(content in ".{0,8}") {
        // @step Given serialized messages containing arbitrary Unicode content
        let original = record(&format!("é🦀日本語 {content}"));
        let bytes = serde_json::to_vec(&original)?;

        // @step When each serialized byte boundary is truncated before a subsequent append
        for boundary in 0..=bytes.len() {
            let (directory, _, prefix) = fixture().map_err(|error| TestCaseError::fail(error.to_string()))?;
            append(directory.path(), &bytes[..boundary]).map_err(|error| TestCaseError::fail(error.to_string()))?;
            let mut store = MessageStore::new().map_err(TestCaseError::fail)?;
            let new_id = store.store("user", "after truncation 🦀").map_err(TestCaseError::fail)?;

            // @step Then recovery and full rebuild retain the valid prefix and exact appended content
            let reopened = rebuild(directory.path()).map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(reopened.get(prefix).ok_or_else(|| TestCaseError::fail("prefix missing"))?.content, "valid prefix 日本語");
            prop_assert_eq!(reopened.get(new_id).ok_or_else(|| TestCaseError::fail("new record missing"))?.content, "after truncation 🦀");
            prop_assert_eq!(reopened.index_len(), if boundary == bytes.len() { 3 } else { 2 });
            if boundary == bytes.len() {
                prop_assert_eq!(reopened.get(original.id).ok_or_else(|| TestCaseError::fail("original missing"))?.content, original.content.clone());
            }
        }
    }
}

#[test]
#[serial]
fn scenario_interrupted_quarantine_recovery_is_retryable() -> TestResult {
    // @step Given a valid prefix and an uncommitted torn tail
    for boundary in [
        "tail.quarantine.sync",
        "tail.quarantine.directory",
        "tail.truncate",
        "tail.sync",
        "tail.directory",
    ] {
        let (directory, mut store, prefix) = fixture()?;
        let torn = b"{\"id\":\"incomplete";
        append(directory.path(), torn)?;

        // @step When each quarantine and repair synchronization boundary fails
        let (result, events) = observe(Some(boundary), || store.store("user", "retry me"));
        assert!(result.is_err(), "boundary {boundary} not reached");
        if events.contains(&"tail.truncate") {
            assert!(events.contains(&"tail.quarantine.directory"));
        }

        // @step Then retry preserves the prefix and durably quarantines the torn bytes before removal
        let new_id = MessageStore::new()?.store("assistant", "retry succeeds")?;
        let reopened = rebuild(directory.path())?;
        assert!(reopened.get(prefix).is_some());
        assert_eq!(
            reopened.get(new_id).ok_or("retry missing")?.content,
            "retry succeeds"
        );
        assert_eq!(quarantines(directory.path())?, vec![torn.as_slice()]);
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_valid_unterminated_records_are_retained_and_delimited() -> TestResult {
    // @step Given valid history ending with complete JSON without a newline
    let (directory, _, prefix) = fixture()?;
    let original = record("complete Ω");
    append(directory.path(), &serde_json::to_vec(&original)?)?;

    // @step When another record is appended and the index is rebuilt
    let new_id = MessageStore::new()?.store("assistant", "next record")?;
    let reopened = rebuild(directory.path())?;

    // @step Then both original and new records remain separately readable
    assert_eq!(reopened.index_len(), 3);
    assert!(reopened.get(prefix).is_some());
    assert_eq!(
        reopened.get(original.id).ok_or("original missing")?.content,
        original.content
    );
    assert_eq!(
        reopened.get(new_id).ok_or("new record missing")?.content,
        "next record"
    );
    assert!(quarantines(directory.path())?.is_empty());
    Ok(())
}

#[test]
#[serial]
fn scenario_referenced_corruption_refuses_destructive_recovery() -> TestResult {
    // @step Given a manifest referencing a record truncated inside its UUID or Unicode content
    for corruption in ["uuid", "unicode", "hash"] {
        let (directory, mut store, prefix) = fixture()?;
        let prefix_bytes = std::fs::read(directory.path().join("messages/messages.jsonl"))?;
        let referenced = store.store("assistant", "referenced 🦀 original")?;
        let mut sessions = SessionStore::new()?;
        let mut session = sessions.create("referenced evidence", directory.path())?;
        session.add_message(prefix, MessageSource::Native);
        session.add_message(referenced, MessageSource::Native);
        sessions.commit(&session)?;
        let data = directory.path().join("messages/messages.jsonl");
        let bytes = std::fs::read(&data)?;
        let suffix = &bytes[prefix_bytes.len()..];
        if corruption == "hash" {
            let mut corrupted = bytes.clone();
            let offset = bytes
                .windows(b"referenced".len())
                .position(|part| part == b"referenced")
                .ok_or("content missing")?;
            corrupted[offset] = b'R';
            std::fs::write(&data, corrupted)?;
        } else {
            let cut = if corruption == "uuid" {
                18
            } else {
                suffix
                    .windows(4)
                    .position(|part| part == "🦀".as_bytes())
                    .ok_or("Unicode missing")?
                    + 2
            };
            std::fs::write(&data, &bytes[..prefix_bytes.len() + cut])?;
        }
        let evidence = std::fs::read(&data)?;

        // @step When the store reopens or attempts another append
        let reopened = MessageStore::new();
        let appended = store.store("user", "must not mask committed corruption");

        // @step Then corruption is reported without changing the message history
        assert!(reopened.is_err(), "committed torn tail was accepted");
        assert!(appended.is_err(), "append masked committed corruption");
        assert!(matches!(
            store.validate_framing(),
            Err(FramingError::Referenced { .. } | FramingError::Corrupt { .. })
        ));
        assert_eq!(std::fs::read(data)?, evidence);
        if corruption != "hash" {
            assert_eq!(
                quarantines(directory.path())?,
                vec![evidence[prefix_bytes.len()..].to_vec()]
            );
        }
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_malformed_interior_records_never_invent_message_identities() -> TestResult {
    // @step Given valid records separated by a malformed complete physical line
    let (directory, _, _) = fixture()?;
    let invented = uuid::Uuid::new_v4();
    append(
        directory.path(),
        format!("{{\"id\":\"{invented}\",bad json}}\n").as_bytes(),
    )?;
    let after = record("valid record after corruption");
    append(directory.path(), &serde_json::to_vec(&after)?)?;
    append(directory.path(), b"\n")?;
    let data = directory.path().join("messages/messages.jsonl");
    let evidence = std::fs::read(&data)?;

    // @step When the index is rebuilt
    let result = rebuild(directory.path());

    // @step Then corruption is reported without indexing UUID text inside malformed content
    assert!(
        result.is_err(),
        "malformed interior line was silently indexed"
    );
    assert_eq!(std::fs::read(data)?, evidence);
    Ok(())
}

#[test]
#[serial]
fn scenario_concurrent_recovery_and_append_preserve_exact_record_identities() -> TestResult {
    // @step Given independent writers and an uncommitted torn tail
    let (directory, _, prefix) = fixture()?;
    append(directory.path(), b"{\"id\":\"partial")?;
    let barrier = Arc::new(Barrier::new(2));

    // @step When the writers reopen and append concurrently
    let writers: Vec<_> = (0..2)
        .map(|writer| {
            let barrier = barrier.clone();
            std::thread::spawn(move || -> Result<_, String> {
                barrier.wait();
                MessageStore::new()?.store("assistant", &format!("writer {writer}"))
            })
        })
        .collect();
    let mut ids = vec![prefix];
    for writer in writers {
        ids.push(writer.join().map_err(|_| "writer failed")??);
    }

    // @step Then repeated full rebuilds retrieve each acknowledged record exactly once
    for _ in 0..3 {
        let reopened = rebuild(directory.path())?;
        assert_eq!(reopened.index_len(), ids.len());
        for id in &ids {
            assert_eq!(reopened.get(*id).ok_or("record missing")?.id, *id);
        }
    }
    Ok(())
}
