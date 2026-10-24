//! Feature: spec/features/dag-cold-boundary-validation.feature

use codelet_core::persistence::compaction_commit::{commit_compaction, reconcile_compaction};
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use uuid::Uuid;

#[path = "support/compaction_boundary.rs"]
mod support;

#[test]
#[serial_test::serial]
fn scenario_reject_malformed_committed_memory_during_cold_replay(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a committed generation with corrupted DAG syntax provenance coverage identity or admission
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("cold boundary", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable original")?;
    let committed = commit_compaction(support::candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("missing commit")?
        .identity
        .clone();
    let mut ready = reconcile_compaction(&identity)?;
    append_message(&mut ready, "user", "uncovered immutable suffix")?;
    let path = directory
        .path()
        .join("sessions")
        .join(format!("{}.json", ready.id));
    let original = std::fs::read(&path)?;
    let mut corruptions = Vec::new();
    for summary in [
        "<dag-node depth=\"D1\" turns=\"0-0\" label=\"open\">unclosed",
        "<dag-node depth=\"D1\" turns=\"0-9\" label=\"unknown\">payload</dag-node>",
    ] {
        let mut corrupted = ready.clone();
        corrupted.compaction.as_mut().ok_or("missing DAG")?.summary = summary.into();
        corruptions.push(corrupted);
    }
    let mut missing_coverage = ready.clone();
    let all_messages = missing_coverage
        .messages
        .iter()
        .map(|message| message.message_id)
        .collect::<Vec<_>>();
    let boundary = missing_coverage
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?;
    boundary.covered_prefix = all_messages.clone();
    boundary.source.prefix = all_messages;
    boundary.verified_suffix.clear();
    missing_coverage
        .compaction
        .as_mut()
        .ok_or("missing DAG")?
        .compacted_before_index = 2;
    corruptions.push(missing_coverage);
    let mut wrong_generation = ready.clone();
    wrong_generation
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?
        .source
        .generation = identity.generation;
    corruptions.push(wrong_generation);
    let mut missing_operation = ready.clone();
    missing_operation
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?
        .identity
        .operation_id = Uuid::nil();
    corruptions.push(missing_operation);
    let mut missing_protocol = ready.clone();
    missing_protocol
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?
        .source
        .protocol
        .clear();
    corruptions.push(missing_protocol);
    let mut missing_model = ready.clone();
    let boundary = missing_model
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?;
    boundary.source.model.clear();
    boundary.checkpoint.model.clear();
    corruptions.push(missing_model);
    let mut missing_revision = ready.clone();
    missing_revision
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?
        .identity
        .revision = 0;
    corruptions.push(missing_revision);
    let mut missing_reservation = ready;
    missing_reservation
        .compaction_commit
        .as_mut()
        .ok_or("missing commit")?
        .output_reservation = 0;
    corruptions.push(missing_reservation);
    for corrupted in corruptions {
        let bytes = serde_json::to_vec_pretty(&corrupted)?;
        std::fs::write(&path, &bytes)?;
        reset_stores_for_tests();
        // @step When a cold reader loads the committed replay envelopes
        let replay = get_session_message_envelopes(corrupted.id);
        // @step Then replay fails explicitly without presenting invalid memory or changing the durable revision
        assert!(
            replay.is_err(),
            "invalid committed boundary must block replay: {replay:?}"
        );
        assert_eq!(std::fs::read(&path)?, bytes);
    }
    std::fs::write(path, original)?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_invalid_promoted_memory_cannot_be_reconciled() -> Result<(), Box<dyn std::error::Error>>
{
    // @step Given a promoted generation whose committed DAG has become invalid
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("invalid publication", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    let mut committed =
        commit_compaction(support::candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("missing commit")?
        .identity
        .clone();
    committed.compaction.as_mut().ok_or("missing DAG")?.summary = "malformed memory".into();
    let path = directory
        .path()
        .join(format!("sessions/{}.json", manifest.id));
    let corrupted = serde_json::to_vec_pretty(&committed)?;
    std::fs::write(&path, &corrupted)?;
    reset_stores_for_tests();
    // @step When its exact identity requests reconciliation
    let result = reconcile_compaction(&identity);
    // @step Then reconciliation fails without clearing the pending publication or changing durable bytes
    assert!(
        result.is_err(),
        "invalid memory must not unblock inference: {result:?}"
    );
    assert_eq!(std::fs::read(&path)?, corrupted);
    assert!(
        load_session(manifest.id)?
            .compaction_commit
            .ok_or("missing commit")?
            .publication_pending
    );
    Ok(())
}
