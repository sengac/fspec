#![doc = "Feature: spec/features/transcript-request-boundaries.feature"]

use codelet_core::persistence::transcript::{JournalEvent, JournalRecord, JournalStore};
use codelet_core::persistence::{
    append_message, create_session, get_session_archival_page, reset_stores_for_tests,
};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn boundary(
    session: Uuid,
    prefix: usize,
    preceding: Uuid,
) -> Result<JournalRecord, serde_json::Error> {
    serde_json::from_value(json!({
        "version": 1, "id": Uuid::new_v4(), "sequence": 0, "timestamp": chrono::Utc::now(),
        "identity": {"session_id": session, "request_id": Uuid::new_v4(), "attempt_id": Uuid::new_v4(), "message_id": Uuid::new_v4()},
        "request_anchor": {"original_count":prefix, "preceding_message_id":preceding},
        "event": {"kind":"terminal", "outcome":"provider_error"}
    }))
}

#[test]
#[serial_test::serial]
fn explicit_boundaries_order_journal_only_requests_before_later_originals() -> TestResult {
    // @step Given consecutive transcript-only requests with authentic original-prefix boundaries
    let root = setup_temp_directory()?;
    codelet_common::set_data_directory(root.path().into())?;
    reset_stores_for_tests();
    let mut manifest = create_session("request anchors", root.path())?;
    let store = JournalStore::configured(manifest.id)?;
    let mut expected = Vec::new();
    for (index, text) in ["first request", "second request"].iter().enumerate() {
        let original = append_message(&mut manifest, "user", text)?;
        expected.push(original);
        let mut failed = boundary(manifest.id, index + 1, original)?;
        failed.timestamp =
            chrono::DateTime::from_timestamp(20 - index as i64, 0).ok_or("timestamp")?;
        store.append(&failed)?;
        expected.push(failed.id);
    }
    // @step When later originals are archived with backward request timestamps
    expected.push(append_message(&mut manifest, "user", "third request")?);
    let mut cursor = None;
    let mut ids = Vec::new();
    loop {
        let page = get_session_archival_page(manifest.id, cursor, 4096)?;
        if let Some(fragment) = page.fragment {
            let value: Value = serde_json::from_str(&fragment.json_fragment)?;
            assert_ne!(value["_transcript"]["event"]["kind"], "request_started");
            ids.push(fragment.message_id);
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    // @step Then journal outcomes retain causal order and boundary records remain private
    assert_eq!(ids, expected);
    Ok(())
}

#[test]
#[serial_test::serial]
fn request_boundaries_cannot_change_after_the_first_delta() -> TestResult {
    // @step Given an acknowledged request with an authentic original-prefix boundary
    let root = setup_temp_directory()?;
    codelet_common::set_data_directory(root.path().into())?;
    reset_stores_for_tests();
    let mut manifest = create_session("immutable request anchor", root.path())?;
    let original = append_message(&mut manifest, "user", "request")?;
    let mut first = boundary(manifest.id, 1, original)?;
    first.event = JournalEvent::Text {
        text: "acknowledged prefix".into(),
    };
    let store = JournalStore::configured(manifest.id)?;
    store.append(&first)?;
    let evidence = store.read()?;
    // @step When a later delta attempts to replace that request boundary
    let mut replacement = boundary(manifest.id, 1, original)?;
    replacement.identity = first.identity;
    replacement.sequence = 1;
    let result = store.append(&replacement);
    // @step Then admission refuses the changed boundary without modifying acknowledged evidence
    assert!(result.is_err());
    assert_eq!(store.read()?, evidence);
    Ok(())
}

#[test]
#[serial_test::serial]
fn invalid_explicit_boundaries_never_become_inferred_coordinates() -> TestResult {
    // @step Given an explicit request boundary naming an incorrect original-prefix identity
    let root = setup_temp_directory()?;
    codelet_common::set_data_directory(root.path().into())?;
    reset_stores_for_tests();
    let mut manifest = create_session("invalid request anchor", root.path())?;
    append_message(&mut manifest, "user", "authentic request")?;
    let started = boundary(manifest.id, 1, Uuid::new_v4())?;
    let store = JournalStore::configured(manifest.id)?;
    store.append(&started)?;
    let original = std::fs::read(
        root.path()
            .join("sessions")
            .join(format!("{}.json", manifest.id)),
    )?;
    let evidence = store.read()?;
    // @step When the canonical archive is requested
    let result = get_session_archival_page(manifest.id, None, 4096);
    // @step Then export fails explicitly without altering originals or transcript evidence
    assert!(result.is_err());
    assert_eq!(store.read()?, evidence);
    assert_eq!(
        std::fs::read(
            root.path()
                .join("sessions")
                .join(format!("{}.json", manifest.id))
        )?,
        original
    );
    Ok(())
}
