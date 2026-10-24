//! Feature: spec/features/legacy-provider-state-coordinates.feature

use super::fixtures::*;

#[test]
#[serial_test::serial]
fn traverse_legacy_checkpoint_coordinates_without_fabricated_dialogue() -> TestResult {
    // @step Given a legacy mixed manifest with a megabyte checkpoint lacking an envelope UUID
    let directory = setup()?;
    let (manifest, _) = legacy(directory.path())?;
    let source_manifest = std::fs::read(
        directory
            .path()
            .join(format!("sessions/{}.json", manifest.id)),
    )?;
    let source_messages = std::fs::read(directory.path().join("messages/messages.jsonl"))?;
    // @step When bounded original pages traverse its inclusive persisted range
    let mut records = Vec::new();
    let mut cursor = None;
    loop {
        let mut selected = request(manifest.id, cursor);
        selected.end_turn = Some(3);
        let page = originals::retrieve_originals(selected)?;
        assert!(page.serialized_len()? <= 61440);
        cursor = page.next_turn;
        records.extend(page.records);
        if cursor.is_none() {
            break;
        }
    }
    // @step Then the checkpoint position returns an explicit bounded provider state identity
    assert_eq!(records[3].envelope["recordKind"], "provider_state");
    assert_eq!(records[3].envelope["version"], 1);
    assert_eq!(records[3].envelope["provider"], "codex");
    assert!(records[3].envelope.get("message").is_none());
    assert!(records[3].envelope["stateId"]
        .as_str()
        .is_some_and(|identity| identity.len() == 64));
    // @step And dialogue identities and continuation coordinates remain unchanged
    assert_eq!(
        records
            .iter()
            .map(|record| record.turn_index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    for (record, reference) in records.iter().zip(&manifest.messages) {
        assert_eq!(record.message_id, reference.message_id);
    }
    assert_eq!(records[0].envelope["message"]["content"][0]["text"], "seed");
    let mut runner = proptest::test_runner::TestRunner::default();
    runner.run(&(0usize..4, 0usize..4), |(first, second)| {
        let start = first.min(second);
        let end = first.max(second);
        let mut selected = request(manifest.id, Some(start));
        selected.start_turn = Some(start);
        selected.end_turn = Some(end);
        selected.limit = Some(4);
        let page = originals::retrieve_originals(selected)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        proptest::prop_assert_eq!(page.records.len(), end - start + 1);
        for (offset, record) in page.records.iter().enumerate() {
            proptest::prop_assert_eq!(record.turn_index, start + offset);
            proptest::prop_assert_eq!(&record.envelope, &records[start + offset].envelope);
        }
        Ok(())
    })?;
    // @step And neither the manifest nor stored messages are modified
    assert_eq!(
        source_manifest,
        std::fs::read(
            directory
                .path()
                .join(format!("sessions/{}.json", manifest.id))
        )?
    );
    assert_eq!(
        source_messages,
        std::fs::read(directory.path().join("messages/messages.jsonl"))?
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn migrate_legacy_state_idempotently_across_interrupted_promotion() -> TestResult {
    // @step Given a legacy mixed session and an interrupted checkpoint staging file
    let directory = setup()?;
    let (manifest, checkpoint) = legacy(directory.path())?;
    let bytes = std::fs::read(directory.path().join("messages/messages.jsonl"))?;
    let coordinates = serde_json::to_value(&manifest.messages)?;
    let state_directory = directory
        .path()
        .join("provider-state")
        .join(manifest.id.to_string());
    std::fs::create_dir_all(&state_directory)?;
    std::fs::write(state_directory.join("interrupted.stage"), b"incomplete")?;
    // @step When migration is repeated before and after a cold reload
    let mut before = session();
    before.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    super::recovery::send_next(&before, &"secret-native".repeat(100_000)).await?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    let path = state_path(directory.path(), manifest.id)?;
    let first = std::fs::read(&path)?;
    live.bind_provider_persistence(manifest.id)?;
    reset_stores_for_tests();
    let mut cold = session();
    cold.bind_provider_persistence(manifest.id)?;
    // @step Then original bytes coordinates and native state identity remain unchanged
    assert_eq!(std::fs::read(&path)?, first);
    assert_eq!(
        std::fs::read(directory.path().join("messages/messages.jsonl"))?,
        bytes
    );
    assert_eq!(
        serde_json::to_value(load_session(manifest.id)?.messages)?,
        coordinates
    );
    // @step And only verified committed state is selected for continuation
    cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    assert_eq!(
        serde_json::to_value(history(&cold)?.snapshot()?)?,
        checkpoint
    );
    super::recovery::send_next(&cold, &"secret-native".repeat(100_000)).await?;
    Ok(())
}
