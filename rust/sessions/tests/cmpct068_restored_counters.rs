//! Feature: spec/features/cold-compaction-counter-reconciliation.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{StreamChunk, TokenRestoreState};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cold_reconciliation_preserves_saved_spend_and_newer_context_occupancy(
) -> support::TestResult {
    let Some(session_id) = support::cold_process_session()? else {
        // @step Given a committed DAG followed by saved request occupancy billing cache and reasoning
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
        candidate.input_tokens = 123;
        drop(inner);
        let promoted = commit_compaction(candidate, &model, |_, _| Ok(()))?;
        manifest = publish_compaction(&promoted, |_| Ok(()))?;
        append_message(&mut manifest, "user", "newer retained suffix")?;
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
            "scenario_cold_reconciliation_preserves_saved_spend_and_newer_context_occupancy",
        )
        .await;
    };
    // @step When a fresh session restores its authoritative token state
    let manager = codelet_sessions::SessionManager::new();
    let mut events = manager.chunks_rx();
    manager.resume_session(&session_id)?;
    let background = manager.get_session(&session_id.value)?;
    manager.restore_session_token_state(
        &session_id,
        TokenRestoreState {
            current_context: 333,
            cumulative_billed_output: 2900,
            cache_read: 43,
            cache_creation: 17,
            cumulative_billed_input: 5030,
            cumulative_billed_output_second: 2900,
        },
    )?;
    // @step Then its live cached and emitted counters retain every saved value
    let inner = background.inner.lock().await;
    assert_eq!(inner.token_tracker.input_tokens, 333);
    assert_eq!(inner.token_tracker.cumulative_billed_input, 5030);
    assert_eq!(inner.token_tracker.cumulative_billed_output, 2900);
    assert_eq!(inner.token_tracker.reasoning_tokens, 1100);
    assert_eq!(background.get_tokens(), (333, 2900, Some(1100)));
    let snapshot = background
        .committed_context_snapshot()
        .ok_or("missing reconciliation")?;
    assert_eq!(snapshot.tokens.input_tokens, 333);
    assert_eq!(snapshot.tokens.output_tokens, 2900);
    assert_eq!(snapshot.tokens.cumulative_billed_input, Some(5030));
    assert_eq!(snapshot.tokens.cumulative_billed_output, Some(2900));
    assert_eq!(snapshot.tokens.cache_read_input_tokens, Some(43));
    assert_eq!(snapshot.tokens.cache_creation_input_tokens, Some(17));
    assert_eq!(snapshot.tokens.reasoning_tokens, Some(1100));
    let mut refreshes = 0;
    while let Ok((_, event)) = events.try_recv() {
        if let StreamChunk::CommittedCompaction { snapshot: emitted } = event {
            assert_eq!(
                serde_json::to_value(&emitted)?,
                serde_json::to_value(&snapshot)?
            );
            refreshes += 1;
        }
    }
    assert_eq!(refreshes, 1);
    // @step And its reconciled context fill uses saved occupancy rather than the original commit estimate
    assert_eq!(snapshot.context_fill.effective_tokens, 333.0);
    assert_eq!(
        snapshot.context_fill.fill_percentage,
        (333.0 / snapshot.context_fill.threshold * 100.0) as u32
    );
    assert!(snapshot.reconciled);
    // @step And its reduction excludes cumulative billing and reasoning spend
    assert_eq!(snapshot.original_tokens, 0);
    assert_eq!(snapshot.reduction_percentage(), 0.0);
    Ok(())
}
