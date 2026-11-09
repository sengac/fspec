//! BUG-516: manual /compact runs the compactor sub-agent round that
//! automatic compaction uses, instead of an in-view 'Continue' turn that
//! the BUG-214 inference gate refuses.
//!
//! Feature: spec/features/rust-tui-compact-real-compaction.feature
//!
//! The harness answers compactor-agent requests (they name the target
//! session) with a valid DAG, or with an error when `fail` is set, and
//! counts every other provider request as ordinary inference.

#[path = "support/automatic_compaction.rs"]
pub mod support;

use codelet_rpc_types::{SessionStatus, StreamChunk};
use std::sync::atomic::Ordering;
use support::Harness;

/// The conversation's messages, without the system reminders every agent
/// run refreshes (environment, work unit).
async fn conversation(harness: &Harness) -> String {
    let inner = harness.session.inner.lock().await;
    let messages: Vec<_> = inner
        .messages
        .iter()
        .filter(|message| {
            !serde_json::to_string(message)
                .expect("serialize message")
                .contains("<system-reminder>")
        })
        .collect();
    serde_json::to_string(&messages).expect("serialize conversation")
}

/// Scenario: Compacting a populated session runs the compactor agent and replaces the conversation with its summary
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn manual_compaction_runs_the_compactor_agent() {
    // @step Given a session with several user and assistant messages
    let mut harness = Harness::new(false, false, false).await;
    assert!(conversation(&harness).await.contains("committed response"));

    // @step When I compact the session through the handle
    let acknowledgement = harness
        .session
        .start_manual_compaction()
        .expect("manual compaction starts");
    assert_eq!(
        acknowledgement.compacted_tokens, 0,
        "RPC-421 acknowledgement"
    );
    let chunks = harness.until_done().await;

    // @step Then the compactor agent builds the summary once, as automatic compaction does
    assert_eq!(harness.builders.load(Ordering::SeqCst), 1, "{chunks:?}");
    assert!(
        chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })),
        "the committed summary is reported: {chunks:?}"
    );

    // @step And the conversation is replaced by the committed summary
    assert!(
        conversation(&harness)
            .await
            .contains("Continue the remaining original task"),
        "the conversation carries the committed DAG"
    );

    // @step And no ordinary inference turn runs and no "Continue" message is sent
    assert_eq!(harness.infer.load(Ordering::SeqCst), 0, "{chunks:?}");
    assert!(
        !chunks.iter().any(|chunk| matches!(
            chunk,
            StreamChunk::UserInput { text } if text == "Continue"
        )),
        "{chunks:?}"
    );
    assert!(
        !chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::Error { .. })),
        "a manual compaction reports no error: {chunks:?}"
    );

    // @step And the session returns to Idle with ordinary inference allowed
    assert_eq!(harness.session.get_status(), SessionStatus::Idle);
    let inner = harness.session.inner.lock().await;
    assert!(!inner.has_compaction_working_view());
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    drop(inner);
    harness.close_injections();
}

/// Scenario: A failed manual compaction leaves the conversation unchanged
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_manual_compaction_leaves_the_conversation_unchanged() {
    // @step Given a session with several user and assistant messages
    // @step And the compactor agent fails with "summary service unavailable; retry later"
    let mut harness = Harness::new(false, true, false).await;
    let before = conversation(&harness).await;

    // @step When I compact the session through the handle
    harness
        .session
        .start_manual_compaction()
        .expect("manual compaction starts");
    let chunks = harness.until_done().await;

    // @step Then the session shows "Compaction failed:" with the compactor agent's reason
    assert!(harness.builders.load(Ordering::SeqCst) >= 1);
    assert!(
        chunks.iter().any(|chunk| matches!(
            chunk,
            StreamChunk::UserNotification { message, .. }
                if message.starts_with("Compaction failed:")
                    && message.contains("summary service unavailable")
        )),
        "{chunks:?}"
    );
    assert!(
        !chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })),
        "{chunks:?}"
    );

    // @step And no ordinary inference turn runs
    assert_eq!(harness.infer.load(Ordering::SeqCst), 0, "{chunks:?}");

    // @step And the conversation is unchanged
    assert_eq!(conversation(&harness).await, before);
    assert_eq!(harness.session.get_status(), SessionStatus::Idle);
    harness.close_injections();
}
