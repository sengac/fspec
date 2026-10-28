//! Feature: spec/features/compaction-explicit-retry-safety.feature

#[path = "support/automatic_compaction.rs"]
pub mod automatic;

use codelet_rpc_types::StreamChunk;
use std::sync::atomic::Ordering;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancelling_an_explicit_builder_retry_retains_the_safety_block() {
    // @step Given fresh user input has restarted a failed builder
    let mut fixture = automatic::Harness::new(false, true, true).await;
    fixture.send();
    fixture.until_done().await;
    fixture.fail_builder.store(false, Ordering::SeqCst);
    fixture.send();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while fixture.builders.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("explicit retry entered the builder");
    // @step When the user cancels the pending summary request
    fixture.session.interrupt();
    let chunks = fixture.until_done().await;
    // @step Then no summary is committed and ordinary inference does not resume
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(!fixture
        .session
        .compaction_in_progress
        .load(Ordering::SeqCst));
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    assert!(fixture
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    fixture.assert_originals_available();
    // @step And another user message cannot restart the cancelled operation
    fixture.send();
    let chunks = fixture.until_done().await;
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_later_publication_failure_overrides_builder_retry_permission() {
    // @step Given automatic compaction failed before committing a summary
    let mut fixture = automatic::Harness::new(false, true, false).await;
    fixture.send();
    fixture.until_done().await;
    fixture.fail_builder.store(false, Ordering::SeqCst);
    // @step And an unresolved publication failure subsequently blocks inference
    fixture
        .session
        .inner
        .lock()
        .await
        .block_compaction_inference("publication failure remains unresolved".into());
    // @step When a fresh user message arrives
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then neither a builder nor ordinary inference starts
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(chunks.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("publication failure remains unresolved"))));
}
