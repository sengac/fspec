//! Feature: spec/features/restored-compaction-context-projection.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::TokenRestoreState;
use std::sync::atomic::Ordering;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_restored_committed_context_refreshes_without_claiming_completion_delivery(
) -> support::TestResult {
    // @step Given a saved committed generation with estimated occupancy and model limits
    let (_directory, manager, session_id, _) = support::manager().await?;
    let mut manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    append_message(&mut manifest, "user", "retained original")?;
    let mut candidate = boundary::candidate(&manifest)?;
    candidate.input_tokens = 123;
    let promoted = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    let committed = publish_compaction(&promoted, |_| Ok(()))?;
    let state = TokenRestoreState {
        current_context: 123,
        cumulative_billed_output: 0,
        cache_read: 0,
        cache_creation: 0,
        cumulative_billed_input: 0,
        cumulative_billed_output_second: 0,
    };
    let background = manager.get_session(&session_id.value)?;
    let mut events = manager.chunks_rx();
    // @step When the session restores its authoritative token state
    manager.restore_session_token_state(&session_id, state.clone())?;
    // @step Then the background context snapshot immediately identifies that durable generation
    let snapshot = background
        .committed_context_snapshot()
        .ok_or("missing restored committed snapshot")?;
    assert_eq!(
        snapshot.generation as u64,
        committed
            .compaction_commit
            .as_ref()
            .ok_or("commit identity")?
            .identity
            .generation
    );
    assert_eq!(snapshot.tokens.input_tokens, 123);
    assert!(snapshot.estimated);
    assert_eq!(
        snapshot.context_fill.threshold,
        f64::from(
            background
                .cached_compaction_threshold
                .load(Ordering::Acquire)
        )
    );
    assert_eq!(
        snapshot.context_fill.context_window,
        f64::from(background.cached_context_window.load(Ordering::Acquire))
    );
    // @step And the refresh is reconciliation rather than a delivered completion notice
    assert!(snapshot.reconciled);
    assert_eq!(background.get_tokens().0, 123);
    let mut refreshes = 0;
    while let Ok((_, event)) = events.try_recv() {
        match event {
            codelet_rpc_types::StreamChunk::CommittedCompaction { snapshot } => {
                assert!(snapshot.reconciled);
                refreshes += 1;
            }
            codelet_rpc_types::StreamChunk::CompactionComplete { .. } => {
                return Err("fabricated completion delivery".into())
            }
            _ => {}
        }
    }
    assert_eq!(refreshes, 1);
    // @step And repeated restoration does not duplicate the refresh
    manager.restore_session_token_state(&session_id, state)?;
    while let Ok((_, event)) = events.try_recv() {
        assert!(!matches!(
            event,
            codelet_rpc_types::StreamChunk::CommittedCompaction { .. }
        ));
    }
    Ok(())
}
