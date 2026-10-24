//! Feature: spec/features/versioned-dag-generation-commit.feature

use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "support/compaction_boundary.rs"]
mod support;
use support::candidate;

#[test]
#[serial_test::serial]
fn scenario_appended_work_survives_a_captured_prefix_commit() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    // @step Given a candidate captured before additional tool work is persisted
    let mut captured = create_session_with_provider("captured source", directory.path(), "openai")?;
    append_message(&mut captured, "user", "covered original")?;
    let candidate = candidate(&captured)?;
    let mut advanced = captured.clone();
    append_message(&mut advanced, "assistant", "uncovered tool work")?;
    // @step When the candidate commits against the unchanged source generation
    let committed = commit_compaction(candidate, "model-a", |current, _| {
        assert_eq!(current.messages.len(), 2);
        Ok(())
    })?;
    // @step Then one new generation covers only the captured prefix
    let boundary = committed
        .compaction_commit
        .as_ref()
        .ok_or("missing generation")?;
    assert_eq!(boundary.identity.generation, 1);
    assert_eq!(boundary.covered_prefix.len(), 1);
    assert_eq!(boundary.verified_suffix.len(), 1);
    // @step And the verified uncovered suffix and immutable originals survive
    reset_stores_for_tests();
    let restored = load_session(captured.id)?;
    assert_eq!(restored.messages.len(), 2);
    assert_eq!(
        serde_json::to_value(restored)?["compaction_commit"]["identity"]["generation"],
        1
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_obsolete_publication_cannot_announce_a_replaced_generation() -> TestResult {
    // @step Given an acknowledged generation and a newer promoted generation
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("obsolete publication", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    let first = commit_compaction(candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let ready = publish_compaction(&first, |_| Ok(()))?;
    let path = directory
        .path()
        .join(format!("sessions/{}.json", manifest.id));
    let mut snapshots = vec![first, ready.clone()];
    for advance in [false, true] {
        if advance {
            commit_compaction(candidate(&ready)?, "model-a", |_, _| Ok(()))?;
            snapshots.push(load_session(manifest.id)?);
        }
        let before = std::fs::read(&path)?;
        for obsolete in &snapshots[..2] {
            let mut notified = false;
            // @step When an obsolete or already acknowledged snapshot requests publication
            let result = publish_compaction(obsolete, |_| {
                notified = true;
                Ok(())
            });
            // @step Then publication is rejected before notifying subscribers
            assert!(matches!(result, Err(CompactionCommitError::Rejected(_))));
            assert!(!notified, "obsolete snapshots must not notify subscribers");
            // @step And the authoritative manifest bytes remain unchanged
            assert_eq!(std::fs::read(&path)?, before);
        }
    }
    let latest = snapshots.last().ok_or("missing latest snapshot")?;
    let reconciled = publish_compaction(latest, |boundary| {
        assert_eq!(boundary.identity.generation, 2);
        Ok(())
    })?;
    assert!(
        !reconciled
            .compaction_commit
            .ok_or("missing boundary")?
            .publication_pending
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_an_older_candidate_cannot_replace_a_newer_generation() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("two producers", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "source")?;
    // @step Given two candidates captured from the same durable generation
    let first = candidate(&manifest)?;
    let second = candidate(&manifest)?;
    // @step When one commits before the other
    let committed = commit_compaction(first, "model-a", |_, _| Ok(()))?;
    let first_identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("boundary")?
        .identity
        .clone();
    reconcile_compaction(&first_identity)?;
    let result = commit_compaction(second, "model-a", |_, _| {
        Err("must not prepare stale request".into())
    });
    // @step Then the second is explicitly rejected as stale
    assert!(
        matches!(result, Err(CompactionCommitError::Rejected(reason)) if reason.contains("stale"))
    );
    // @step And the newer generation and operation identity remain unchanged
    reset_stores_for_tests();
    let restored = load_session(manifest.id)?;
    assert_eq!(
        restored.compaction_commit.ok_or("boundary")?.identity,
        first_identity
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_changed_prefix_or_model_identity_rejects_a_candidate() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    // @step Given a candidate with captured durable source and model identity
    let mut manifest = create_session_with_provider("changed source", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "source")?;
    let different_model = candidate(&manifest)?;
    let changed_prefix = candidate(&manifest)?;
    // @step When its prefix or selected model no longer matches
    let model_result = commit_compaction(different_model, "model-b", |_, _| Ok(()));
    update_session_manifest(manifest.id, |current| {
        current.messages[0].message_id = Uuid::new_v4();
        Ok(())
    })?;
    let prefix_result = commit_compaction(changed_prefix, "model-a", |_, _| Ok(()));
    // @step Then commit is explicitly rejected without promotion or completion
    for result in [model_result, prefix_result] {
        assert!(matches!(result, Err(CompactionCommitError::Rejected(_))));
    }
    reset_stores_for_tests();
    assert!(load_session(manifest.id)?.compaction.is_none());
    Ok(())
}
