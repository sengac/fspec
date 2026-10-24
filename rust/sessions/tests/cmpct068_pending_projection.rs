//! Feature: spec/features/pending-compaction-restore-projection.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_pending_publication_restore_cannot_claim_completed_reconciliation(
) -> support::TestResult {
    let Some(session_id) = support::cold_process_session()? else {
        // @step Given a durable generation still awaiting completion publication
        let (directory, manager, session_id, _) = support::manager().await?;
        let mut manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
        append_message(&mut manifest, "user", "covered original")?;
        let background = manager.get_session(&session_id.value)?;
        let inner = background.inner.lock().await;
        let model = inner.current_model_id().ok_or("selected model")?;
        let mut candidate = boundary::candidate(&manifest)?;
        candidate.source = inner.capture_durable_compaction(manifest.id)?;
        candidate.checkpoint.model = model.clone();
        candidate.checkpoint.payload =
            serde_json::to_value(inner.provider_session.prepare_generation(1, &model)?)?;
        drop(inner);
        let promoted = commit_compaction(candidate, &model, |_, _| Ok(()))?;
        assert!(publish_compaction(&promoted, |_| Err("receiver lost".into())).is_err());
        manifest = load_session(manifest.id)?;
        set_session_tokens(
            &mut manifest,
            &TokenUsage {
                current_context_tokens: 333,
                cumulative_billed_input: 5030,
                cumulative_billed_output: 2900,
                cache_read_tokens: 43,
                cache_creation_tokens: 17,
                reasoning_tokens: 1100,
            },
        )?;
        drop(manager);
        return support::run_cold_process(
            directory.path(),
            &session_id,
            "scenario_pending_publication_restore_cannot_claim_completed_reconciliation",
        )
        .await;
    };
    let manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    let expected = manifest.compaction_commit.clone();
    // @step When a fresh session restores its saved context and spend
    let manager = support::cold_resume(&session_id)?;
    let background = manager.get_session(&session_id.value)?;
    // @step Then no reconciled committed snapshot or completion notice is emitted
    assert!(
        background.committed_context_snapshot().is_none(),
        "pending publication is not completed reconciliation"
    );
    assert!(background
        .get_buffered_output(100)
        .iter()
        .all(|chunk| !matches!(
            chunk,
            StreamChunk::CommittedCompaction { .. } | StreamChunk::CompactionComplete { .. }
        )));
    // @step And the pending generation remains authoritative without losing saved counters
    assert_eq!(background.get_tokens(), (333, 2900, Some(1100)));
    let mut inner = background.inner.lock().await;
    assert!(inner.ensure_compaction_inference_ready().is_err());
    assert_eq!(load_session(manifest.id)?.compaction_commit, expected);
    let identity = expected.ok_or("pending boundary")?.identity;
    inner.reconcile_durable_compaction(&identity)?;
    assert_eq!(inner.token_tracker.input_tokens, 333);
    assert_eq!(inner.token_tracker.cumulative_billed_input, 5030);
    assert_eq!(inner.token_tracker.cumulative_billed_output, 2900);
    assert_eq!(inner.token_tracker.reasoning_tokens, 1100);
    Ok(())
}
