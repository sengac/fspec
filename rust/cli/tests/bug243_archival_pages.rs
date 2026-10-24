//! Feature: spec/features/bounded-session-archives.feature

#[path = "bug243/fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

use codelet_core::persistence::{get_session_archival_page, ArchiveError, ArchiveRecordKind};
use fixtures::*;

#[test]
#[serial_test::serial]
fn page_full_archival_dialogue_independently_from_reduced_model_replay() -> TestResult {
    // @step Given a compacted session with large UTF-8 dialogue and legacy provider state
    let directory = setup()?;
    let (mut manifest, _) = legacy(directory.path())?;
    let text = "雪🙂 quote\" slash\\\n".repeat(6000);
    let original = json!({"uuid":Uuid::new_v4(),"provider":"codex","unknown":{"retained":true},
        "message":{"role":"user","content":[{"type":"text","text":text}]}});
    let large_id = append_message_with_metadata(
        &mut manifest,
        "user",
        "",
        serde_json::from_value(original.clone())?,
    )?;
    set_compaction_state(
        &mut manifest,
        "<dag-node depth=\"D1\" turns=\"0-4\" label=\"Prior\">archive summary</dag-node>".into(),
        5,
    )?;
    append_message_with_metadata(
        &mut manifest,
        "user",
        "suffix",
        serde_json::from_value(
            json!({"uuid":Uuid::new_v4(),"message":{"role":"user","content":[{"type":"text","text":"suffix"}]}}),
        )?,
    )?;
    let snapshot_end = manifest.messages.len();
    // @step When bounded archival fragments traverse a pinned original snapshot
    let mut cursor = None;
    let mut encoded = String::new();
    let mut records = Vec::new();
    let mut fragment_count = 0;
    let mut saved_cursor = None;
    loop {
        cursor = serde_json::from_str(&serde_json::to_string(&cursor)?)?;
        let page = get_session_archival_page(manifest.id, cursor, 4096)?;
        assert!(serde_json::to_vec(&page)?.len() <= 4096);
        assert_eq!(page.end_turn, snapshot_end);
        if saved_cursor.is_none() {
            saved_cursor = page.next_cursor.clone();
            append_message(&mut manifest, "user", "later append outside pinned archive")?;
        }
        if let Some(fragment) = page.fragment {
            assert_eq!(fragment.byte_offset, encoded.len());
            encoded.push_str(&fragment.json_fragment);
            fragment_count += 1;
            if encoded.len() == fragment.total_bytes {
                records.push((fragment, serde_json::from_str::<Value>(&encoded)?));
                encoded.clear();
            }
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    // @step Then complete original dialogue and persisted identities survive reassembly
    assert!(fragment_count > records.len());
    assert_eq!(records.len(), snapshot_end);
    let (record, envelope) = records
        .iter()
        .find(|(record, _)| record.message_id == large_id)
        .ok_or("large original")?;
    assert_eq!(record.turn_index, 4);
    assert_eq!(envelope["uuid"], original["uuid"]);
    assert_eq!(envelope["message"], original["message"]);
    assert_eq!(envelope["unknown"], original["unknown"]);
    assert_eq!(
        record.timestamp,
        get_message(large_id)?.ok_or("stored original")?.created_at
    );
    // @step And provider state is explicitly non-conversational and native payloads stay excluded
    let (state, marker) = &records[3];
    assert_eq!(state.record_kind, ArchiveRecordKind::ProviderState);
    assert_eq!(marker["recordKind"], "provider_state");
    assert!(marker.get("message").is_none());
    assert!(!serde_json::to_string(marker)?.contains("secret-native"));
    // @step And model replay still contains only the committed summary and uncovered suffix
    let replay = get_session_message_envelopes(manifest.id)?;
    assert_eq!(replay.len(), 3);
    assert!(replay[0].contains("archive summary"));
    assert!(!replay.join("").contains("secret-native"));
    assert!(!replay.join("").contains("雪🙂"));
    for budget in [1024, 1025, 2048, 61440] {
        let page = get_session_archival_page(manifest.id, None, budget)?;
        assert!(serde_json::to_vec(&page)?.len() <= budget);
    }
    assert!(get_session_archival_page(manifest.id, None, 1023).is_err());
    assert!(get_session_archival_page(manifest.id, None, 61441).is_err());
    let other = create_session("another archive", directory.path())?;
    assert!(get_session_archival_page(other.id, saved_cursor.clone(), 4096).is_err());
    let mut unsupported = saved_cursor.clone().ok_or("archive cursor")?;
    unsupported.version = 2;
    assert!(get_session_archival_page(manifest.id, Some(unsupported), 4096).is_err());
    let mut source_store = MessageStore::new()?;
    source_store.update_metadata(
        large_id,
        std::collections::HashMap::from([("unknown".into(), json!({"changed":true}))]),
    )?;
    assert!(matches!(
        get_session_archival_page(manifest.id, saved_cursor.clone(), 4096),
        Err(ArchiveError::SourceChanged)
    ));
    // @step And unavailable or changed original sources fail explicitly
    update_session_manifest(manifest.id, |current| {
        current.messages.swap(0, 1);
        Ok(())
    })?;
    assert!(matches!(
        get_session_archival_page(manifest.id, saved_cursor, 4096),
        Err(ArchiveError::SourceChanged)
    ));
    let mut missing = create_session("missing archive", directory.path())?;
    missing.add_message(Uuid::new_v4(), MessageSource::Native);
    save_session(&missing)?;
    assert!(matches!(
        get_session_archival_page(missing.id, None, 4096),
        Err(ArchiveError::MissingMessage { .. })
    ));
    Ok(())
}
