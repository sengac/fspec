//! Feature: spec/features/background-compaction-publication-reconciliation.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cold_background_reconciliation_refreshes_context_without_duplicate_completion(
) -> support::TestResult {
    let Some(session_id) = support::cold_process_session()? else {
        // @step Given a cold background session with pending publication and saved occupancy and spend
        let (directory, manager, session_id, _) = support::manager().await?;
        let mut manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
        append_message(&mut manifest, "user", "covered immutable original")?;
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
        assert!(publish_compaction(&promoted, |_| Err("receiver disconnected".into())).is_err());
        manifest = load_session(manifest.id)?;
        append_message(&mut manifest, "user", "retained suffix")?;
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
        drop(background);
        drop(manager);
        return support::run_cold_process(
        directory.path(),
        &session_id,
        "scenario_cold_background_reconciliation_refreshes_context_without_duplicate_completion",
    )
    .await;
    };
    let manager = support::cold_resume(&session_id)?;
    let background = manager.get_session(&session_id.value)?;
    let mut events = background.subscribe_to_stream();
    let before = load_session(background.id)?;
    let boundary = before
        .compaction_commit
        .as_ref()
        .ok_or("pending boundary")?;
    let identity = boundary.identity.clone();
    let mut wrong = identity.clone();
    wrong.operation_id = uuid::Uuid::new_v4();
    // @step When an incorrect identity is reconciled
    assert!(background
        .reconcile_durable_compaction(&wrong)
        .await
        .is_err());
    // @step Then inference remains blocked and no committed context is published
    assert!(background
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    assert!(background.committed_context_snapshot().is_none());
    assert!(events.try_recv().is_err());
    // @step When the authoritative identity is reconciled twice
    background.reconcile_durable_compaction(&identity).await?;
    background.reconcile_durable_compaction(&identity).await?;
    // @step Then one reconciled snapshot preserves saved counters coverage and generation
    let snapshot = background
        .committed_context_snapshot()
        .ok_or("missing reconciled background snapshot")?;
    assert_eq!(snapshot.generation, 1);
    assert!(snapshot.reconciled);
    assert_eq!(snapshot.tokens.input_tokens, 333);
    assert_eq!(snapshot.tokens.output_tokens, 2900);
    assert_eq!(snapshot.tokens.cumulative_billed_input, Some(5030));
    assert_eq!(snapshot.tokens.cumulative_billed_output, Some(2900));
    assert_eq!(snapshot.tokens.cache_read_input_tokens, Some(43));
    assert_eq!(snapshot.tokens.cache_creation_input_tokens, Some(17));
    assert_eq!(snapshot.tokens.reasoning_tokens, Some(1100));
    assert_eq!(snapshot.context_fill.effective_tokens, 333.0);
    assert_eq!(snapshot.covered_turns, Some(1));
    assert_eq!(snapshot.retained_turns, Some(1));
    assert_eq!(background.get_tokens(), (333, 2900, Some(1100)));
    assert!(background
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_ok());
    // @step And no completion notice or second durable promotion is produced
    let mut refreshes = 0;
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, StreamChunk::CompactionComplete { .. }));
        if let StreamChunk::CommittedCompaction { snapshot: emitted } = event {
            assert_eq!(
                serde_json::to_value(emitted)?,
                serde_json::to_value(&snapshot)?
            );
            refreshes += 1;
        }
    }
    assert_eq!(refreshes, 1);
    let after = load_session(background.id)?;
    assert_eq!(
        after.compaction_commit.as_ref().ok_or("boundary")?.identity,
        identity
    );
    assert!(
        !after
            .compaction_commit
            .as_ref()
            .ok_or("boundary")?
            .publication_pending
    );
    assert_eq!(
        serde_json::to_value(after.messages)?,
        serde_json::to_value(before.messages)?
    );
    assert_eq!(after.token_usage, before.token_usage);
    Ok(())
}
